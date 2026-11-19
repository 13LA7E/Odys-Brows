// The browser chrome's layout: sky, rail area, page panel, Helm, notch (DESIGN.md §4).
// Used by fixture mode now and by the live UI from Phase 2. Everything is built with DOM
// methods (never innerHTML with data), except the fixture's sample website, which is our own
// file from design/preview.

import { createSky } from './sky.js';
import { renderCollapsedRail, renderExpandedRail } from './rail.js';
import { renderHelm, renderDownloadsPopover, renderAnswerCard } from './helm.js';
import { renderSettings, applyAccent } from './settings.js';
import { renderHistory } from './history.js';
import { renderNewTab } from './newtab.js';
import { initPageBasicsShortcuts, renderErrorPage } from './pagebasics.js';

const SVG = 'http://www.w3.org/2000/svg';

function el(tag, cls, text) {
  const e = document.createElement(tag);
  if (cls) e.className = cls;
  if (text != null) e.textContent = text;
  return e;
}

function svg(tag, attrs) {
  const e = document.createElementNS(SVG, tag);
  for (const [k, v] of Object.entries(attrs)) e.setAttribute(k, v);
  return e;
}

function notch(count, onToggleShields) {
  const n = el('div', 'notch');
  const shields = el('button', 'notch-shields');
  shields.setAttribute('aria-label', `Shields, ${count} blocked`);
  shields.addEventListener('click', () => {
    if (typeof onToggleShields === 'function') onToggleShields();
  });
  const shieldIcon = svg('svg', {
    width: 16, height: 16, viewBox: '0 0 24 24', fill: 'none',
    stroke: 'currentColor', 'stroke-width': '1.6', 'stroke-linecap': 'round',
    'stroke-linejoin': 'round', 'aria-hidden': 'true'
  });
  shieldIcon.append(svg('path', { d: 'M12 3l7 3v5c0 5-3 8-7 10-4-2-7-5-7-10V6z' }));
  shields.append(shieldIcon, el('span', null, String(count)));
  n.append(shields);

  const actions = {
    Minimize: 'minimize-window',
    Maximize: 'maximize-window',
    Close: 'close-window',
  };

  const btns = [
    { label: 'Minimize', d: 'M6 12h12' },
    { label: 'Maximize', rect: true },
    { label: 'Close', d: 'M6 6l12 12M18 6L6 18' },
  ];
  for (const info of btns) {
    const b = el('button', 'notch-button');
    b.setAttribute('aria-label', info.label);
    const s = svg('svg', {
      width: 15, height: 15, viewBox: '0 0 24 24', fill: 'none',
      stroke: 'currentColor', 'stroke-width': '1.6', 'stroke-linecap': 'round',
      'stroke-linejoin': 'round', 'aria-hidden': 'true'
    });
    if (info.rect) {
      s.append(svg('rect', { x: 5, y: 5, width: 14, height: 14, rx: 2 }));
    } else {
      s.append(svg('path', { d: info.d }));
    }
    b.append(s);
    b.addEventListener('click', () => {
      if (typeof window.odysseyBridge === 'function') {
        window.odysseyBridge(JSON.stringify({ type: actions[info.label] }));
      }
    });
    n.append(b);
  }
  return n;
}

export function renderPrivateCopy(copy = {}) {
  const container = el('div', 'private-copy');
  container.style.cssText = 'position:absolute;left:150px;top:214px;width:520px;';

  const title = el('div', 'private-title', copy.title || 'Private voyage');
  title.style.cssText = 'font-size:46px;font-weight:600;letter-spacing:-1px;line-height:1.1;color:var(--text-strong);';

  const body = el('div', 'private-body', copy.body || 'This window leaves no trail behind. Nothing you open here is kept once you close it.');
  body.style.cssText = 'font-size:16px;line-height:1.6;color:var(--text-muted);margin-top:18px;';

  const bulletsWrapper = el('div', 'private-bullets');
  bulletsWrapper.style.cssText = 'margin-top:28px;';

  const bullets = copy.bullets || [
    'History is not saved',
    'Cookies and site data are cleared on close',
    'Shields stay on',
  ];

  bullets.forEach((bText) => {
    const row = el('div', 'private-bullet');
    row.style.cssText = 'display:flex;align-items:center;gap:12px;font-size:15px;color:var(--text-soft);height:34px;';
    const ringSvg = svg('svg', {
      width: 14, height: 14, viewBox: '0 0 14 14', 'aria-hidden': 'true',
    });
    ringSvg.append(svg('circle', {
      cx: 7, cy: 7, r: 4.5, fill: 'none', stroke: 'var(--accent)', 'stroke-width': '1.3',
    }));
    row.append(ringSvg, document.createTextNode(bText));
    bulletsWrapper.append(row);
  });

  container.append(title, body, bulletsWrapper);
  return container;
}

