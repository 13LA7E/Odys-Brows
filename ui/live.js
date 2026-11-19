// Live browser UI: connects shell chrome to engine events (P2-01+).
import { SHIELDS_COUNT } from './fixtures.js';
import { renderShell } from './shell.js';
import { onMessage } from './bridge.js';
import { initPageBasicsShortcuts } from './pagebasics.js';

export async function startLive() {
  const app = document.getElementById('app');
  let sky = [];
  try {
    const skyData = await (await fetch('fixtures/sky.json')).json();
    sky = skyData.Main || [];
  } catch (_) {
    sky = [];
  }

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
      const existing = state.tabs.find((t) => t.id === msg.view);
      if (!existing) {
        state.tabs.push({ id: msg.view, title: 'New tab', parent: msg.opener });
      }
      state.activeTab = msg.view;
      const tab = state.tabs.find((t) => t.id === msg.view);
      if (tab?.url && tab.url !== 'about:blank' && !tab.url.startsWith('odyssey://')) {
        state.page = 'web';
        try {
          const u = new URL(tab.url);
          state.helm = { kind: 'address', address: { kind: 'web', host: u.host, path: u.pathname } };
        } catch (_) {
          state.helm = { kind: 'address', address: { kind: 'web', host: tab.url, path: '' } };
        }
      } else if (tab?.url?.startsWith('odyssey://')) {
        state.page = 'internal';
        try {
          const u = new URL(tab.url);
          state.pageName = u.hostname;
          state.helm = { kind: 'address', address: { kind: 'internal', name: u.hostname } };
        } catch (_) {
          state.helm = { kind: 'empty' };
        }
      } else {
        state.page = 'none';
        state.helm = { kind: 'empty' };
      }
      renderShell(app, state);
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
      if (state.tabs) {
        const tab = state.tabs.find((t) => t.id === msg.view);
        if (tab) tab.url = msg.url;
      }
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
            state.page = 'web';
            state.helm = { kind: 'address', address: { kind: 'web', host: msg.url, path: '' } };
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
    }
  });

  renderShell(app, state);

  if (typeof window.odysseyBridge === 'function') {
    window.odysseyBridge(JSON.stringify({ type: 'load-settings' }));
  }
}
