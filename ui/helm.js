// Helm: floating bottom command bar (DESIGN.md §7).
// Handles address display, input classification, navigation, back/forward/reload, and keyboard flow.

import { STAR_PATH } from './rail.js';

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

function ico(d, size = 18, extra = null) {
  const s = svg('svg', {
    width: size, height: size, viewBox: '0 0 24 24', fill: 'none',
    stroke: 'currentColor', 'stroke-width': '1.6', 'stroke-linecap': 'round',
    'stroke-linejoin': 'round', 'aria-hidden': 'true',
  });
  if (d) s.append(svg('path', { d }));
  if (extra) {
    for (const child of extra) s.append(child);
  }
  return s;
}

/** Classify typed input into a URL or search query (EDGE_CASES.md §3). */
export function classifyInput(raw) {
  const trimmed = (raw || '').trim().replace(/[\r\n]+/g, ' ');
  if (!trimmed) return null;

  // Block dangerous schemes
  const lower = trimmed.toLowerCase();
  if (lower.startsWith('javascript:')) {
    return { kind: 'blocked', reason: 'javascript: URLs are not executed' };
  }
  if (lower.startsWith('data:')) {
    return { kind: 'blocked', reason: 'Top-level data: navigation is blocked' };
  }

  // Internal schemes
  if (lower.startsWith('odyssey://')) {
    const page = lower.slice('odyssey://'.length).split('/')[0];
    if (page === 'settings' || page === 'history') {
      return { kind: 'internal', page, url: `odyssey://${page}` };
    }
    return { kind: 'internal-404', page, url: trimmed };
  }
  if (lower.startsWith('chrome://') || lower.startsWith('devtools://')) {
    return { kind: 'blocked', reason: 'Internal browser URLs are mapped or blocked' };
  }

  // Windows drive path: C:\... -> file:///
  if (/^[a-zA-Z]:[/\\]/.test(trimmed)) {
    const norm = trimmed.replace(/\\/g, '/');
    return { kind: 'url', url: `file:///${encodeURI(norm)}` };
  }

  // file:/// URLs
  if (lower.startsWith('file:///')) {
    return { kind: 'url', url: trimmed };
  }

  // Standard web schemes
  if (lower.startsWith('http://') || lower.startsWith('https://')) {
    return { kind: 'url', url: trimmed };
  }

  // Localhost or IP addresses
  if (/^(localhost|127\.0\.0\.1|\[::1\])(:\d+)?(\/.*)?$/i.test(trimmed)) {
    return { kind: 'url', url: `http://${trimmed}` };
  }

  // Intranet hostnames (e.g., router.local, nas) or domains with dots and no spaces
  if (!/\s/.test(trimmed)) {
    if (/\.(local|lan|home|corp)(:\d+)?(\/.*)?$/i.test(trimmed)) {
      return { kind: 'url', url: `http://${trimmed}` };
    }
    if (/^[a-zA-Z0-9-]+(\.[a-zA-Z0-9-]+)+(:[0-9]+)?(\/.*)?$/.test(trimmed)) {
      return { kind: 'url', url: `https://${trimmed}` };
    }
  }

  // Search query fallback
  return { kind: 'search', query: trimmed, url: `https://duckduckgo.com/?q=${encodeURIComponent(trimmed)}` };
}

