// Live browser UI: connects shell chrome to engine events (P2-01+).
import { renderShell, requestRender, getSky } from './shell.js';
import { onMessage } from './bridge.js';
import { initPageBasicsShortcuts, focusHelm } from './pagebasics.js';
import { generateSky } from './sky.js';

export async function startLive() {
  const app = document.getElementById('app');
  const winW = window.innerWidth || 1280;
  const winH = window.innerHeight || 800;
  const isPrivate = new URLSearchParams(location.search).get('private') === '1';
  const sky = generateSky(isPrivate ? 'private' : 'newtab', winW, winH);

  const state = {
    private: isPrivate,
    rail: isPrivate ? 'private' : 'collapsed',
    tabs: [],
    activeTab: null,
    page: 'none',
    sky,
    live: true,
    shields: 0,
    downloads: [],
    activeSpace: 0,
    helm: { kind: 'empty' },
  };

  initPageBasicsShortcuts(() => app, () => state.activeTab);
  // Read-only handle for automated live checks (tools/live-check); the UI page is ours alone.
  window.__uiState = state;

  // Pages that show Rust-side data ask for it when they open (AUDIT-09).
  let dataPage = '';
  const renderShellLive = () => {
    const name = state.page === 'internal' ? (state.pageName || '') : '';
    if (name !== dataPage) {
      dataPage = name;
      if (name === 'history') {
        state.historyQuery = '';
        window.odysseyBridge?.(JSON.stringify({ type: 'query-history' }));
      } else if (name === 'extensions') {
        window.odysseyBridge?.(JSON.stringify({ type: 'get-extensions' }));
      }
    }
    requestRender(app, state);
  };

  const fmtBytes = (n) => {
    if (n >= 1024 ** 3) return `${(n / 1024 ** 3).toFixed(1)} GB`;
    if (n >= 1024 ** 2) return `${(n / 1024 ** 2).toFixed(n >= 100 * 1024 ** 2 ? 0 : 1)} MB`;
    if (n >= 1024) return `${Math.round(n / 1024)} KB`;
    return `${n} B`;
  };

  const applyDownload = (msg) => {
    let d = state.downloads.find((x) => x.id === String(msg.id));
    if (!d) {
      d = { id: String(msg.id), name: msg.file_name || 'Download', path: msg.path, received: 0, total: null };
      state.downloads.unshift(d);
    }
    d.name = msg.file_name || d.name;
    if (msg.path) d.path = msg.path;
    if (msg.received > 0) d.received = msg.received;
    if (msg.total != null) d.total = msg.total;
    d.speed = msg.speed || 0;
    d.cancellable = true;
    if (msg.complete) {
      d.active = false; d.done = true; d.failed = false; d.detail = 'Done';
    } else if (msg.cancelled) {
      d.active = false; d.done = false; d.failed = true; d.detail = 'Cancelled';
    } else {
      d.active = true; d.done = false;
      d.progress = d.total ? Math.min(1, d.received / d.total) : 0;
      const connections = state.settings?.connections || 8;
      const seg = d.total && d.total >= 8 * 1024 * 1024 && connections > 1;
      const parts = [d.total ? `${fmtBytes(d.received)} of ${fmtBytes(d.total)}` : fmtBytes(d.received)];
      if (d.speed > 0) parts.push(`${fmtBytes(d.speed)}/s`);
      if (seg) parts.push(`${connections} connections`);
      d.detail = parts.join(', ');
    }
    if (state.downloads.length > 20) state.downloads.length = 20;
    if (state.downloads.some((x) => x.active)) state.downloadsOpen = state.downloadsOpen ?? true;
  };

  onMessage((msg) => {
    if (!msg || typeof msg !== 'object') return;
    if (msg.type === 'view-opened') {
      if (!state.tabs) state.tabs = [];
      let tab = state.tabs.find((t) => t.id === msg.view);
      if (!tab) {
        tab = { id: msg.view, title: 'New tab', parent: msg.opener, url: msg.url || '' };
        state.tabs.push(tab);
      } else {
        if (msg.url) tab.url = msg.url;
        // An early url/title event may have created this tab out of order; the replay from
        // Rust defines the order, so move it to the end.
        state.tabs = state.tabs.filter((t) => t !== tab);
        state.tabs.push(tab);
        if (msg.opener != null) tab.parent = msg.opener;
      }
      state.activeTab = msg.view;
      state.shields = 0;
      const url = tab.url || msg.url || '';
      if (url && url !== 'about:blank' && !url.startsWith('odyssey://')) {
        state.page = 'web';
        try {
          const u = new URL(url);
          state.helm = { kind: 'address', address: { kind: 'web', host: u.host, path: u.pathname } };
        } catch (_) {
          state.helm = { kind: 'address', address: { kind: 'web', host: url, path: '' } };
        }
      } else if (url && url.startsWith('odyssey://')) {
        state.page = 'internal';
        const name = url.replace(/^odyssey:\/\//, '').replace(/\/$/, '');
        state.pageName = name;
        state.helm = { kind: 'address', address: { kind: 'internal', name } };
      } else {
        state.page = 'none';
        state.helm = { kind: 'empty' };
      }
      renderShellLive();
    } else if (msg.type === 'view-shown') {
      state.activeTab = msg.view;
      const tab = state.tabs?.find((t) => t.id === msg.view);
      const url = tab?.url || '';
      if (url && url !== 'about:blank' && !url.startsWith('odyssey://')) {
        state.page = 'web';
        try {
          const u = new URL(url);
          state.helm = { kind: 'address', address: { kind: 'web', host: u.host, path: u.pathname } };
        } catch (_) {
          state.helm = { kind: 'address', address: { kind: 'web', host: url, path: '' } };
        }
      } else if (url && url.startsWith('odyssey://')) {
        state.page = 'internal';
        const name = url.replace(/^odyssey:\/\//, '').replace(/\/$/, '');
        state.pageName = name;
        state.helm = { kind: 'address', address: { kind: 'internal', name } };
      } else {
        state.page = 'none';
        state.helm = { kind: 'empty' };
      }
      renderShellLive();
    } else if (msg.type === 'helm-focus') {
      focusHelm();
    } else if (msg.type === 'find-in-page-prompt') {
      import('./pagebasics.js').then(({ toggleFindBar }) => {
        toggleFindBar(app, msg.view || state.activeTab);
      });
    } else if (msg.type === 'closed') {
      if (state.tabs) {
        state.tabs = state.tabs.filter((t) => t.id !== msg.view);
        if (state.activeTab === msg.view) {
          state.activeTab = state.tabs[state.tabs.length - 1]?.id ?? null;
          if (state.activeTab) {
            const current = state.tabs.find((t) => t.id === state.activeTab);
            if (current?.url && current.url !== 'about:blank' && !current.url.startsWith('odyssey://')) {
              state.page = 'web';
              try {
                const u = new URL(current.url);
                state.helm = { kind: 'address', address: { kind: 'web', host: u.host, path: u.pathname } };
              } catch (_) {
                state.helm = { kind: 'address', address: { kind: 'web', host: current.url, path: '' } };
              }
            } else if (current?.url?.startsWith('odyssey://')) {
              state.page = 'internal';
              try {
                const u = new URL(current.url);
                state.pageName = u.hostname;
                state.helm = { kind: 'address', address: { kind: 'internal', name: u.hostname } };
              } catch (_) {
                state.helm = { kind: 'empty' };
              }
            } else {
              state.page = 'none';
              state.helm = { kind: 'empty' };
            }
          } else {
            state.page = 'none';
            state.helm = { kind: 'empty' };
          }
        }
      }
      renderShellLive();
    } else if (msg.type === 'url') {
      if (!state.tabs) state.tabs = [];
      let tab = state.tabs.find((t) => t.id === msg.view);
      if (!tab) {
        tab = { id: msg.view, title: 'New tab' };
        state.tabs.push(tab);
      }
      if (tab.url !== msg.url && state.activeTab === msg.view) state.shields = 0;
      // A permission question belongs to the page that asked; a new page makes it stale.
      if (tab.url !== msg.url && state.prompts) {
        state.prompts = state.prompts.filter((p) => !(p.view === msg.view && p.kind === 'permission'));
      }
      tab.url = msg.url;
      if (state.activeTab === msg.view || state.activeTab == null) {
        state.activeTab = msg.view;
        if (!msg.url || msg.url === 'about:blank') {
          state.page = 'none';
          state.helm = { kind: 'empty' };
        } else {
          try {
            const u = new URL(msg.url);
            if (u.protocol === 'odyssey:') {
              state.page = 'internal';
              state.pageName = u.hostname;
              state.helm = { kind: 'address', address: { kind: 'internal', name: u.hostname } };
            } else {
              state.page = 'web';
              state.helm = { kind: 'address', address: { kind: 'web', host: u.host, path: u.pathname } };
            }
          } catch (_) {
            if (msg.url.startsWith('odyssey://')) {
              state.page = 'internal';
              const name = msg.url.slice('odyssey://'.length).replace(/\/$/, '');
              state.pageName = name;
              state.helm = { kind: 'address', address: { kind: 'internal', name } };
            } else {
              state.page = 'web';
              state.helm = { kind: 'address', address: { kind: 'web', host: msg.url, path: '' } };
            }
          }
        }
      }
      renderShellLive();
    } else if (msg.type === 'title') {
      if (state.tabs) {
        const tab = state.tabs.find((t) => t.id === msg.view);
        if (tab) tab.title = msg.title;
      }
      renderShellLive();
    } else if (msg.type === 'loading') {
      if (state.activeTab === msg.view && state.helm && state.helm.kind === 'address') {
        state.isLoading = msg.loading;
        state.canGoBack = msg.can_go_back;
        state.canGoForward = msg.can_go_forward;
      }
    } else if (msg.type === 'settings-loaded') {
      state.settings = msg.settings;
      renderShellLive();
    } else if (msg.type === 'space-changed') {
      state.activeSpace = msg.space;
      renderShellLive();
    } else if (msg.type === 'history-loaded') {
      state.history = msg.groups;
      renderShellLive();
    } else if (msg.type === 'permission-request') {
      state.prompts = (state.prompts || []).filter((p) => p.request !== msg.request);
      state.prompts.push({ kind: 'permission', view: msg.view, request: msg.request, origin: msg.origin, what: msg.kind });
      renderShellLive();
    } else if (msg.type === 'certificate-error') {
      state.prompts = (state.prompts || []).filter((p) => p.request !== msg.request);
      state.prompts.push({ kind: 'certificate', view: msg.view, request: msg.request, url: msg.url, error: msg.error });
      renderShellLive();
    } else if (msg.type === 'download-progress') {
      applyDownload(msg);
      renderShellLive();
    } else if (msg.type === 'closed-tabs-updated') {
      state.closed = msg.tabs;
      renderShellLive();
    } else if (msg.type === 'shields-status') {
      if (state.activeTab === msg.view || state.activeTab == null) {
        state.shields = msg.count;
        state.shieldsEnabled = msg.enabled;
        renderShellLive();
      }
    } else if (msg.type === 'tab-sleeping') {
      if (state.tabs) {
        const tab = state.tabs.find((t) => t.id === msg.view);
        if (tab) tab.sleeping = msg.sleeping;
      }
      renderShellLive();
    } else if (msg.type === 'extensions-loaded') {
      state.extensions = msg.extensions;
      renderShellLive();
    } else if (msg.type === 'ai-config-loaded') {
      state.aiConfig = msg.config;
      renderShellLive();
    } else if (msg.type === 'ai-test-result') {
      state.aiTestResult = msg;
      renderShellLive();
    } else if (msg.type === 'ai-event') {
      if (msg.event) {
        if (!state.answer) {
          state.answer = {
            text: '',
            sources: [],
            footer: state.aiConfig?.provider === 'odysseus'
              ? 'Answered by your Odysseus server'
              : (state.aiConfig?.provider === 'cloud'
                ? 'Answered by Cloud AI'
                : (state.aiConfig?.provider === 'local' ? 'Answered by local AI' : 'AI')),
            action: state.aiConfig?.provider === 'odysseus' ? 'Continue in Odysseus' : '',
          };
        }
        if (state.answer.text === 'Thinking...') {
          state.answer.text = '';
        }
        if (msg.event.type === 'delta') {
          state.answer.text += msg.event.data;
          renderShellLive();
        } else if (msg.event.type === 'source') {
          const s = msg.event.data?.title || msg.event.data?.url;
          if (s && !state.answer.sources.includes(s)) {
            state.answer.sources.push(s);
            renderShellLive();
          }
        } else if (msg.event.type === 'error') {
          state.answer.text = `Error: ${msg.event.data}`;
          renderShellLive();
        }
      }
    } else if (msg.type === 'vault-loaded') {
      state.vaultEntries = msg.entries;
      state.vaultNotice = msg.notice || '';
      renderShellLive();
    } else if (msg.type === 'vault-credentials') {
      state.revealedPassword = { id: msg.id, username: msg.username, password: msg.password };
      renderShellLive();
    }
  });

  renderShell(app, state);

  // The new-tab clock is part of a cached region; tick it once a minute.
  state.minute = Math.floor(Date.now() / 60000);
  setInterval(() => {
    const m = Math.floor(Date.now() / 60000);
    if (m !== state.minute) {
      state.minute = m;
      renderShellLive();
    }
  }, 5000);

  // Click on the empty new-tab background: a ring and a few particles (AUDIT-32).
  app.addEventListener('pointerdown', (e) => {
    if (state.page !== 'none') return;
    const t = e.target;
    const empty = t === app || (t.classList && (t.classList.contains('sky') || t.classList.contains('newtab-hint')));
    if (empty) getSky()?.burst(e.clientX, e.clientY);
  }, { passive: true });

  // A click while another app is in front brings us forward (page windows are separate windows).
  window.addEventListener('pointerdown', () => {
    if (!document.hasFocus()) window.odysseyBridge?.(JSON.stringify({ type: 'focus-window' }));
  }, true);

  window.addEventListener('resize', () => {
    state.sky = generateSky(isPrivate ? 'private' : 'newtab', window.innerWidth || 1280, window.innerHeight || 800);
    renderShellLive();
  });

  if (typeof window.odysseyBridge === 'function') {
    window.odysseyBridge(JSON.stringify({ type: 'load-settings' }));
    window.odysseyBridge(JSON.stringify({ type: 'get-extensions' }));
    window.odysseyBridge(JSON.stringify({ type: 'ai-get-config' }));
    window.odysseyBridge(JSON.stringify({ type: 'vault-list' }));
  }
}