/** Render the whole window for one state (a fixture now, live state later). */
export function renderShell(root, state) {
  if (!window.__pageBasicsInit) {
    window.__pageBasicsInit = true;
    initPageBasicsShortcuts(() => document.getElementById('app'), () => state.activeTab);
  }
  root.replaceChildren();
  root.classList.toggle('private', !!state.private);
  if (state.settings?.accent) {
    applyAccent(state.settings.accent);
  }
  const railWidth = state.rail === 'expanded' ? 264 : 56;
  root.style.setProperty('--rail', `${railWidth}px`);
  if (typeof window.odysseyBridge === 'function') {
    window.odysseyBridge(JSON.stringify({ type: 'set-rail-width', width: railWidth }));
  }

  createSky(root, {
    stars: state.sky,
    mode: state.starsMode || 'still',
    shootingStars: state.shootingStars,
    private: state.private,
  });
  const rail = el('nav', `rail rail-${state.rail}`);
  rail.setAttribute('aria-label', 'Trail');
  if (state.rail === 'collapsed' || state.rail === 'private') {
    const tabs = state.tabs || Array.from({ length: state.tabCount || 1 }, (_, i) => ({ id: i + 1 }));
    renderCollapsedRail(rail, {
      tabs,
      activeTab: state.activeTab ?? 1,
      activeSpace: state.activeSpace ?? 0,
      isPrivate: !!state.private,
      onSelectTab: (id) => {
        state.activeTab = id;
        const current = state.tabs?.find((t) => t.id === id);
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
        renderShell(root, state);
        if (typeof window.odysseyBridge === 'function') {
          window.odysseyBridge(JSON.stringify({ type: 'show', view: id }));
        }
      },
      onNewTab: () => {
        state.page = 'none';
        state.helm = { kind: 'empty' };
        if (typeof window.odysseyBridge === 'function') {
          window.odysseyBridge(JSON.stringify({ type: 'open', url: '' }));
        } else {
          const nextId = (state.tabs ? Math.max(...state.tabs.map((t) => t.id), 0) : (state.tabCount || 1)) + 1;
          if (state.tabs) {
            state.tabs.push({ id: nextId, title: 'New tab' });
          } else {
            state.tabCount = (state.tabCount || 1) + 1;
          }
          state.activeTab = nextId;
          renderShell(root, state);
        }
      },
      onExpand: () => {
        state.rail = 'expanded';
        renderShell(root, state);
        if (typeof window.odysseyBridge === 'function') {
          window.odysseyBridge(JSON.stringify({ type: 'set-rail-width', width: 264 }));
        }
      },
      onOpenSettings: () => {
        if (typeof window.odysseyBridge === 'function') {
          window.odysseyBridge(JSON.stringify({ type: 'open', url: 'odyssey://settings' }));
        }
      },
      onSwitchSpace: (space) => {
        state.activeSpace = space;
        renderShell(root, state);
        if (typeof window.odysseyBridge === 'function') {
          window.odysseyBridge(JSON.stringify({ type: 'switch-space', space }));
        }
      },
      onOpenPinnedSite: (name) => {
        if (typeof window.odysseyBridge === 'function') {
          window.odysseyBridge(JSON.stringify({ type: 'open-pinned-site', name }));
        }
      },
    });
  } else if (state.rail === 'expanded') {
    renderExpandedRail(rail, {
      tabs: state.tabs || [],
      activeTab: state.activeTab ?? 1,
      activeSpace: state.activeSpace ?? 0,
      closed: state.closed || [],
      isPrivate: !!state.private,
      onSelectTab: (id) => {
        state.activeTab = id;
        const current = state.tabs?.find((t) => t.id === id);
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
        renderShell(root, state);
        if (typeof window.odysseyBridge === 'function') {
          window.odysseyBridge(JSON.stringify({ type: 'show', view: id }));
        }
      },
      onNewTab: () => {
        state.page = 'none';
        state.helm = { kind: 'empty' };
        if (typeof window.odysseyBridge === 'function') {
          window.odysseyBridge(JSON.stringify({ type: 'open', url: '' }));
        } else {
          const nextId = (state.tabs ? Math.max(...state.tabs.map((t) => t.id), 0) : 0) + 1;
          state.tabs.push({ id: nextId, title: 'New tab' });
          state.activeTab = nextId;
          renderShell(root, state);
        }
      },
      onCollapse: () => {
        state.rail = 'collapsed';
        renderShell(root, state);
        if (typeof window.odysseyBridge === 'function') {
          window.odysseyBridge(JSON.stringify({ type: 'set-rail-width', width: 56 }));
        }
      },
      onOpenSettings: () => {
        if (typeof window.odysseyBridge === 'function') {
          window.odysseyBridge(JSON.stringify({ type: 'open', url: 'odyssey://settings' }));
        }
      },
      onSwitchSpace: (space) => {
        state.activeSpace = space;
        renderShell(root, state);
        if (typeof window.odysseyBridge === 'function') {
          window.odysseyBridge(JSON.stringify({ type: 'switch-space', space }));
        }
      },
      onOpenPinnedSite: (name) => {
        if (typeof window.odysseyBridge === 'function') {
          window.odysseyBridge(JSON.stringify({ type: 'open-pinned-site', name }));
        }
      },
    });
  }
  root.append(rail);

  if (state.page !== 'none') {
    const panel = el('section', `page page-${state.page}`);
    panel.setAttribute('aria-label', 'Page');
    if (state.pageError) {
      renderErrorPage(panel, state.pageError, () => {
        if (typeof window.odysseyBridge === 'function') {
          window.odysseyBridge(JSON.stringify({ type: 'reload' }));
        }
      });
    } else if (state.page === 'web' && state.pageHtml) {
      // The fixture's sample website: our own file extracted from design/preview.
      const site = el('div', 'page-site');
      site.innerHTML = state.pageHtml;
      panel.append(site);
    } else if (state.page === 'internal') {
      const pageName = state.pageName || state.helm?.address?.name;
      if (pageName === 'history' || state.history) {
        renderHistory(panel, state.history, {
          onSearch: (q) => {
            if (typeof window.odysseyBridge === 'function') {
              window.odysseyBridge(JSON.stringify({ type: 'search-history', query: q }));
            }
          },
          onOpen: (url) => {
            if (typeof window.odysseyBridge === 'function') {
              window.odysseyBridge(JSON.stringify({ type: 'open', url }));
            }
          },
          onForget: (id) => {
            if (typeof window.odysseyBridge === 'function') {
              window.odysseyBridge(JSON.stringify({ type: 'forget-visit', id }));
            }
          },
          onClearDay: (dayStart) => {
            if (typeof window.odysseyBridge === 'function') {
              window.odysseyBridge(JSON.stringify({ type: 'clear-history-day', day_start_secs: dayStart }));
            }
          },
        });
      } else if (pageName === 'settings' || state.settings) {
        renderSettings(panel, state, (field, val) => {
          if (field === 'stars') {
            state.starsMode = val.toLowerCase();
          } else if (field === 'shootingStars') {
            state.shootingStars = val;
          } else if (field === 'trail') {
            state.rail = val === 'Wide' ? 'expanded' : 'collapsed';
          }
          renderShell(root, state);
        });
      }
    }
    root.append(panel);
    if (state.dimPage && !state.pageHtml?.includes('rgba(0,0,0,.5)')) {
      root.append(el('div', 'page-dim'));
    }
  }

  if (state.private && (state.page === 'none' || state.privateCopy)) {
    root.append(renderPrivateCopy(state.privateCopy || {}));
  } else if (state.page === 'none') {
    renderNewTab(root, state, (tabId) => {
      state.activeTab = tabId;
      const current = state.tabs?.find((t) => t.id === tabId);
      if (current?.url && current.url !== 'about:blank' && !current.url.startsWith('odyssey://')) {
        state.page = 'web';
      } else if (current?.url?.startsWith('odyssey://')) {
        state.page = 'internal';
      } else {
        state.page = 'none';
      }
      renderShell(root, state);
      if (typeof window.odysseyBridge === 'function') {
        window.odysseyBridge(JSON.stringify({ type: 'show', view: tabId }));
      }
    });
  }

  // Answer card (HelmAnswer board, DESIGN.md §7)
  if (state.answer) {
    root.append(renderAnswerCard(state.answer));
  }

  // Helm toolbar
  root.append(renderHelm(state.helm || { kind: 'empty' }, {
    onBack: () => {
      if (typeof window.odysseyBridge === 'function') {
        window.odysseyBridge(JSON.stringify({ type: 'back', view: state.activeTab }));
      }
    },
    onForward: () => {
      if (typeof window.odysseyBridge === 'function') {
        window.odysseyBridge(JSON.stringify({ type: 'forward', view: state.activeTab }));
      }
    },
    onReload: () => {
      if (typeof window.odysseyBridge === 'function') {
        window.odysseyBridge(JSON.stringify({ type: 'reload', view: state.activeTab }));
      }
    },
    onNavigate: (target) => {
      const url = typeof target === 'string' ? target : (target?.url || '');
      if (!url) return;
      if (typeof window.odysseyBridge === 'function') {
        if (state.activeTab) {
          window.odysseyBridge(JSON.stringify({ type: 'navigate', view: state.activeTab, url }));
        } else {
          window.odysseyBridge(JSON.stringify({ type: 'open', url }));
        }
      }
    },
    onOpenDownloads: () => {
      state.downloadsOpen = !state.downloadsOpen;
      renderShell(root, state);
    },
  }));

  // Downloads popover (Browsing board)
  if (state.downloadsOpen && state.downloads) {
    root.append(renderDownloadsPopover(state.downloads, {
      onControl: (id, action) => {
        if (typeof window.odysseyBridge === 'function') {
          window.odysseyBridge(JSON.stringify({ type: 'download-control', id, action }));
        }
      },
      onOpen: (path) => {
        if (typeof window.odysseyBridge === 'function') {
          window.odysseyBridge(JSON.stringify({ type: 'open-download', path }));
        }
      },
    }));
  }

  root.append(notch(state.shields ?? 0, () => {
    state.shieldsOpen = !state.shieldsOpen;
    renderShell(root, state);
  }));

  if (state.shieldsOpen) {
    root.append(renderShieldsPanel(state, () => {
      state.shieldsOpen = false;
      renderShell(root, state);
    }));
  }
}