/** Render the Helm toolbar. */
export function renderHelm(state, handlers = {}) {
  const {
    onBack,
    onForward,
    onReload,
    onNavigate,
    onOpenDownloads,
  } = handlers;

  const h = el('div', 'helm');
  h.setAttribute('role', 'toolbar');
  h.setAttribute('aria-label', 'Helm');

  if (state.kind === 'asking') {
    // Asking mode (HelmAnswer board, DESIGN.md §7)
    h.style.padding = '0 8px 0 18px';
    h.style.gap = '12px';

    const starIcon = svg('svg', {
      width: 16, height: 16, viewBox: '0 0 24 24', 'aria-hidden': 'true',
    });
    starIcon.append(svg('path', { d: STAR_PATH, fill: 'var(--accent)' }));
    h.append(starIcon);

    const questionBox = el('div');
    questionBox.style.cssText = 'flex:1;font-size:15px;color:var(--text);display:flex;align-items:center;white-space:nowrap;overflow:hidden;';
    const qInput = el('input');
    qInput.type = 'text';
    qInput.value = state.question || '';
    qInput.placeholder = 'Ask a question...';
    qInput.style.cssText = 'width:100%;border:0;outline:0;background:transparent;color:var(--text);font:inherit;font-size:15px;padding:0;caret-color:var(--accent);';
    questionBox.append(qInput);
    h.append(questionBox);

    setTimeout(() => qInput.focus(), 0);

    const enterLabel = el('span', null, 'Enter');
    enterLabel.style.cssText = 'font-size:12px;color:var(--text-muted);';
    h.append(enterLabel);

    const askBtn = el('button');
    askBtn.setAttribute('aria-label', 'Ask');
    askBtn.style.cssText = 'border:0;background:none;color:inherit;font:inherit;padding:0;margin:0;cursor:pointer;width:36px;height:36px;border-radius:18px;background:var(--accent);color:var(--on-accent);display:flex;align-items:center;justify-content:center;';
    const askIco = svg('svg', {
      width: 18, height: 18, viewBox: '0 0 24 24', fill: 'none',
      stroke: 'currentColor', 'stroke-width': '2', 'stroke-linecap': 'round',
      'stroke-linejoin': 'round', 'aria-hidden': 'true',
    });
    askIco.append(svg('path', { d: 'M12 19V6m0 0l-5 5m5-5l5 5' }));
    askBtn.append(askIco);
    h.append(askBtn);

    qInput.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        if (qInput.value.trim() && handlers.onAsk) handlers.onAsk(qInput.value.trim());
      } else if (e.key === 'Escape') {
        handlers.onToggleAsk?.();
      }
    });

    askBtn.addEventListener('click', () => {
      if (qInput.value.trim() && handlers.onAsk) handlers.onAsk(qInput.value.trim());
    });

    return h;
  }

  // Standard frame: Back, Forward, Address field, Reload, Download ring, Spacer, Assistant
  const backBtn = el('button', 'helm-btn');
  backBtn.setAttribute('aria-label', 'Back');
  backBtn.style.cssText = 'border:0;background:none;color:inherit;font:inherit;padding:0;margin:0;cursor:pointer;width:32px;height:32px;border-radius:16px;display:flex;align-items:center;justify-content:center;color:var(--text-soft);background:transparent;';
  backBtn.append(ico('M15 6l-6 6 6 6', 18));
  backBtn.addEventListener('click', () => onBack?.());
  h.append(backBtn);

  const fwdBtn = el('button', 'helm-btn');
  fwdBtn.setAttribute('aria-label', 'Forward');
  fwdBtn.style.cssText = 'border:0;background:none;color:inherit;font:inherit;padding:0;margin:0;cursor:pointer;width:32px;height:32px;border-radius:16px;display:flex;align-items:center;justify-content:center;color:var(--text-soft);background:transparent;';
  fwdBtn.append(ico('M9 6l6 6-6 6', 18));
  fwdBtn.addEventListener('click', () => onForward?.());
  h.append(fwdBtn);

  // Address area
  const addressBox = el('div', 'helm-address');
  addressBox.style.cssText = 'flex:1;display:flex;align-items:center;gap:10px;padding:0 12px;font-size:15px;white-space:nowrap;overflow:hidden;cursor:text;';

  // Input for typing
  const inputEl = el('input');
  inputEl.type = 'text';
  inputEl.className = 'helm-input';
  inputEl.style.cssText = 'display:none;width:100%;height:100%;border:0;outline:0;background:transparent;color:var(--text);font:inherit;font-size:15px;padding:0;';
  inputEl.setAttribute('aria-label', 'Address or search');

  // Display container
  const displayBox = el('div', 'helm-display');
  displayBox.style.cssText = 'display:contents;';

  if (state.kind === 'empty') {
    const placeholder = el('span', null, 'Ask or enter an address');
    placeholder.style.color = 'var(--text-muted)';
    displayBox.append(placeholder);
  } else if (state.kind === 'address' && state.address.kind === 'web') {
    const lockIcon = svg('svg', {
      width: 14, height: 14, viewBox: '0 0 24 24', fill: 'none',
      stroke: 'var(--text-muted)', 'stroke-width': '1.6', 'stroke-linecap': 'round',
      'stroke-linejoin': 'round', 'aria-hidden': 'true',
    });
    lockIcon.append(
      svg('rect', { x: 5, y: 11, width: 14, height: 9, rx: 2 }),
      svg('path', { d: 'M8 11V8a4 4 0 0 1 8 0v3' }),
    );
    const addrSpan = el('span');
    const hostSpan = el('span', null, state.address.host);
    hostSpan.style.color = 'var(--text)';
    const pathSpan = el('span', null, state.address.path || '');
    pathSpan.style.color = 'var(--text-muted)';
    addrSpan.append(hostSpan, pathSpan);
    displayBox.append(lockIcon, addrSpan);
  } else if (state.kind === 'address' && state.address.kind === 'internal') {
    const schemeSpan = el('span', null, 'odyssey://');
    schemeSpan.style.color = 'var(--text-muted)';
    const nameSpan = el('span', null, state.address.name);
    displayBox.append(schemeSpan, nameSpan);
  }

  addressBox.append(displayBox, inputEl);

  // Focus and typing flow
  function activateInput() {
    displayBox.style.display = 'none';
    inputEl.style.display = 'block';
    if (state.kind === 'address') {
      const full = state.address.kind === 'web'
        ? `${state.address.host}${state.address.path || ''}`
        : `odyssey://${state.address.name}`;
      inputEl.value = full;
    } else {
      inputEl.value = '';
    }
    inputEl.focus();
    inputEl.select();
  }

  function deactivateInput() {
    inputEl.style.display = 'none';
    displayBox.style.display = 'contents';
  }

  addressBox.addEventListener('click', () => {
    if (inputEl.style.display === 'none') activateInput();
  });

  inputEl.addEventListener('keydown', (e) => {
    if (e.isComposing) return; // IME active
    if (e.key === 'Enter') {
      const classified = classifyInput(inputEl.value);
      if (classified && onNavigate) onNavigate(classified);
      deactivateInput();
    } else if (e.key === 'Escape') {
      deactivateInput();
    }
  });

  inputEl.addEventListener('blur', () => {
    deactivateInput();
  });

  // Global Ctrl+L shortcut
  window._focusHelm = activateInput;

  h.append(addressBox);

  // Reload button
  const reloadBtn = el('button', 'helm-btn');
  reloadBtn.setAttribute('aria-label', 'Reload');
  reloadBtn.style.cssText = 'border:0;background:none;color:inherit;font:inherit;padding:0;margin:0;cursor:pointer;width:32px;height:32px;border-radius:16px;display:flex;align-items:center;justify-content:center;color:var(--text-soft);background:transparent;';
  reloadBtn.append(ico('M20 12a8 8 0 1 1-2.3-5.6M20 4v5h-5', 17));
  reloadBtn.addEventListener('click', () => onReload?.());
  h.append(reloadBtn);

  // Download ring (if active downloading)
  if (state.download != null) {
    const pct = Math.round(state.download * 100);
    const ringBtn = el('button', state.downloadDone ? 'helm-dl-btn pulse' : 'helm-dl-btn');
    ringBtn.setAttribute('aria-label', `Downloads, ${pct} percent`);
    ringBtn.style.cssText = 'border:0;background:none;color:inherit;font:inherit;padding:0;margin:0;cursor:pointer;width:32px;height:32px;border-radius:16px;display:flex;align-items:center;justify-content:center;position:relative;color:var(--text-soft);';

    const c = 69.1;
    const d = (c * state.download).toFixed(1);
    const dashRest = (c - parseFloat(d)).toFixed(1);

    const ringSvg = svg('svg', {
      width: 32, height: 32, viewBox: '0 0 32 32',
      style: 'position:absolute;left:0;top:0;', 'aria-hidden': 'true',
    });
    ringSvg.append(svg('circle', {
      cx: 16, cy: 16, r: 11, fill: 'none', stroke: 'var(--white)',
      'stroke-opacity': '.14', 'stroke-width': 2,
    }));
    ringSvg.append(svg('circle', {
      cx: 16, cy: 16, r: 11, fill: 'none', stroke: 'var(--accent)',
      'stroke-width': 2, 'stroke-linecap': 'round', class: 'helm-dl-progress',
      'stroke-dasharray': `${d} ${dashRest}`, transform: 'rotate(-90 16 16)',
    }));
    ringBtn.append(ringSvg);

    const dlIco = svg('svg', {
      width: 14, height: 14, viewBox: '0 0 24 24', fill: 'none',
      stroke: 'currentColor', 'stroke-width': '1.6', 'stroke-linecap': 'round',
      'stroke-linejoin': 'round', 'aria-hidden': 'true',
    });
    dlIco.append(svg('path', { d: 'M12 7v8m0 0l-3-3m3 3l3-3' }));
    ringBtn.append(dlIco);

    ringBtn.addEventListener('click', () => onOpenDownloads?.());
    h.append(ringBtn);
  }

  // 4px spacer
  const spacer = el('span');
  spacer.style.width = '4px';
  h.append(spacer);

  // Assistant button (36x36, white .08 background, accent star 15; disabled until Phase 7)
  const asstBtn = el('button', 'helm-asst-btn');
  asstBtn.setAttribute('aria-label', 'Assistant');
  asstBtn.style.cssText = 'border:0;background:none;color:inherit;font:inherit;padding:0;margin:0;cursor:pointer;width:36px;height:36px;border-radius:18px;display:flex;align-items:center;justify-content:center;color:var(--text-soft);background:var(--w-assistant);';
  const asstStar = svg('svg', {
    width: 15, height: 15, viewBox: '0 0 24 24', 'aria-hidden': 'true',
  });
  asstStar.append(svg('path', { d: STAR_PATH, fill: 'var(--accent)' }));
  asstBtn.append(asstStar);
  asstBtn.addEventListener('click', () => handlers.onToggleAsk?.());
  h.append(asstBtn);

  return h;
}

