// Live browser UI: connects shell chrome to engine events (P2-01+).
import { renderShell, requestRender, revealHelm, toggleAsk } from './shell.js';
import { onMessage } from './bridge.js';
import { focusHelm } from './pagebasics.js';
import { helmSuggestions } from './helm.js';
import { initShortcuts, setShortcutTable } from './shortcuts.js';
import { generateSky } from './sky.js';
import { setWallpaper, wallpaperVersion } from './newtab.js';
import { chatUpdate, chatFocusInput, chatAdopt, chatCaptured, chatToast, chatRedraw, chatTranscribed } from './chat.js';

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

  initShortcuts();
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
      } else if (name === 'chat' || name === 'ai') {
        window.odysseyBridge?.(JSON.stringify({ type: 'chat-list' }));
        if (state.chatOpenId) {
          window.odysseyBridge?.(JSON.stringify({ type: 'chat-open', id: state.chatOpenId }));
          state.chatOpenId = null;
        }
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
    const wasActive = !!d && d.active;
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
      if (wasActive) {
        // The ring stays for a moment at 100% and pulses once.
        state.dlDoneAt = Date.now();
        setTimeout(() => { state.dlTick = Date.now(); renderShellLive(); }, 1300);
      }
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

  // What the assistant sends while it works: steps, answer text, sources, browser actions.
  const onAiEvent = (ev) => {
    const origin = state.aiOrigin === 'chat' ? 'chat' : 'helm';
    // A question sent straight over the bridge (no card of ours yet) still gets a card.
    if (origin === 'helm' && !state.answer && state.aiOrigin == null) {
      const provider = state.aiConfig?.provider;
      state.answer = {
        live: true, question: '', text: '', steps: [], sources: [], done: false,
        footer: provider === 'odysseus' ? 'Answered by your Odysseus server' : (provider === 'cloud' ? 'Answered by Cloud AI' : (provider === 'local' ? 'Answered by local AI' : 'AI not configured')),
      };
    }
    const target = origin === 'chat' ? state.chatStream : state.answer;
    if (!target) return;
    if (ev.type === 'delta') {
      target.text += ev.data;
    } else if (ev.type === 'step') {
      target.steps = [...(target.steps || []), ev.data];
    } else if (ev.type === 'source') {
      const title = ev.data?.title || ev.data?.url;
      if (title && !(target.sources || []).some((x) => x.url === ev.data?.url)) {
        target.sources = [...(target.sources || []), { title, url: ev.data?.url || '' }];
      }
    } else if (ev.type === 'action') {
      // The assistant opens a page in a new tab through the same path as you would.
      if (ev.data?.kind === 'open' && ev.data.url) window.odysseyBridge?.(JSON.stringify({ type: 'open', url: ev.data.url }));
    } else if (ev.type === 'error') {
      target.error = String(ev.data);
      if (!target.text) target.text = `Error: ${ev.data}`;
    } else if (ev.type === 'done') {
      target.done = true;
      // A question to you is void once the assistant has finished or been stopped.
      if (state.prompts?.some((p) => p.kind === 'agent-confirm')) state.prompts = state.prompts.filter((p) => p.kind !== 'agent-confirm');
      if (origin === 'chat' && state.chat) {
        const m = { role: 'assistant', text: target.text || '(No answer)', steps: target.steps, sources: target.sources, time: Math.floor(Date.now() / 1000) };
        state.chat.messages = [...state.chat.messages, m];
        state.chatStream = null;
      } else if (!target.text) {
        target.text = '(No answer)';
      }
    }
    if (origin === 'chat') chatUpdate(state); else renderShellLive();
  };

  onMessage((msg) => {
    if (!msg || typeof msg !== 'object') return;
    if (msg.type === 'view-opened') {
      if (!state.tabs) state.tabs = [];
      let tab = state.tabs.find((t) => t.id === msg.view);
      if (!tab) {
        tab = { id: msg.view, title: 'New tab', parent: msg.opener, url: msg.url || '', space: msg.space ?? 0 };
        state.tabs.push(tab);
      } else {
        if (msg.url) tab.url = msg.url;
        tab.space = msg.space ?? tab.space ?? 0;
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
    } else if (msg.type === 'agent-confirm') {
      state.prompts = (state.prompts || []).filter((p) => p.kind !== 'agent-confirm');
      state.prompts.push({ kind: 'agent-confirm', view: state.activeTab, request: msg.request, text: msg.text });
      renderShellLive();
    } else if (msg.type === 'toggle-ask') {
      revealHelm();
      toggleAsk(app, state);
    } else if (msg.type === 'suggestions') {
      helmSuggestions(msg.query, msg.items);
    } else if (msg.type === 'helm-focus') {
      revealHelm();
      focusHelm();
      setTimeout(focusHelm, 150);
    } else if (msg.type === 'find-in-page-prompt') {
      import('./pagebasics.js').then(({ toggleFindBar }) => {
        toggleFindBar(app, msg.view || state.activeTab);
      });
    } else if (msg.type === 'closed') {
      if (state.prompts) state.prompts = state.prompts.filter((p) => p.view !== msg.view);
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
        state.prompts = state.prompts.filter((p) => !(p.view === msg.view && (p.kind === 'permission' || p.kind === 'fill-choice')));
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
        document.querySelector('.helm')?.classList.toggle('is-loading', !!msg.loading && state.page === 'web');
        state.canGoBack = msg.can_go_back;
        state.canGoForward = msg.can_go_forward;
      }
    } else if (msg.type === 'wallpaper') {
      setWallpaper(msg.data);
      state.wallpaper = { has: !!msg.data, error: msg.error || '', rev: wallpaperVersion() };
      renderShellLive();
    } else if (msg.type === 'shortcuts') {
      setShortcutTable(msg.commands);
      state.shortcutList = msg.commands;
      renderShellLive();
    } else if (msg.type === 'shortcut-result') {
      state.shortcutResult = { ok: msg.ok, message: msg.message, at: Date.now() };
      if (state.settings) state.settings.shortcuts = msg.shortcuts;
      renderShellLive();
    } else if (msg.type === 'settings-loaded') {
      state.settings = msg.settings;
      renderShellLive();
    } else if (msg.type === 'space-changed') {
      state.activeSpace = msg.space;
      renderShellLive();
    } else if (msg.type === 'history-loaded') {
      state.history = msg.groups;
      renderShellLive();
    } else if (msg.type === 'save-login-prompt') {
      // Only the site and username arrive here; the password stays in Rust.
      state.prompts = (state.prompts || []).filter((p) => !(p.kind === 'save-login' && p.view === msg.view));
      state.prompts.push({ kind: 'save-login', view: msg.view, request: msg.request, host: msg.host, username: msg.username, update: msg.update });
      renderShellLive();
    } else if (msg.type === 'fill-choices') {
      state.prompts = (state.prompts || []).filter((p) => !(p.kind === 'fill-choice' && p.view === msg.view));
      state.prompts.push({ kind: 'fill-choice', view: msg.view, request: msg.request, host: msg.host, logins: msg.logins });
      renderShellLive();
    } else if (msg.type === 'vault-exported') {
      state.vaultExport = { ok: msg.ok, path: msg.path, message: msg.message };
      renderShell(app, state);
    } else if (msg.type === 'vault-imported') {
      state.vaultImport = { added: msg.added, skipped: msg.skipped, error: msg.error };
      renderShell(app, state);
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
      if (msg.event) onAiEvent(msg.event);
    } else if (msg.type === 'chat-list') {
      state.chatList = msg.chats || [];
      if (state.chat?.id) {
        const row = state.chatList.find((c) => c.id === state.chat.id);
        if (row) { state.chat.title = row.title; state.chat.pinned = row.pinned; }
      }
      chatUpdate(state);
    } else if (msg.type === 'chat-found') {
      if (msg.query === state.chatQuery) { state.chatFound = msg.chats || []; chatUpdate(state); }
    } else if (msg.type === 'chat-exported') {
      chatToast(msg.ok ? `Saved to ${msg.path}` : (msg.message || 'Could not save'));
    } else if (msg.type === 'ai-models') {
      state.aiModels = { models: msg.models || [], error: msg.error || '' };
      chatUpdate(state);
    } else if (msg.type === 'page-captured') {
      chatCaptured(msg);
    } else if (msg.type === 'update-status') {
      state.update = { state: msg.state, version: msg.version, notes: msg.notes, message: msg.message, progress: msg.progress };
      renderShellLive();
    } else if (msg.type === 'transcribed') {
      chatTranscribed(msg, state);
    } else if (msg.type === 'chat-data') {
      chatRedraw();
      state.chat = msg.chat ? { id: msg.chat.id, title: msg.chat.title, pinned: !!msg.chat.pinned, messages: msg.chat.messages || [] } : null;
      chatUpdate(state);
      chatFocusInput();
    } else if (msg.type === 'chat-started') {
      if (state.aiOrigin === 'chat' && state.chat) {
        state.chat.id = msg.id;
        chatAdopt(msg.id);
      } else {
        state.helmChat = msg.id;
      }
    } else if (msg.type === 'chat-saved') {
      window.odysseyBridge?.(JSON.stringify({ type: 'chat-list' }));
      if (state.chatRefresh && state.chat?.id) {
        state.chatRefresh = false;
        window.odysseyBridge?.(JSON.stringify({ type: 'chat-open', id: state.chat.id }));
      }
    } else if (msg.type === 'vault-loaded') {
      state.vaultEntries = msg.entries;
      state.vaultNever = msg.never || [];
      state.vaultNotice = msg.notice || '';
      renderShellLive();
    } else if (msg.type === 'vault-credentials') {
      state.vaultAuth = null;
      state.revealedPassword = { id: msg.id, username: msg.username, password: msg.password };
      renderShellLive();
      // Hide it again after 30 seconds.
      const shownId = msg.id;
      setTimeout(() => {
        if (state.revealedPassword && state.revealedPassword.id === shownId) {
          state.revealedPassword = null;
          renderShellLive();
        }
      }, 30000);
    } else if (msg.type === 'vault-reveal-denied') {
      state.revealedPassword = null;
      state.vaultAuth = { message: msg.message };
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
