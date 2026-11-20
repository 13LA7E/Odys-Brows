// Page basics: find in page, zoom, print, DevTools, error & crash states (DESIGN.md §10, PHASES.md P4-06, EDGE_CASES §4).

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

const STAR_PATH = 'M12 2C12.8 8.5 15.5 11.2 22 12C15.5 12.8 12.8 15.5 12 22C11.2 15.5 8.5 12.8 2 12C8.5 11.2 11.2 8.5 12 2Z';

let currentZoomFactor = 1.0;
let findBarEl = null;

/** Render floating Find-in-page bar (DESIGN.md §10 quiet style). */
export function toggleFindBar(root, activeViewId) {
  if (findBarEl && findBarEl.isConnected) {
    closeFindBar(activeViewId);
    return;
  }

  findBarEl = el('div', 'find-bar');
  findBarEl.style.cssText = 'position:absolute;top:18px;right:140px;width:320px;height:42px;border-radius:var(--r-track);background:var(--surface);box-shadow:var(--shadow-popover);display:flex;align-items:center;padding:0 10px;gap:8px;z-index:30;box-sizing:border-box;font-family:var(--font);';

  const searchIcon = svg('svg', {
    width: '16',
    height: '16',
    viewBox: '0 0 24 24',
    fill: 'none',
    stroke: 'var(--text-muted)',
    'stroke-width': '1.6',
    'stroke-linecap': 'round',
    'stroke-linejoin': 'round',
    'aria-hidden': 'true',
  });
  searchIcon.append(
    svg('circle', { cx: '11', cy: '11', r: '6' }),
    svg('path', { d: 'M16 16l4 4' }),
  );

  const input = el('input', 'find-input');
  input.type = 'text';
  input.placeholder = 'Find in page';
  input.style.cssText = 'border:0;background:none;outline:none;font:inherit;font-size:13.5px;color:var(--text);flex:1;min-width:0;padding:0;margin:0;';

  const sendFind = (forward, next) => {
    const text = input.value.trim();
    if (!text) return;
    if (typeof window.odysseyBridge === 'function') {
      window.odysseyBridge(JSON.stringify({
        type: 'find-in-page',
        view: activeViewId || 1,
        text,
        forward,
        match_case: false,
        next,
      }));
    }
  };

  input.addEventListener('input', () => sendFind(true, false));
  input.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      sendFind(!e.shiftKey, true);
    } else if (e.key === 'Escape') {
      e.preventDefault();
      closeFindBar(activeViewId);
    }
  });

  const prevBtn = el('button', null);
  prevBtn.setAttribute('aria-label', 'Previous match');
  prevBtn.style.cssText = 'width:24px;height:24px;border-radius:4px;display:flex;align-items:center;justify-content:center;color:var(--text-soft);cursor:pointer;';
  const prevSvg = svg('svg', {
    width: '14',
    height: '14',
    viewBox: '0 0 24 24',
    fill: 'none',
    stroke: 'currentColor',
    'stroke-width': '1.6',
    'stroke-linecap': 'round',
    'stroke-linejoin': 'round',
  });
  prevSvg.append(svg('path', { d: 'M18 15l-6-6-6 6' }));
  prevBtn.append(prevSvg);
  prevBtn.addEventListener('click', () => sendFind(false, true));

  const nextBtn = el('button', null);
  nextBtn.setAttribute('aria-label', 'Next match');
  nextBtn.style.cssText = 'width:24px;height:24px;border-radius:4px;display:flex;align-items:center;justify-content:center;color:var(--text-soft);cursor:pointer;';
  const nextSvg = svg('svg', {
    width: '14',
    height: '14',
    viewBox: '0 0 24 24',
    fill: 'none',
    stroke: 'currentColor',
    'stroke-width': '1.6',
    'stroke-linecap': 'round',
    'stroke-linejoin': 'round',
  });
  nextSvg.append(svg('path', { d: 'M6 9l6 6 6-6' }));
  nextBtn.append(nextSvg);
  nextBtn.addEventListener('click', () => sendFind(true, true));

  const closeBtn = el('button', null);
  closeBtn.setAttribute('aria-label', 'Close find bar');
  closeBtn.style.cssText = 'width:24px;height:24px;border-radius:4px;display:flex;align-items:center;justify-content:center;color:var(--text-soft);cursor:pointer;';
  const closeSvg = svg('svg', {
    width: '14',
    height: '14',
    viewBox: '0 0 24 24',
    fill: 'none',
    stroke: 'currentColor',
    'stroke-width': '1.6',
    'stroke-linecap': 'round',
    'stroke-linejoin': 'round',
  });
  closeSvg.append(svg('path', { d: 'M6 6l12 12M18 6L6 18' }));
  closeBtn.append(closeSvg);
  closeBtn.addEventListener('click', () => closeFindBar(activeViewId));

  findBarEl.append(searchIcon, input, prevBtn, nextBtn, closeBtn);
  root.append(findBarEl);
  input.focus();
}

