// Live browser UI: connects shell chrome to engine events (P2-01+).
import { SHIELDS_COUNT } from './fixtures.js';
import { renderShell } from './shell.js';
import { onMessage } from './bridge.js';
import { initPageBasicsShortcuts } from './pagebasics.js';
import { generateSky } from './sky.js';

export async function startLive() {
  const app = document.getElementById('app');
  const winW = window.innerWidth || 1280;
  const winH = window.innerHeight || 800;
  const sky = generateSky('newtab', winW, winH);

  const state = {
    rail: 'collapsed',
    tabs: [],
    activeTab: null,
    page: 'none',
    sky,
    shields: SHIELDS_COUNT,
    activeSpace: 0,
    helm: { kind: 'empty' },
  };

  initPageBasicsShortcuts(() => app, () => state.activeTab);

  onMessage((msg) => {
    if (!msg || typeof msg !== 'object') return;
    if (msg.type === 'view-opened') {
      if (!state.tabs) state.tabs = [];
      let tab = state.tabs.find((t) => t.id === msg.view);
      if (!tab) {
        tab = { id: msg.view, title: 'New tab', parent: msg.opener, url: msg.url || '' };
        state.tabs.push(tab);
      } else if (msg.url) {
        tab.url = msg.url;
      }
      state.activeTab = msg.view;
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
      renderShell(app, state);
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
      renderShell(app, state);
    } else if (msg.type === 'helm-focus') {
      const input = document.querySelector('.helm-search-input, .helm-address, .helm');
      if (input) {
        input.focus();
        if (input.select) input.select();
      }
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
      renderShell(app, state);
    } else if (msg.type === 'url') {
      if (!state.tabs) state.tabs = [];
      let tab = state.tabs.find((t) => t.id === msg.view);
      if (!tab) {
        tab = { id: msg.view, title: 'New tab' };
        state.tabs.push(tab);
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
      renderShell(app, state);
    } else if (msg.type === 'title') {
      if (state.tabs) {
        const tab = state.tabs.find((t) => t.id === msg.view);
        if (tab) tab.title = msg.title;
      }
      renderShell(app, state);
    } else if (msg.type === 'loading') {
      if (state.activeTab === msg.view && state.helm && state.helm.kind === 'address') {
        state.isLoading = msg.loading;
        state.canGoBack = msg.can_go_back;
        state.canGoForward = msg.can_go_forward;
      }
    } else if (msg.type === 'settings-loaded') {
      state.settings = msg.settings;
      renderShell(app, state);
    } else if (msg.type === 'space-changed') {
      state.activeSpace = msg.space;
      renderShell(app, state);
    } else if (msg.type === 'history-loaded') {
      state.history = msg.groups;
      renderShell(app, state);
    } else if (msg.type === 'closed-tabs-updated') {
      state.closed = msg.tabs;
      renderShell(app, state);
    } else if (msg.type === 'shields-status') {
      if (state.activeTab === msg.view || state.activeTab == null) {
        state.shields = msg.count;
        state.shieldsEnabled = msg.enabled;
        renderShell(app, state);
      }
    } else if (msg.type === 'tab-sleeping') {
      if (state.tabs) {
        const tab = state.tabs.find((t) => t.id === msg.view);
        if (tab) tab.sleeping = msg.sleeping;
      }
      renderShell(app, state);
    } else if (msg.type === 'extensions-loaded') {
      state.extensions = msg.extensions;
      renderShell(app, state);
    } else if (msg.type === 'ai-config-loaded') {
      state.aiConfig = msg.config;
      renderShell(app, state);
    } else if (msg.type === 'ai-test-result') {
      state.aiTestResult = msg;
      renderShell(app, state);
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
          renderShell(app, state);
        } else if (msg.event.type === 'source') {
          const s = msg.event.data?.title || msg.event.data?.url;
          if (s && !state.answer.sources.includes(s)) {
            state.answer.sources.push(s);
            renderShell(app, state);
          }
        } else if (msg.event.type === 'error') {
          state.answer.text = `Error: ${msg.event.data}`;
          renderShell(app, state);
        }
      }
    } else if (msg.type === 'vault-loaded') {
      state.vaultEntries = msg.entries;
      renderShell(app, state);
    } else if (msg.type === 'vault-credentials') {
      state.revealedPassword = { id: msg.id, username: msg.username, password: msg.password };
      renderShell(app, state);
    }
  });

  renderShell(app, state);

  window.addEventListener('resize', () => {
    state.sky = generateSky('newtab', window.innerWidth || 1280, window.innerHeight || 800);
    renderShell(app, state);
  });

  if (typeof window.odysseyBridge === 'function') {
    window.odysseyBridge(JSON.stringify({ type: 'load-settings' }));
    window.odysseyBridge(JSON.stringify({ type: 'get-extensions' }));
    window.odysseyBridge(JSON.stringify({ type: 'ai-get-config' }));
    window.odysseyBridge(JSON.stringify({ type: 'vault-list' }));
  }
}
