// The browser chrome's layout: sky, rail area, page panel, Helm, notch (DESIGN.md §4).
// Used by fixture mode now and by the live UI from Phase 2. Everything is built with DOM
// methods (never innerHTML with data), except the fixture's sample website, which is our own
// file from design/preview.

import { createSky } from './sky.js';
import { renderCollapsedRail, renderExpandedRail } from './rail.js';
import { renderHelm, renderDownloadsPopover, renderAnswerCard, setSearchEngine } from './helm.js';
import { renderSettings, applyAccent } from './settings.js';
import { applySavedTheme, resolveTheme, starColour } from './themes.js';
import { renderHistory, formatDayGroups } from './history.js';
import { renderNewTab } from './newtab.js';
import { renderErrorPage, renderPromptCard } from './pagebasics.js';
import { renderExtensionsPage } from './extensions.js';

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

function notch(count, onToggleShields, onOpenExtensions, showExtensions = true) {
  const n = el('div', 'notch');
  if (showExtensions) {
    const extBtn = el('button', 'notch-extensions');
    extBtn.setAttribute('aria-label', 'Extensions');
    extBtn.style.cssText =
      'padding:0 8px;height:34px;background:transparent;border:none;color:var(--text-soft);cursor:pointer;display:flex;align-items:center;justify-content:center;';
    const extSvg = svg('svg', {
      width: 15,
      height: 15,
      viewBox: '0 0 24 24',
      fill: 'none',
      stroke: 'currentColor',
      'stroke-width': '1.6',
      'stroke-linecap': 'round',
      'stroke-linejoin': 'round',
      'aria-hidden': 'true',
    });
    extSvg.append(
      svg('path', {
        d: 'M4 11a2 2 0 0 1 .4-1.2l2.6-3.8A2 2 0 0 1 8.6 5H19a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H8.6a2 2 0 0 1-1.6-.8L4.4 14.2A2 2 0 0 1 4 13v-2z',
      })
    );
    extBtn.append(extSvg);
    extBtn.addEventListener('click', () => {
      if (typeof onOpenExtensions === 'function') onOpenExtensions();
    });
    n.append(extBtn);
  }

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

// --- Rendering helpers (AUDIT-03, AUDIT-04) -------------------------------------------------
// The window is still rebuilt from state, but: renders are coalesced and skipped when nothing
// that is drawn changed; the sky is one long-lived instance; the rail width is only sent when it
// changes; and a text field the user is typing in keeps its text and caret.

let skyHandle = null;
let skyKey = '';
let lastRailWidth = null;
let lastPanelKind = null;
let lastSignature = '';
let renderQueued = false;
window.__renderCount = 0;

function signature(state) {
  try {
    // The window size is part of what is drawn (the sky is made for it), so a resize must render.
    return JSON.stringify(state, (k, v) => (k === 'sky' ? undefined : v)) + `|${window.innerWidth}x${window.innerHeight}`;
  } catch (_) {
    return String(Math.random());
  }
}

// --- Regions (AUDIT-03) ----------------------------------------------------------------------
// Every region of the window (rail, panel, Helm, notch, popovers...) is built once and kept while
// the data it draws is unchanged. renderShell only decides which nodes should be there and
// reconcile() puts them in order, so untouched nodes are never removed or re-inserted: their
// hover state, scroll position and running transitions survive any number of renders.
const regionCache = new Map();
let regionsUsed = new Set();

function region(key, sig, build, opts = {}) {
  regionsUsed.add(key);
  const s = JSON.stringify(sig);
  const cached = regionCache.get(key);
  if (cached && cached.sig === s) return cached.el;
  const el = build();
  if (cached) {
    // Same region, new data: swap without any enter or exit animation.
    cached.el.dataset.exit = '0';
  } else if (opts.enter && document.documentElement.dataset.motion !== 'off') {
    el.classList.add('enter');
    el.addEventListener('animationend', () => el.classList.remove('enter'), { once: true });
  }
  if (opts.exit) el.dataset.exit = '1';
  regionCache.set(key, { sig: s, el });
  return el;
}

function cssMs(name, fallback) {
  const v = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  const n = parseFloat(v);
  if (Number.isNaN(n)) return fallback;
  return v.endsWith('ms') ? n : v.endsWith('s') ? n * 1000 : n;
}

/** Make root's children exactly `nodes`, in order, moving or removing only what differs. */
function reconcile(root, nodes) {
  const want = new Set(nodes);
  const motionOff = document.documentElement.dataset.motion === 'off';
  for (const child of [...root.children]) {
    if (want.has(child)) {
      child.classList.remove('leaving');
      continue;
    }
    if (child.dataset && child.dataset.exit === '1' && !motionOff) {
      if (!child.classList.contains('leaving')) {
        child.classList.add('leaving');
        setTimeout(() => {
          if (child.classList.contains('leaving') && !want.has(child)) child.remove();
        }, cssMs('--dur-base', 200) + 30);
      }
      continue;
    }
    child.remove();
  }
  let ref = root.firstChild;
  const skip = () => {
    while (ref && ref.classList && ref.classList.contains('leaving') && !want.has(ref)) ref = ref.nextSibling;
  };
  skip();
  for (const n of nodes) {
    if (n === ref) {
      ref = ref.nextSibling;
      skip();
    } else {
      root.insertBefore(n, ref);
    }
  }
}

const PANEL_EXCLUDE = new Set([
  'sky', 'tabs', 'activeTab', 'closed', 'helm', 'savedHelm', 'answer', 'dimPage', 'downloads',
  'downloadsOpen', 'shields', 'shieldsOpen', 'shieldsEnabled', 'prompts', 'isLoading',
  'canGoBack', 'canGoForward', 'activeSpace', 'minute', 'tabCount',
]);
/** What the page panel draws: everything except the data only other regions use. */
function panelSig(state) {
  const o = {};
  for (const k of Object.keys(state)) if (!PANEL_EXCLUDE.has(k)) o[k] = state[k];
  return o;
}

/** Sky options from live state, falling back to the saved settings. */
function skyOptions(state) {
  const mode = String(state.starsMode || state.settings?.stars || 'still').toLowerCase();
  const comets = state.shootingStars ?? state.settings?.shootingStars ?? false;
  const cursor = state.settings?.starsFollowCursor !== false;
  return { mode, comets: !!comets, cursor };
}

function mountSky(state, add) {
  const first = state.sky && state.sky[0];
  const so = skyOptions(state);
  const key = JSON.stringify([
    so.mode, so.comets, so.cursor, !!state.private, starColour(state.settings, !!state.private),
    window.innerWidth, window.innerHeight, state.sky ? state.sky.length : 0,
    first ? [first.x, first.y] : 0,
  ]);
  if (!(skyHandle && skyKey === key)) {
    if (skyHandle) skyHandle.destroy();
    skyHandle = createSky(null, {
      stars: state.sky,
      mode: so.mode,
      shootingStars: so.comets,
      cursor: so.cursor,
      starColor: starColour(state.settings, !!state.private),
      private: state.private,
    });
    skyKey = key;
  }
  add(skyHandle.canvas);
  // A web page covers the sky; don't animate what nobody can see (AUDIT-34).
  skyHandle.setPaused(state.live === true && state.page === 'web');
}

function captureFocus(root) {
  const a = document.activeElement;
  if (!a || !root.contains(a) || !(a.tagName === 'INPUT' || a.tagName === 'TEXTAREA')) return null;
  return {
    tag: a.tagName, cls: a.className, ph: a.placeholder, value: a.value,
    start: a.selectionStart, end: a.selectionEnd,
  };
}

function restoreFocus(root, f) {
  if (!f) return;
  const target = [...root.querySelectorAll(f.tag.toLowerCase())]
    .find((e) => e.className === f.cls && e.placeholder === f.ph);
  if (!target) return;
  if (target.value !== f.value) target.value = f.value;
  target.focus({ preventScroll: true });
  try { target.setSelectionRange(f.start, f.end); } catch (_) { /* not a text field */ }
}

/**
 * Live mode: ask for a render. Several calls in one task become one render, nothing is drawn if
 * the state is unchanged, and the Helm's address field is never replaced while it is being edited.
 */
export function requestRender(root, state) {
  if (renderQueued) return;
  renderQueued = true;
  Promise.resolve().then(() => {
    renderQueued = false;
    const a = document.activeElement;
    if (a && a.classList && a.classList.contains('helm-input') && a.style.display !== 'none') {
      a.addEventListener('blur', () => requestRender(root, state), { once: true });
      return;
    }
    if (signature(state) === lastSignature) return;
    renderShell(root, state);
  });
}

/** Render the whole window for one state (a fixture now, live state later). */
/** Opens a link from our own pages (History, answer sources): a new tab, or this tab, per Settings. */
export function openLink(state, url) {
  if (typeof window.odysseyBridge !== 'function' || !url) return;
  const same = state.settings?.linkTarget === 'Same tab' && state.activeTab;
  window.odysseyBridge(JSON.stringify(same
    ? { type: 'navigate', view: state.activeTab, url }
    : { type: 'open', url }));
}

export function renderShell(root, state) {
  window.__renderCount += 1;
  lastSignature = signature(state);
  const focused = captureFocus(root);
  const nodes = [];
  const add = (n) => { if (n) nodes.push(n); };
  regionsUsed = new Set();
  root.classList.toggle('private', !!state.private);
  root.dataset.overPage = state.page === 'web' ? '1' : '0';
  syncNotch(root, state);
  setSearchEngine(state.settings);
  applySavedTheme(state.settings, !!state.private);
  // A theme sets its own accent; without one the chosen accent swatch applies.
  if (state.settings?.accent && !(resolveTheme(state.settings) && !state.private)) {
    applyAccent(state.settings.accent);
  }
  const railWidth = state.rail === 'expanded' ? 264 : 56;
  root.style.setProperty('--rail', `${railWidth}px`);
  if (typeof window.odysseyBridge === 'function' && railWidth !== lastRailWidth) {
    const grow = railWidth > (lastRailWidth ?? 0);
    lastRailWidth = railWidth;
    const send = () => window.odysseyBridge(JSON.stringify({ type: 'set-rail-width', width: railWidth }));
    // The page window jumps at once; when the rail shrinks, wait so the rail's width animation
    // isn't cut off by it.
    if (grow || document.documentElement.dataset.motion === 'off') send();
    else setTimeout(send, cssMs('--dur-slow', 320));
  }

  mountSky(state, add);
  const railNode = region('rail', {
    rail: state.rail, tabs: state.tabs ?? state.tabCount, activeTab: state.activeTab,
    activeSpace: state.activeSpace, closed: state.closed, priv: !!state.private,
  }, () => {
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
        renderShell(root, state);
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
          /* the width is sent by renderShell */
        }
      },
      onOpenSettings: () => {
        state.page = 'internal';
        state.pageName = 'settings';
        state.helm = { kind: 'address', address: { kind: 'internal', name: 'settings' } };
        renderShell(root, state);
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
        renderShell(root, state);
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
          /* the width is sent by renderShell (after the rail has shrunk) */
        }
      },
      onOpenSettings: () => {
        state.page = 'internal';
        state.pageName = 'settings';
        state.helm = { kind: 'address', address: { kind: 'internal', name: 'settings' } };
        renderShell(root, state);
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
  return rail;
  });
  add(railNode);

  if (state.page !== 'none') {
    const panelNode = region('panel', panelSig(state), () => {
    const panel = el('section', `page page-${state.page}`);
    const panelKind = `${state.page}:${state.pageName || ''}`;
    if (panelKind !== lastPanelKind) {
      if (lastPanelKind !== null && document.documentElement.dataset.motion !== 'off') panel.classList.add('page-in');
      lastPanelKind = panelKind;
    }
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
      // Fixtures have no page name and pick by the data they carry; live mode goes by name only.
      const byData = state.live !== true;
      if (pageName === 'history' || (byData && state.history)) {
        let historyView = state.history;
        if (state.live) {
          // Live data arrives as day groups from Rust (AUDIT-09).
          const days = formatDayGroups(Array.isArray(state.history) ? state.history : []);
          const rows = days.flatMap((d) => d.rows);
          const branches = rows.filter((r) => r.branch).length;
          historyView = {
            days,
            searchQuery: state.historyQuery || '',
            side: {
              title: days[0]?.label || 'History',
              summary: rows.length
                ? `${rows.length} ${rows.length === 1 ? 'page' : 'pages'}, ${branches} ${branches === 1 ? 'branch' : 'branches'}.`
                : 'Nothing here yet. Pages you visit will appear on your trail.',
            },
          };
        }
        renderHistory(panel, historyView, {
          onSearch: (q) => {
            state.historyQuery = q;
            if (typeof window.odysseyBridge === 'function') {
              window.odysseyBridge(JSON.stringify(q.trim()
                ? { type: 'search-history', query: q }
                : { type: 'query-history' }));
            }
          },
          onOpen: (url) => openLink(state, url),
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
      } else if (pageName === 'settings' || (byData && state.settings)) {
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
      } else if (pageName === 'extensions') {
        renderExtensionsPage(panel, state.extensions || [], {
          onToggle: (id, enabled) => {
            if (typeof window.odysseyBridge === 'function') {
              window.odysseyBridge(JSON.stringify({ type: 'toggle-extension', id, enabled }));
            }
          },
          onRemove: (id) => {
            if (typeof window.odysseyBridge === 'function') {
              window.odysseyBridge(JSON.stringify({ type: 'remove-extension', id }));
            }
          },
          onLoadUnpacked: (path) => {
            if (typeof window.odysseyBridge === 'function') {
              window.odysseyBridge(JSON.stringify({ type: 'load-unpacked-extension', path }));
            }
          },
        });
      }
    }
    return panel;
    });
    add(panelNode);
    if (state.dimPage && !state.pageHtml?.includes('rgba(0,0,0,.5)')) {
      add(region('dim', 1, () => el('div', 'page-dim'), { enter: true, exit: true }));
    }
  }

  if (state.private && (state.page === 'none' || state.privateCopy)) {
    add(region('private-copy', state.privateCopy || {}, () => renderPrivateCopy(state.privateCopy || {})));
  } else if (state.page === 'none') {
    add(region('newtab', {
      tabs: (state.tabs || []).map((t) => [t.id, t.title, t.url, t.parent]),
      activeTab: state.activeTab, newTab: state.newTab, minute: state.minute,
    }, () => {
    const wrap = el('div', 'newtab-region');
    wrap.style.display = 'contents';
    renderNewTab(wrap, state, (tabId) => {
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
    return wrap;
    }));
  }

  // Answer card (HelmAnswer board, DESIGN.md §7)
  if (state.answer) {
    add(region('answer', state.answer, () => renderAnswerCard(state.answer, (url) => openLink(state, url)), { enter: true, exit: true }));
  }

  // Permission and certificate questions for the tab being shown (AUDIT-21).
  const prompt = (state.prompts || []).find((p) => p.view === state.activeTab);
  if (prompt) {
    add(region('prompt', prompt, () => renderPromptCard(prompt, (answer) => {
      state.prompts = state.prompts.filter((p) => p !== prompt);
      if (typeof window.odysseyBridge === 'function') {
        const msgs = {
          permission: { type: 'answer-permission', request: prompt.request, allow: answer },
          certificate: { type: 'answer-certificate', request: prompt.request, proceed: answer },
          'save-login': { type: 'answer-save-login', request: prompt.request, action: answer },
          'fill-choice': { type: 'answer-fill', request: prompt.request, id: answer },
        };
        window.odysseyBridge(JSON.stringify(msgs[prompt.kind]));
      }
      renderShell(root, state);
    }), { enter: true, exit: true }));
  }

  // Helm toolbar. The download ring follows the active download (AUDIT-05).
  const activeDl = (state.downloads || []).find((d) => d.active);
  const recentlyDone = !activeDl && state.dlDoneAt && Date.now() - state.dlDoneAt < 1200;
  const helmState = (activeDl || recentlyDone) && state.helm?.kind === 'address'
    ? { ...state.helm, download: activeDl ? (activeDl.progress ?? 0) : 1, downloadDone: !activeDl }
    : (state.helm || { kind: 'empty' });
  // The ring's progress is updated in place below (so it can animate), not by rebuilding the Helm.
  const helmSig = { ...helmState, download: helmState.download != null ? 'ring' : null };
  add(region('helm', helmSig, () => renderHelm(helmState, {
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
    onToggleAsk: () => {
      if (state.helm?.kind === 'asking') {
        state.helm = state.savedHelm || { kind: 'empty' };
        state.answer = null;
        state.dimPage = false;
        if (typeof window.odysseyBridge === 'function') {
          window.odysseyBridge(JSON.stringify({ type: 'ai-cancel' }));
        }
      } else {
        state.savedHelm = state.helm;
        state.helm = { kind: 'asking', question: '' };
        state.dimPage = true;
      }
      renderShell(root, state);
    },
    onAsk: (question) => {
      state.dimPage = true;
      state.answer = {
        text: 'Thinking...',
        sources: [],
        footer: state.aiConfig?.provider === 'odysseus'
          ? 'Answered by your Odysseus server'
          : (state.aiConfig?.provider === 'cloud'
            ? 'Answered by Cloud AI'
            : (state.aiConfig?.provider === 'local' ? 'Answered by local AI' : 'AI not configured')),
        action: state.aiConfig?.provider === 'odysseus' ? 'Continue in Odysseus' : '',
      };
      renderShell(root, state);
      if (typeof window.odysseyBridge === 'function') {
        window.odysseyBridge(JSON.stringify({ type: 'ai-ask', prompt: question }));
      }
    },
  })));

  // Downloads popover (Browsing board)
  if (state.downloadsOpen && state.downloads && state.downloads.length) {
    add(region('downloads', state.downloads, () => renderDownloadsPopover(state.downloads, {
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
    }), { enter: true, exit: true }));
  }

  if (autoHideOn && state.live) {
    add(region('notch-grip', 1, () => {
      const grip = el('div', 'notch-grip');
      grip.append(el('span'));
      return grip;
    }));
    add(region('helm-grip', 1, () => {
      const grip = el('div', 'helm-grip');
      grip.append(el('span'));
      return grip;
    }));
  }
  add(region('notch', {
    count: state.live ? (state.page === 'web' ? state.shields ?? 0 : 0) : state.shields ?? 0,
    open: !!state.shieldsOpen,
  }, () =>
    notch(
      // Live: only a web page has a count (AUDIT-25).
      state.live ? (state.page === 'web' ? state.shields ?? 0 : 0) : state.shields ?? 0,
      () => {
        state.shieldsOpen = !state.shieldsOpen;
        renderShell(root, state);
      },
      () => {
        state.page = 'internal';
        state.pageName = 'extensions';
        state.helm = { kind: 'address', address: { kind: 'internal', name: 'extensions' } };
        renderShell(root, state);
        if (typeof window.odysseyBridge === 'function') {
          window.odysseyBridge(JSON.stringify({ type: 'get-extensions' }));
        }
      },
      !document.documentElement.classList.contains('fixture')
    )
  ));

  if (state.shieldsOpen) {
    add(region('shields', { c: state.shields, e: state.shieldsEnabled, h: state.helm?.address?.host }, () => renderShieldsPanel(state, () => {
      state.shieldsOpen = false;
      renderShell(root, state);
    }), { enter: true, exit: true }));
  }

  for (const k of [...regionCache.keys()]) if (!regionsUsed.has(k)) regionCache.delete(k);
  reconcile(root, nodes);
  const ring = root.querySelector('.helm-dl-progress');
  if (ring && helmState.download != null) {
    const d = 69.1 * Math.max(0, Math.min(1, helmState.download));
    ring.setAttribute('stroke-dasharray', `${d.toFixed(1)} ${(69.1 - d).toFixed(1)}`);
  }
  root.querySelector('.helm')?.classList.toggle('is-loading', !!state.isLoading && state.page === 'web');
  restoreFocus(root, focused);
  watchOverlays(root, state);
}

// --- Auto-hide of the notch and the Helm over web pages (AUDIT-28) -----------------------------
// Over a web page the notch would cover the page's own top-right controls and the Helm its bottom.
// Each shows for a moment after the page appears, slides away, and comes back while the pointer is
// on its thin strip (or, for the Helm, while you type, ask, or a download is open). While one
// moves, the hole it has in the page window follows it frame by frame, so the page shows through
// as it goes: the slide is visible instead of a black box vanishing.
const HIDERS = {
  notch: { hidden: false, timer: 0, hover: false, showMs: 2500, hideAfterMs: 700, sel: '.notch, .notch-grip' },
  helm: { hidden: false, timer: 0, hover: false, showMs: 4500, hideAfterMs: 1200, sel: '.helm, .helm-grip' },
};
let autoHideOn = false;
let helmBusy = false;
let hiderWired = false;
let lastHelmUrlKey = '';
let shellRoot = null;

function setHidden(name, hidden) {
  const h = HIDERS[name];
  h.hidden = hidden;
  shellRoot.dataset[name] = hidden ? 'hidden' : 'shown';
}

function scheduleHide(name, ms) {
  const h = HIDERS[name];
  clearTimeout(h.timer);
  h.timer = setTimeout(() => {
    if (autoHideOn && !h.hover && !(name === 'helm' && helmBusy)) setHidden(name, true);
  }, ms);
}

/** Bring the Helm back (Ctrl+L, typing, a new address) and let it hide again later. */
export function revealHelm() {
  if (!shellRoot || !autoHideOn) return;
  setHidden('helm', false);
  scheduleHide('helm', HIDERS.helm.showMs);
}

function syncNotch(root, state) {
  shellRoot = root;
  const want = state.live === true && state.page === 'web' && !state.shieldsOpen
    && !(state.prompts || []).some((p) => p.view === state.activeTab);
  helmBusy = state.helm?.kind === 'asking' || !!state.answer || !!state.downloadsOpen;
  if (!hiderWired && state.live) {
    hiderWired = true;
    for (const name of Object.keys(HIDERS)) {
      const h = HIDERS[name];
      const near = (n) => n && n.closest && n.closest(h.sel);
      root.addEventListener('pointerover', (e) => {
        if (!near(e.target)) return;
        h.hover = true;
        clearTimeout(h.timer);
        if (autoHideOn) setHidden(name, false);
      });
      root.addEventListener('pointerout', (e) => {
        if (!near(e.target) || near(e.relatedTarget)) return;
        h.hover = false;
        if (autoHideOn) scheduleHide(name, h.hideAfterMs);
      });
    }
    // Typing in the address bar keeps the Helm up.
    root.addEventListener('focusin', (e) => {
      if (e.target.closest && e.target.closest('.helm')) {
        HIDERS.helm.hover = true;
        clearTimeout(HIDERS.helm.timer);
        if (autoHideOn) setHidden('helm', false);
      }
    });
    root.addEventListener('focusout', (e) => {
      if (e.target.closest && e.target.closest('.helm')) {
        HIDERS.helm.hover = false;
        if (autoHideOn) scheduleHide('helm', HIDERS.helm.hideAfterMs);
      }
    });
  }
  if (want && !autoHideOn) {
    autoHideOn = true;
    for (const name of Object.keys(HIDERS)) {
      setHidden(name, false);
      scheduleHide(name, HIDERS[name].showMs); // visible for a moment when the page appears
    }
  } else if (!want && autoHideOn) {
    autoHideOn = false;
    for (const name of Object.keys(HIDERS)) {
      clearTimeout(HIDERS[name].timer);
      setHidden(name, false);
    }
  }
  if (autoHideOn) {
    if (helmBusy) {
      setHidden('helm', false);
    }
    // A new address or tab: show the Helm for a moment.
    const a = state.helm?.address;
    const key = `${state.activeTab}|${a ? `${a.host || ''}${a.path || ''}${a.name || ''}` : ''}`;
    if (key !== lastHelmUrlKey) {
      lastHelmUrlKey = key;
      revealHelm();
    }
  }
}

// --- Overlays over the page (AUDIT-06) ---------------------------------------------------
// Page windows cover our UI, so each open popover or card tells Rust where it is and a hole is
// cut there. A DOM observer keeps it current for things drawn outside renderShell (find bar,
// hover previews).
const OVERLAY_SELECTOR = '.downloads-popover, .answer-card, .shields-popover, .find-bar, .closed-preview-card, .permission-card, .helm, .helm-grip';
let overlayObserver = null;
let lastOverlayJson = '[]';
let lastNotchHeight = 34;
let overlayTimer = 0;

function reportOverlays() {
  if (typeof window.odysseyBridge !== 'function') return;
  const rects = [...document.querySelectorAll(OVERLAY_SELECTOR)].map((e) => {
    const r = e.getBoundingClientRect();
    const radius = parseFloat(getComputedStyle(e).borderTopLeftRadius) || 0;
    return { x: Math.round(r.left), y: Math.round(r.top), width: Math.round(r.width), height: Math.round(r.height), radius: Math.round(radius) };
  }).filter((r) => r.width > 0 && r.height > 0);
  // How much of the notch is showing (it slides away): the page window's notch hole follows.
  const notch = document.querySelector('.notch');
  if (notch) {
    const h = Math.max(4, Math.min(34, Math.round(notch.getBoundingClientRect().bottom)));
    if (h !== lastNotchHeight) {
      lastNotchHeight = h;
      (window.__notchHeights = window.__notchHeights || []).push(h);
      window.odysseyBridge(JSON.stringify({ type: 'set-notch-height', height: h }));
    }
  }
  const json = JSON.stringify(rects);
  if (json === lastOverlayJson) return;
  lastOverlayJson = json;
  window.odysseyBridge(JSON.stringify({ type: 'set-overlay-rects', rects }));
}

// While a card animates its rectangle moves every frame: keep the hole glued to it.
let trackingOverlays = false;
function trackOverlayAnimations() {
  if (trackingOverlays) return;
  trackingOverlays = true;
  let frames = 0;
  const tick = () => {
    reportOverlays();
    frames += 1;
    // The notch is tracked too: its hole height follows it while it slides.
    const animating = [...document.querySelectorAll(`${OVERLAY_SELECTOR}, .notch`)].some((e) => e.getAnimations().length > 0);
    if (animating && frames < 120) {
      requestAnimationFrame(tick);
    } else {
      trackingOverlays = false;
      reportOverlays();
    }
  };
  requestAnimationFrame(tick);
}

function watchOverlays(root, state) {
  if (!state.live) return;
  window.__overlayRects = () => JSON.parse(lastOverlayJson);
  if (!overlayObserver) {
    overlayObserver = new MutationObserver(() => {
      clearTimeout(overlayTimer);
      overlayTimer = setTimeout(reportOverlays, 30);
    });
    overlayObserver.observe(root, { childList: true, subtree: true });
    root.addEventListener('animationstart', trackOverlayAnimations, true);
    root.addEventListener('transitionrun', trackOverlayAnimations, true);
    window.addEventListener('resize', () => { clearTimeout(overlayTimer); overlayTimer = setTimeout(reportOverlays, 60); });
  }
  clearTimeout(overlayTimer);
  overlayTimer = setTimeout(reportOverlays, 30);
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