export function closeFindBar(activeViewId) {
  if (findBarEl && findBarEl.isConnected) {
    findBarEl.remove();
    findBarEl = null;
    if (typeof window.odysseyBridge === 'function') {
      window.odysseyBridge(JSON.stringify({
        type: 'stop-finding',
        view: activeViewId || 1,
        clear_selection: true,
      }));
    }
  }
}

/** Zoom helpers */
export function zoomIn(activeViewId) {
  currentZoomFactor = Math.min(3.0, currentZoomFactor + 0.1);
  sendZoom(activeViewId, currentZoomFactor);
}

export function zoomOut(activeViewId) {
  currentZoomFactor = Math.max(0.3, currentZoomFactor - 0.1);
  sendZoom(activeViewId, currentZoomFactor);
}

export function zoomReset(activeViewId) {
  currentZoomFactor = 1.0;
  sendZoom(activeViewId, 1.0);
}

function sendZoom(activeViewId, factor) {
  if (typeof window.odysseyBridge === 'function') {
    window.odysseyBridge(JSON.stringify({
      type: 'set-zoom',
      view: activeViewId || 1,
      factor,
    }));
  }
}

/** Print helper */
export function printPage(activeViewId) {
  if (typeof window.odysseyBridge === 'function') {
    window.odysseyBridge(JSON.stringify({
      type: 'print-page',
      view: activeViewId || 1,
    }));
  }
}

/** DevTools helper */
export function openDevTools(activeViewId) {
  if (typeof window.odysseyBridge === 'function') {
    window.odysseyBridge(JSON.stringify({
      type: 'open-dev-tools',
      view: activeViewId || 1,
    }));
  }
}

/** Render a calm, quiet error or crash page inside the page panel (DESIGN.md §10, EDGE_CASES §4). */
export function renderErrorPage(container, info = {}, onReload = null) {
  container.replaceChildren();

  const wrap = el('div', 'page-error-container');
  wrap.style.cssText = 'position:absolute;inset:0;background:var(--panel-internal);display:flex;flex-direction:column;align-items:center;justify-content:center;padding:40px;box-sizing:border-box;font-family:var(--font);';

  const starSvg = svg('svg', {
    width: '28',
    height: '28',
    viewBox: '0 0 24 24',
    style: 'margin-bottom:20px;',
    'aria-hidden': 'true',
  });
  starSvg.append(svg('path', { d: STAR_PATH, fill: 'var(--text-muted)' }));

  const heading = el('div', null, info.title || 'The page could not be reached');
  heading.style.cssText = 'font-size:24px;font-weight:600;letter-spacing:-.4px;color:var(--text-strong);margin-bottom:10px;text-align:center;';

  const message = el('div', null, info.message || 'Check your connection or try loading the page again.');
  message.style.cssText = 'font-size:15px;line-height:1.6;color:var(--text-muted);margin-bottom:24px;text-align:center;max-width:440px;';

  const btnRow = el('div');
  btnRow.style.cssText = 'display:flex;gap:14px;align-items:center;';

  const reloadBtn = el('button', null, 'Reload');
  reloadBtn.style.cssText = 'border:0;background:var(--w-field);color:var(--accent);font:inherit;font-size:13.5px;font-weight:500;padding:8px 20px;border-radius:8px;cursor:pointer;';
  reloadBtn.addEventListener('click', () => {
    if (typeof onReload === 'function') {
      onReload();
    } else if (typeof window.odysseyBridge === 'function') {
      window.odysseyBridge(JSON.stringify({ type: 'reload', view: info.view || 1 }));
    }
  });

  btnRow.append(reloadBtn);
  wrap.append(starSvg, heading, message, btnRow);
  container.append(wrap);
}