/** Render downloads popover aligned with Helm right edge (DESIGN.md §7). */
export function renderDownloadsPopover(downloads = [], handlers = {}) {
  const { onControl, onOpen } = handlers;
  const pop = el('div', 'downloads-popover');
  pop.style.cssText = 'position:absolute;left:calc(var(--rail) + (100% - var(--rail) - var(--helm-w)) / 2 + var(--helm-w) - 300px);bottom:92px;width:300px;border-radius:14px;background:var(--surface);box-shadow:var(--shadow-popover);padding:6px 0;';

  downloads.forEach((item) => {
    const row = el('div');
    row.style.cssText = 'display:flex;gap:12px;align-items:flex-start;padding:12px 14px;';

    const info = el('div');
    info.style.cssText = 'flex:1;min-width:0;';
    const nameEl = el('div', null, item.name);
    nameEl.style.cssText = 'font-size:14px;color:var(--text);white-space:nowrap;overflow:hidden;text-overflow:ellipsis;';
    const detailEl = el('div', null, item.detail);
    detailEl.style.cssText = 'font-size:12.5px;color:var(--text-muted);margin-top:3px;';
    info.append(nameEl, detailEl);

    if (item.active) {
      const track = el('div');
      track.style.cssText = 'height:2px;border-radius:1px;background:var(--w-progress-track);margin-top:10px;';
      const fill = el('div');
      fill.style.cssText = `height:2px;border-radius:1px;width:${Math.round((item.progress || 0) * 100)}%;background:var(--accent);`;
      track.append(fill);
      info.append(track);

      const pauseBtn = el('button');
      pauseBtn.setAttribute('aria-label', item.paused ? 'Resume' : 'Pause');
      pauseBtn.style.cssText = 'border:0;background:none;color:inherit;font:inherit;padding:0;margin:0;cursor:pointer;width:32px;height:32px;border-radius:8px;display:flex;align-items:center;justify-content:center;color:var(--text-soft);background:transparent;';
      if (item.paused) {
        pauseBtn.append(ico('M8 5v14l11-7z', 16));
      } else {
        pauseBtn.append(ico('M9 6v12M15 6v12', 16));
      }
      pauseBtn.addEventListener('click', () => {
        const action = item.paused ? 'resume' : 'pause';
        onControl?.(item.id || item.path, action);
      });
      if (item.cancellable) {
        const cancelBtn = el('button');
        cancelBtn.setAttribute('aria-label', 'Cancel download');
        cancelBtn.style.cssText = 'border:0;background:none;font:inherit;padding:0;margin:0;cursor:pointer;width:32px;height:32px;border-radius:8px;display:flex;align-items:center;justify-content:center;color:var(--text-soft);';
        cancelBtn.append(ico('M6 6l12 12M18 6L6 18', 16));
        cancelBtn.addEventListener('click', () => onControl?.(item.id || item.path, 'cancel'));
        row.append(info, pauseBtn, cancelBtn);
      } else {
        row.append(info, pauseBtn);
      }
    } else if (item.failed) {
      row.append(info);
    } else {
      const openBtn = el('button', null, 'Open');
      openBtn.style.cssText = 'border:0;background:none;color:inherit;font:inherit;padding:0;margin:0;cursor:pointer;font-size:13px;color:var(--accent);padding:6px 2px;';
      openBtn.addEventListener('click', () => {
        onOpen?.(item.path || item.id);
      });
      row.append(info, openBtn);
    }
    pop.append(row);
  });

  return pop;
}

