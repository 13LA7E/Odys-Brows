// The browser chrome's layout: sky, rail area, page panel, Helm, notch (DESIGN.md §4).
// Used by fixture mode now and by the live UI from Phase 2. Everything is built with DOM
// methods (never innerHTML with data), except the fixture's sample website, which is our own
// file from design/preview.

import { createSky } from './sky.js';
import { renderCollapsedRail, renderExpandedRail } from './rail.js';
import { renderHelm, renderDownloadsPopover, renderAnswerCard, setSearchEngine } from './helm.js';
import { renderSettings, applyAccent, saveSettings } from './settings.js';
import { applySavedTheme, resolveTheme, starColour } from './themes.js';
import { applyInterface, interfaceScale } from './interface.js';
import { renderHistory, formatDayGroups } from './history.js';
import { renderNewTab, wallpaperVersion } from './newtab.js';
import { renderChatPage, chatUpdate } from './chat.js';
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

function notch(count, onToggleShields, onOpenExtensions, showExtensions = true, pinned = []) {
  const n = el('div', 'notch');
  // Pinned extensions get their own button, so the notch grows with each one (AUDIT: owner request).
  for (const ext of pinned) {
    const b = el('button', 'notch-pinned');
    b.setAttribute('aria-label', ext.name);
    b.title = ext.name;
    b.dataset.extId = ext.id;
    b.style.cssText = 'padding:0 6px;height:34px;background:transparent;border:none;color:var(--text-soft);cursor:pointer;display:flex;align-items:center;justify-content:center;';
    const chip = el('span', null, (ext.name || '?').trim().charAt(0).toUpperCase());
    chip.style.cssText = 'width:20px;height:20px;border-radius:6px;background:var(--w-field);color:var(--accent);font-size:12px;font-weight:600;display:flex;align-items:center;justify-content:center;';
    b.append(chip);
    // The button opens the extension's own popup when it has one, else the Extensions page.
    b.addEventListener('click', () => {
      if (ext.popup) window.odysseyBridge?.(JSON.stringify({ type: 'open-extension-popup', id: ext.id }));
      else if (typeof onOpenExtensions === 'function') onOpenExtensions();
    });
    n.append(b);
  }
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
  'chat', 'chatList', 'chatStream', 'aiOrigin', 'helmChat', 'chatOpenId',
]);
/** Switches the Helm between your address bar and asking the assistant (Ctrl+J, or the star button). */
export function toggleAsk(root, state) {
  if (state.helm?.kind === 'asking') {
    state.helm = state.savedHelm || { kind: 'empty' };
    state.answer = null;
    state.helmChat = null;
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
}

function isChatPage(state) {
  return state.page === 'internal' && (state.pageName === 'chat' || state.pageName === 'ai');
}

/** What the assistant page can do (it talks to Rust through the bridge). */
function chatActions(root, state) {
  const send = (m) => window.odysseyBridge?.(JSON.stringify(m));
  let captureId = 0;
  return {
    /** opts: { text, images, usePage, editFrom, regenerate } */
    send: (opts) => {
      const o = typeof opts === 'string' ? { text: opts } : opts;
      const text = o.text || '';
      state.chat = state.chat || { id: null, title: 'New chat', messages: [] };
      let msgs = state.chat.messages;
      if (o.editFrom != null) msgs = msgs.slice(0, o.editFrom);
      if (o.regenerate) { while (msgs.length && msgs[msgs.length - 1].role === 'assistant') msgs = msgs.slice(0, -1); }
      if (!o.regenerate) msgs = [...msgs, { role: 'user', text, images: o.images || [], files: (o.files || []).map((f) => f.name), time: Math.floor(Date.now() / 1000) }];
      state.chat.messages = msgs;
      state.chatStream = { text: '', steps: [], sources: [], done: false };
      state.aiOrigin = 'chat';
      // a new version or answer changes what the saved conversation looks like: reload it afterwards
      state.chatRefresh = !!(o.regenerate || o.editFrom != null);
      send({
        type: 'ai-ask', prompt: text, chat: state.chat.id,
        images: o.images || [], use_page: !!o.usePage,
        files: (o.files || []).map((f) => ({ name: f.name, data: f.data })),
        edit_from: o.editFrom ?? null,
        regenerate: !!o.regenerate,
      });
      chatUpdate(state);
    },
    stop: () => {
      send({ type: 'ai-cancel' });
      const s = state.chatStream;
      if (s && s.text) state.chat.messages = [...state.chat.messages, { role: 'assistant', text: s.text, steps: s.steps, sources: s.sources, time: Math.floor(Date.now() / 1000) }];
      state.chatStream = null;
      chatUpdate(state);
    },
    openChat: (id) => { state.chatStream = null; send({ type: 'chat-open', id }); },
    newChat: () => { state.chat = null; state.chatStream = null; chatUpdate(state); },
    deleteChat: (id) => {
      if (state.chat?.id === id) { state.chat = null; state.chatStream = null; }
      send({ type: 'chat-delete', id });
    },
    clearAll: () => { state.chat = null; state.chatStream = null; send({ type: 'chat-clear-all' }); },
    rename: (id, title) => { if (state.chat?.id === id) state.chat.title = title; send({ type: 'chat-rename', id, title }); chatUpdate(state); },
    pin: (id, pinned) => { if (state.chat?.id === id) state.chat.pinned = pinned; send({ type: 'chat-pin', id, pinned }); chatUpdate(state); },
    search: (query) => { state.chatQuery = query; if (query) send({ type: 'chat-search', query }); else { state.chatFound = null; chatUpdate(state); } },
    exportChat: (id) => send({ type: 'chat-export', id }),
    switchAlt: (at, to) => { if (state.chat?.id) send({ type: 'chat-switch-alt', id: state.chat.id, at, to }); },
    switchVariant: (index, to) => { if (state.chat?.id) send({ type: 'chat-switch-variant', id: state.chat.id, index, to }); },
    transcribe: (audio, mime) => { captureId += 1; send({ type: 'transcribe', audio, mime, request: captureId }); },
    listModels: () => send({ type: 'ai-list-models' }),
    setModel: (model) => send({ type: 'ai-set-model', model }),
    setSetting: (key, value) => {
      state.settings = { ...(state.settings || {}), [key]: value };
      saveSettings(state.settings);
      chatUpdate(state);
    },
    capture: (full) => { captureId += 1; send({ type: 'capture-page', full: !!full, request: captureId }); },
    openLink: (url) => openLink(state, url),
  };
}

/** What the page panel draws: everything except the data only other regions use. */
function panelSig(state) {
  // The assistant page updates itself in place (so typing and scrolling survive); it is built once.
  if (isChatPage(state)) return { page: 'chat' };
  const o = {};
  for (const k of Object.keys(state)) if (!PANEL_EXCLUDE.has(k)) o[k] = state[k];
  return o;
}

/** Sky options from live state, falling back to the saved settings. */
function skyOptions(state) {
  const set = state.settings || {};
  let mode = String(state.starsMode || set.stars || 'still').toLowerCase();
  let comets = !!(state.shootingStars ?? set.shootingStars ?? false);
  let cursor = set.starsFollowCursor !== false;
  // Low power (AUDIT-36): a still sky, no comets, no cursor links: nothing runs.
  // Reduce motion (AUDIT-42) does the same: a still sky, nothing runs. Auto follows the system.
  const reduced = set.reduceMotion === 'On' || (set.reduceMotion !== 'Off' && window.matchMedia('(prefers-reduced-motion: reduce)').matches);
  if ((set.lowPower === true || reduced) && mode !== 'off') { mode = 'still'; comets = false; cursor = false; }
  return { mode, comets, cursor, density: set.skyDensity || '100%', speed: set.skySpeed || 'Normal', cometRate: set.cometRate || 'Normal' };
}

function mountSky(state, add) {
  const first = state.sky && state.sky[0];
  const so = skyOptions(state);
  const key = JSON.stringify([
    so.mode, so.comets, so.cursor, state.settings?.reduceMotion || 'Auto', so.density, so.speed, so.cometRate, !!state.private, starColour(state.settings, !!state.private),
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
      density: so.density,
      speed: so.speed,
      cometRate: so.cometRate,
      starColor: starColour(state.settings, !!state.private),
      private: state.private,
    });
    skyKey = key;
  }
  window.__skyInfo = () => skyHandle?.info();
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
  if (target.classList.contains('helm-input') && target.style.display === 'none') {
    // The address field was being used: show it again, as it was
    target.style.display = 'block';
    const shown = target.parentNode && target.parentNode.querySelector('.helm-display');
    if (shown) shown.style.display = 'none';
  }
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
    // A redraw while you type is fine: renderShell puts the address field back as it was (text, caret).
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

/** The tabs of the space you are in (Home, Work, Personal); tabs without a space belong to Home. */
/** The enabled extensions you pinned (in the order they were pinned). */
function pinnedExtensions(state) {
  const ids = state.settings?.pinnedExtensions;
  if (!Array.isArray(ids) || !state.extensions) return [];
  return ids.map((id) => state.extensions.find((e) => e.id === id && e.enabled)).filter(Boolean);
}

function tabsInSpace(state) {
  if (!state.tabs) return state.tabs;
  const space = state.activeSpace ?? 0;
  return state.tabs.filter((t) => (t.space ?? 0) === space);
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
  applyInterface(state.settings, root);
  // A theme sets its own accent; without one the chosen accent swatch applies.
  if (state.settings?.accent && !(resolveTheme(state.settings) && !state.private)) {
    applyAccent(state.settings.accent);
  }
  const railCss = state.rail === 'expanded' ? 264 : 56;
  root.style.setProperty('--rail', `${railCss}px`);
  // The page windows sit right of the rail on the screen, so tell Rust the width as drawn (scaled).
  const railWidth = Math.round(railCss * interfaceScale());
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
    const tabs = tabsInSpace(state) || Array.from({ length: state.tabCount || 1 }, (_, i) => ({ id: i + 1 }));
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
      tabs: tabsInSpace(state) || [],
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
      } else if (pageName === 'chat' || pageName === 'ai') {
        renderChatPage(panel, state, chatActions(root, state));
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
          pinned: state.settings?.pinnedExtensions || [],
          onPin: (id, pin) => {
            const cur = (state.settings?.pinnedExtensions || []).filter((x) => x !== id);
            if (pin) cur.push(id);
            state.settings = { ...(state.settings || {}), pinnedExtensions: cur.slice(-24) };
            saveSettings(state.settings);
            renderShell(root, state);
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
      tabs: (tabsInSpace(state) || []).map((t) => [t.id, t.title, t.url, t.parent]),
      space: state.activeSpace ?? 0,
      activeTab: state.activeTab, newTab: state.newTab, minute: state.minute,
      opts: [state.settings?.showClock, state.settings?.clockFormat, state.settings?.showGreeting,
        state.settings?.greetingName, state.settings?.showTrailHint, state.settings?.wallpaperDim,
        state.settings?.wallpaperBlur, wallpaperVersion()],
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
    add(region('answer', state.answer, () => renderAnswerCard(state.answer, (url) => openLink(state, url), state.answer.live ? () => {
      state.chatOpenId = state.helmChat || null;
      state.helm = state.savedHelm || { kind: 'empty' };
      state.answer = null;
      state.dimPage = false;
      state.helmChat = null;
      window.odysseyBridge?.(JSON.stringify({ type: 'open', url: 'odyssey://chat' }));
      renderShell(root, state);
    } : null), { enter: true, exit: true }));
  }

  // Permission and certificate questions for the tab being shown (AUDIT-21).
  // The assistant's "allow this?" question shows whatever tab it is working in.
  const prompt = (state.prompts || []).find((p) => p.view === state.activeTab || p.kind === 'agent-confirm');
  if (prompt) {
    add(region('prompt', prompt, () => renderPromptCard(prompt, (answer) => {
      state.prompts = state.prompts.filter((p) => p !== prompt);
      if (typeof window.odysseyBridge === 'function') {
        const msgs = {
          permission: { type: 'answer-permission', request: prompt.request, allow: answer },
          certificate: { type: 'answer-certificate', request: prompt.request, proceed: answer },
          'save-login': { type: 'answer-save-login', request: prompt.request, action: answer },
          'fill-choice': { type: 'answer-fill', request: prompt.request, id: answer },
          'agent-confirm': { type: 'agent-answer', request: prompt.request, allow: !!answer },
        };
        window.odysseyBridge(JSON.stringify(msgs[prompt.kind]));
      }
      renderShell(root, state);
    }), { enter: true, exit: true }));
  }

  // Helm toolbar. The download ring follows the active download (AUDIT-05).
  const activeDl = (state.downloads || []).find((d) => d.active);
  const recentlyDone = !activeDl && state.dlDoneAt && Date.now() - state.dlDoneAt < 1200;
  // Live: the download button stays while there is something to see (running, or finished and not cleared yet).
  const shownDls = state.live ? (state.downloads || []).filter((d) => !d.dismissed) : (state.downloads || []);
  const keepBtn = state.live && shownDls.length > 0;
  const helmState = (activeDl || recentlyDone || keepBtn) && state.helm?.kind === 'address'
    ? { ...state.helm, download: activeDl ? (activeDl.progress ?? 0) : 1, downloadDone: !activeDl && !!recentlyDone, downloadCount: shownDls.length }
    : (state.helm || { kind: 'empty' });
  // The ring's progress is updated in place below (so it can animate), not by rebuilding the Helm.
  const helmSig = { ...helmState, download: helmState.download != null ? 'ring' : null, downloadDone: !!helmState.downloadDone };
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
    onSuggest: (query) => {
      if (typeof window.odysseyBridge === 'function') window.odysseyBridge(JSON.stringify({ type: 'suggest', query }));
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
    onToggleAsk: () => toggleAsk(root, state),
    onAsk: (question) => {
      state.dimPage = true;
      state.aiOrigin = 'helm';
      // Your question stays above the answer; the footer comes only when the answer is complete.
      state.answer = {
        live: true,
        question,
        text: '',
        steps: [],
        sources: [],
        done: false,
        footer: state.aiConfig?.provider === 'odysseus'
          ? 'Answered by your Odysseus server'
          : (state.aiConfig?.provider === 'cloud'
            ? 'Answered by Cloud AI'
            : (state.aiConfig?.provider === 'local' ? 'Answered by local AI' : 'AI not configured')),
      };
      // The box empties so you can follow up.
      state.helm = { kind: 'asking', question: '' };
      renderShell(root, state);
      if (typeof window.odysseyBridge === 'function') {
        window.odysseyBridge(JSON.stringify({ type: 'ai-ask', prompt: question, chat: state.helmChat || null }));
      }
    },
  })));

  // Downloads popover (Browsing board)
  if (state.downloadsOpen && shownDls.length) {
    add(region('downloads', shownDls.map((d) => ({ ...d })), () => renderDownloadsPopover(shownDls, {
      live: state.live === true,
      onClose: () => { state.downloadsOpen = false; renderShell(root, state); },
      onDismiss: (id) => { const d = state.downloads.find((x) => x.id === String(id)); if (d) d.dismissed = true; renderShell(root, state); },
      onClear: () => { for (const d of state.downloads) if (!d.active) d.dismissed = true; renderShell(root, state); },
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

  if (autoHideOn && state.live && state.settings?.showGuideBar !== false) {
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
  const pinnedExts = pinnedExtensions(state);
  add(region('notch', {
    count: state.live ? (state.page === 'web' ? state.shields ?? 0 : 0) : state.shields ?? 0,
    open: !!state.shieldsOpen,
    pinned: pinnedExts.map((e) => [e.id, e.name]),
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
      !document.documentElement.classList.contains('fixture'),
      pinnedExts
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
  notch: { hidden: false, timer: 0, hover: false, edge: false, showMs: 2500, hideAfterMs: 700, sel: '.notch, .notch-grip' },
  helm: { hidden: false, timer: 0, hover: false, edge: false, showMs: 4500, hideAfterMs: 1200, sel: '.helm, .helm-grip' },
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
    if (autoHideOn && !h.hover && !h.edge && !h.typing && !(name === 'helm' && helmBusy)) setHidden(name, true);
  }, ms);
}

/** The pointer is at the top-right corner ("top"), the bottom edge ("bottom") or neither (""): from Rust. */
export function edgeHover(zone) {
  if (!shellRoot) return;
  const want = { notch: zone === 'top', helm: zone === 'bottom' };
  for (const name of Object.keys(HIDERS)) {
    const h = HIDERS[name];
    if (want[name] === h.edge) continue;
    h.edge = want[name];
    if (!autoHideOn) continue;
    if (h.edge) { clearTimeout(h.timer); setHidden(name, false); } else scheduleHide(name, h.hideAfterMs);
  }
}

let edgeWatching = false;
function watchEdges(on) {
  if (on === edgeWatching) return;
  edgeWatching = on;
  if (typeof window.odysseyBridge === 'function') window.odysseyBridge(JSON.stringify({ type: 'edge-watch', on }));
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
        HIDERS.helm.typing = true;
        clearTimeout(HIDERS.helm.timer);
        if (autoHideOn) setHidden('helm', false);
      }
    });
    root.addEventListener('focusout', (e) => {
      if (e.target.closest && e.target.closest('.helm')) {
        HIDERS.helm.typing = false;
        if (autoHideOn) scheduleHide('helm', HIDERS.helm.hideAfterMs);
      }
    });
  }
  // A redraw can replace the address field while it has focus (no focusout then): don't stay "typing".
  const fa = document.activeElement;
  if (HIDERS.helm.typing && !(fa && fa.closest && fa.closest('.helm'))) {
    HIDERS.helm.typing = false;
    if (autoHideOn) scheduleHide('helm', HIDERS.helm.hideAfterMs);
  }
  if (want && !autoHideOn) {
    autoHideOn = true;
    watchEdges(true);
    for (const name of Object.keys(HIDERS)) {
      setHidden(name, false);
      scheduleHide(name, HIDERS[name].showMs); // visible for a moment when the page appears
    }
  } else if (!want && autoHideOn) {
    autoHideOn = false;
    watchEdges(false);
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
const OVERLAY_SELECTOR = '.helm-suggest, .notch-grip, .downloads-popover, .answer-card, .shields-popover, .find-bar, .closed-preview-card, .permission-card, .helm, .helm-grip';
let overlayObserver = null;
let lastOverlayJson = '[]';
let lastNotchHeight = 34;
let lastNotchWidth = 224;
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
    const bottom = notch.getBoundingClientRect().bottom;
    // Slid away: no hole at all (0.01 tells Rust). The line you can hover is its own tiny hole.
    const h = bottom <= 1 ? 0.01 : Math.max(1, Math.min(34, Math.round(bottom)));
    const w = Math.round(notch.getBoundingClientRect().width);
    document.documentElement.style.setProperty('--notch-w', `${w}px`);
    if (h !== lastNotchHeight || w !== lastNotchWidth) {
      lastNotchHeight = h;
      lastNotchWidth = w;
      (window.__notchHeights = window.__notchHeights || []).push(h);
      window.odysseyBridge(JSON.stringify({ type: 'set-notch-height', height: h, width: w }));
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
  panel.style.cssText = 'position:absolute;top:42px;right:12px;width:280px;border-radius:var(--r-popover);background:var(--surface);box-shadow:var(--shadow-popover);padding:16px;z-index:200;display:flex;flex-direction:column;gap:12px;';

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