/** Put the caret in the Helm with the address selected (Ctrl+L, F6). */
export function focusHelm() {
  const box = document.querySelector('.helm-address');
  if (box) box.click();
}

/** Register global keyboard shortcuts across chrome and page basics. */
export function initPageBasicsShortcuts(getRoot, getActiveViewId) {
  window.addEventListener('keydown', (e) => {
    const isCtrl = e.ctrlKey || e.metaKey;
    const viewId = typeof getActiveViewId === 'function' ? getActiveViewId() : 1;
    const root = typeof getRoot === 'function' ? getRoot() : document.getElementById('app');

    if (isCtrl && !e.altKey && !e.shiftKey && (e.key === 't' || e.key === 'T')) {
      e.preventDefault();
      if (typeof window.odysseyBridge === 'function') {
        window.odysseyBridge(JSON.stringify({ type: 'open', url: '' }));
      }
    } else if (isCtrl && !e.altKey && e.shiftKey && (e.key === 't' || e.key === 'T')) {
      e.preventDefault();
      if (typeof window.odysseyBridge === 'function') {
        window.odysseyBridge(JSON.stringify({ type: 'relight-closed-tab' }));
      }
    } else if (isCtrl && !e.altKey && (e.key === 'w' || e.key === 'W')) {
      e.preventDefault();
      if (typeof window.odysseyBridge === 'function') {
        window.odysseyBridge(JSON.stringify({ type: 'close', view: viewId }));
      }
    } else if ((isCtrl && !e.altKey && (e.key === 'l' || e.key === 'L')) || e.key === 'F6') {
      e.preventDefault();
      focusHelm();
    } else if (isCtrl && !e.altKey && e.shiftKey && (e.key === 'n' || e.key === 'N')) {
      e.preventDefault();
      if (typeof window.odysseyBridge === 'function') {
        window.odysseyBridge(JSON.stringify({ type: 'open-private-window' }));
      }
    } else if ((isCtrl && !e.altKey && (e.key === 'r' || e.key === 'R')) || e.key === 'F5') {
      e.preventDefault();
      if (typeof window.odysseyBridge === 'function') {
        window.odysseyBridge(JSON.stringify({ type: 'reload', view: viewId }));
      }
    } else if (isCtrl && !e.altKey && (e.key === 'h' || e.key === 'H')) {
      e.preventDefault();
      if (typeof window.odysseyBridge === 'function') {
        window.odysseyBridge(JSON.stringify({ type: 'open', url: 'odyssey://history' }));
      }
    } else if (isCtrl && !e.altKey && e.key === ',') {
      e.preventDefault();
      if (typeof window.odysseyBridge === 'function') {
        window.odysseyBridge(JSON.stringify({ type: 'open', url: 'odyssey://settings' }));
      }
    } else if (isCtrl && !e.altKey && e.key === 'Tab') {
      e.preventDefault();
      if (typeof window.odysseyBridge === 'function') {
        window.odysseyBridge(JSON.stringify({ type: e.shiftKey ? 'prev-tab' : 'next-tab' }));
      }
    } else if (e.altKey && !isCtrl && e.key === 'ArrowLeft') {
      e.preventDefault();
      if (typeof window.odysseyBridge === 'function') {
        window.odysseyBridge(JSON.stringify({ type: 'back', view: viewId }));
      }
    } else if (e.altKey && !isCtrl && e.key === 'ArrowRight') {
      e.preventDefault();
      if (typeof window.odysseyBridge === 'function') {
        window.odysseyBridge(JSON.stringify({ type: 'forward', view: viewId }));
      }
    } else if (isCtrl && (e.key === 'f' || e.key === 'F')) {
      e.preventDefault();
      if (root) toggleFindBar(root, viewId);
    } else if (isCtrl && (e.key === '=' || e.key === '+')) {
      e.preventDefault();
      zoomIn(viewId);
    } else if (isCtrl && (e.key === '-' || e.key === '_')) {
      e.preventDefault();
      zoomOut(viewId);
    } else if (isCtrl && e.key === '0') {
      e.preventDefault();
      zoomReset(viewId);
    } else if (isCtrl && (e.key === 'p' || e.key === 'P')) {
      e.preventDefault();
      printPage(viewId);
    } else if (e.key === 'F12' || (isCtrl && e.shiftKey && (e.key === 'I' || e.key === 'i'))) {
      e.preventDefault();
      openDevTools(viewId);
    }
  });
}
