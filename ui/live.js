// Live browser UI: connects shell chrome to engine events (P2-01+).
import { FIXTURES, SHIELDS_COUNT } from './fixtures.js';
import { renderShell } from './shell.js';
import { onMessage } from './bridge.js';

export async function startLive() {
  const app = document.getElementById('app');
  let sky = [];
  try {
    const skyData = await (await fetch('fixtures/sky.json')).json();
    sky = skyData.Main || [];
  } catch (_) {
    sky = [];
  }

  const base = FIXTURES.Main || {};
  const state = {
    ...base,
    sky,
    shields: SHIELDS_COUNT,
    activeSpace: 0,
  };

  onMessage((msg) => {
    if (!msg || typeof msg !== 'object') return;
    if (msg.type === 'view-opened') {
      if (!state.tabs) state.tabs = [];
      if (!state.tabs.some((t) => t.id === msg.view)) {
        state.tabs.push({ id: msg.view, title: 'New tab', parent: msg.opener });
      }
      state.activeTab = msg.view;
      renderShell(app, state);
    } else if (msg.type === 'closed') {
      if (state.tabs) {
        state.tabs = state.tabs.filter((t) => t.id !== msg.view);
        if (state.activeTab === msg.view) {
          state.activeTab = state.tabs[0]?.id ?? 1;
        }
      }
      renderShell(app, state);
    } else if (msg.type === 'url') {
      if (state.tabs) {
        const tab = state.tabs.find((t) => t.id === msg.view);
        if (tab) tab.url = msg.url;
      }
      if (state.activeTab === msg.view) {
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
          state.helm = { kind: 'address', address: { kind: 'web', host: msg.url, path: '' } };
        }
      }
      renderShell(app, state);
    } else if (msg.type === 'title') {
      if (state.tabs) {
        const tab = state.tabs.find((t) => t.id === msg.view);
        if (tab) tab.title = msg.title;
      }
      renderShell(app, state);
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
    }
  });

  renderShell(app, state);

  if (typeof window.odysseyBridge === 'function') {
    window.odysseyBridge(JSON.stringify({ type: 'load-settings' }));
  }
}