/** Render asking answer card (HelmAnswer board, DESIGN.md §7). */
export function renderAnswerCard(answer) {
  const card = el('div', 'answer-card');
  card.style.cssText = 'position:absolute;left:calc(var(--rail) + (100% - var(--rail) - var(--helm-w)) / 2);bottom:90px;width:560px;box-sizing:border-box;border-radius:20px;background:var(--surface);box-shadow:var(--shadow-popover);padding:22px 24px 16px;';

  const textEl = el('div', null, answer.text);
  textEl.style.cssText = 'font-size:15.5px;line-height:1.6;color:var(--text);';
  card.append(textEl);

  // Sources row
  const sourcesRow = el('div');
  sourcesRow.style.cssText = 'display:flex;align-items:center;gap:10px;margin-top:18px;';

  const constel = svg('svg', {
    width: 76, height: 30, viewBox: '0 0 76 30', 'aria-hidden': 'true',
  });
  constel.append(
    svg('line', { x1: 8, y1: 21, x2: 36, y2: 8, stroke: 'var(--white)', 'stroke-opacity': '.25' }),
    svg('line', { x1: 36, y1: 8, x2: 66, y2: 20, stroke: 'var(--white)', 'stroke-opacity': '.25' }),
    svg('circle', { cx: 8, cy: 21, r: 3, fill: 'var(--white)', opacity: '.9' }),
    svg('circle', { cx: 36, cy: 8, r: 3.4, fill: 'var(--white)', opacity: '.9' }),
    svg('circle', { cx: 66, cy: 20, r: 2.6, fill: 'var(--white)', opacity: '.9' }),
  );
  sourcesRow.append(constel);

  if (answer.sources) {
    answer.sources.forEach((src) => {
      const chip = el('span', null, src);
      chip.style.cssText = 'height:26px;padding:0 10px;border-radius:8px;background:var(--w-field);display:inline-flex;align-items:center;font-size:12.5px;color:var(--text-soft);';
      sourcesRow.append(chip);
    });
  }
  card.append(sourcesRow);

  // Footer
  const footer = el('div');
  footer.style.cssText = 'display:flex;justify-content:space-between;align-items:center;margin-top:18px;font-size:12.5px;';
  const author = el('span', null, answer.footer);
  author.style.color = 'var(--text-muted)';
  const actionBtn = el('button', null, answer.action);
  actionBtn.style.cssText = 'border:0;background:none;color:inherit;font:inherit;padding:4px 0;margin:0;cursor:pointer;color:var(--accent);font-size:13px;';
  footer.append(author, actionBtn);
  card.append(footer);

  return card;
}

// Global keyboard handler for Ctrl+L
window.addEventListener('keydown', (e) => {
  if ((e.ctrlKey || e.metaKey) && (e.key === 'l' || e.key === 'L')) {
    e.preventDefault();
    if (typeof window._focusHelm === 'function') {
      window._focusHelm();
    }
  }
});