export function renderShieldsPanel(state, onClose) {
  const panel = el('div', 'shields-popover');
  panel.style.cssText = 'position:absolute;top:42px;right:12px;width:280px;border-radius:14px;background:var(--surface);box-shadow:var(--shadow-popover);padding:16px;z-index:200;display:flex;flex-direction:column;gap:12px;';

  const header = el('div');
  header.style.cssText = 'display:flex;align-items:center;justify-content:space-between;';
  const title = el('div', null, 'Shields');
  title.style.cssText = 'font-size:15px;font-weight:600;color:var(--text-strong);';
  const close = el('button', null, '✕');
  close.style.cssText = 'color:var(--text-soft);cursor:pointer;font-size:13px;';
  close.addEventListener('click', onClose);
  header.append(title, close);

  const blockedRow = el('div');
  blockedRow.style.cssText = 'display:flex;align-items:center;gap:10px;padding:10px 12px;border-radius:8px;background:var(--w-field);';
  const countText = el('div', null, `${state.shields ?? 0} ads & trackers blocked`);
  countText.style.cssText = 'font-size:13.5px;color:var(--text);';
  blockedRow.append(countText);

  const host = state.helm?.address?.host || 'this site';
  const toggleRow = el('div');
  toggleRow.style.cssText = 'display:flex;align-items:center;justify-content:space-between;padding:4px 0;';
  const label = el('div');
  label.style.cssText = 'font-size:13.5px;color:var(--text-soft);';
  label.textContent = `Shields for ${host}`;

  const switchBtn = el('button', 'shields-toggle-btn');
  const isEnabled = state.shieldsEnabled !== false;
  switchBtn.style.cssText = `width:42px;height:24px;border-radius:12px;background:${isEnabled ? 'var(--accent)' : 'var(--w-toggle-off)'};position:relative;cursor:pointer;transition:background 0.15s;`;
  const knob = el('span');
  knob.style.cssText = `position:absolute;top:3px;${isEnabled ? 'right:3px;' : 'left:3px;'}width:18px;height:18px;border-radius:9px;background:var(--white);transition:all 0.15s;display:block;`;
  switchBtn.append(knob);
  switchBtn.addEventListener('click', () => {
    state.shieldsEnabled = !isEnabled;
    if (typeof window.odysseyBridge === 'function') {
      window.odysseyBridge(JSON.stringify({ type: 'toggle-shields-exception', host }));
    }
    onClose();
  });
  toggleRow.append(label, switchBtn);

  const footer = el('div', null, 'EasyList & EasyPrivacy active');
  footer.style.cssText = 'font-size:12px;color:var(--text-muted);border-top:1px solid var(--w-trail);padding-top:8px;';

  panel.append(header, blockedRow, toggleRow, footer);
  return panel;
}
