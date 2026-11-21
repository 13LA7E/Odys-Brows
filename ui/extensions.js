// Extensions page and action popovers (Phase 6: P6-01, P6-02, P6-03 / DESIGN.md §10).

function el(tag, className, text) {
  const e = document.createElement(tag);
  if (className) e.className = className;
  if (text != null) e.textContent = text;
  return e;
}

function svg(tag, attrs = {}) {
  const e = document.createElementNS('http://www.w3.org/2000/svg', tag);
  for (const [k, v] of Object.entries(attrs)) e.setAttribute(k, v);
  return e;
}

export function renderExtensionsPage(container, extensions = [], actions = {}) {
  container.replaceChildren();

  const wrap = el('div', 'extensions-page');
  wrap.style.cssText = 'padding:64px 80px;max-width:840px;color:var(--text);';

  // Header
  const header = el('div', 'extensions-header');
  header.style.cssText = 'display:flex;justify-content:space-between;align-items:flex-start;margin-bottom:32px;';

  const titleCol = el('div');
  const h1 = el('h1', null, 'Extensions');
  h1.style.cssText = 'font-size:32px;font-weight:600;margin:0 0 8px 0;letter-spacing:-0.5px;color:var(--text-strong);';
  const subtitle = el('p', null, 'Add capabilities, privacy tools and custom controls to your browser.');
  subtitle.style.cssText = 'font-size:15px;color:var(--text-muted);margin:0;';
  titleCol.append(h1, subtitle);

  const topActions = el('div');
  topActions.style.cssText = 'display:flex;gap:12px;';

  const loadUnpackedBtn = el('button', 'btn-load-unpacked', 'Load unpacked');
  loadUnpackedBtn.style.cssText =
    'padding:8px 16px;background:var(--surface);border:1px solid var(--w-border);color:var(--text);border-radius:8px;font-size:13.5px;font-weight:500;cursor:pointer;';
  // Link to the Chrome Web Store: opens it in a tab of its own.
  const storeBtn = el('button', 'btn-web-store', 'Chrome Web Store');
  storeBtn.style.cssText =
    'padding:8px 16px;background:var(--surface);border:1px solid var(--w-border);color:var(--accent);border-radius:8px;font-size:13.5px;font-weight:500;cursor:pointer;';
  storeBtn.title = 'Find extensions in the Chrome Web Store';
  storeBtn.addEventListener('click', () => {
    window.odysseyBridge?.(JSON.stringify({ type: 'open', url: 'https://chromewebstore.google.com/category/extensions' }));
  });
  topActions.append(storeBtn, loadUnpackedBtn);
  header.append(titleCol, topActions);
  wrap.append(header);

  // A folder path field (window.prompt is not reliable in our UI window).
  const loadRow = el('div', 'extensions-load-row');
  loadRow.style.cssText = 'display:none;gap:10px;align-items:center;margin:-12px 0 24px 0;';
  const pathInput = el('input');
  pathInput.type = 'text';
  pathInput.placeholder = 'Folder with manifest.json (Manifest V3)';
  pathInput.style.cssText = 'flex:1;height:38px;border-radius:8px;border:1px solid var(--w-border);background:var(--w-field);color:var(--text);font:inherit;font-size:13.5px;padding:0 12px;outline:none;';
  const goBtn = el('button', null, 'Load');
  goBtn.style.cssText = 'padding:8px 16px;background:var(--surface);border:1px solid var(--w-border);color:var(--accent);border-radius:8px;font-size:13.5px;font-weight:500;cursor:pointer;';
  const submit = () => {
    const path = pathInput.value.trim();
    if (path && actions.onLoadUnpacked) actions.onLoadUnpacked(path);
  };
  goBtn.addEventListener('click', submit);
  pathInput.addEventListener('keydown', (e) => { if (e.key === 'Enter') submit(); });
  loadRow.append(pathInput, goBtn);
  loadUnpackedBtn.addEventListener('click', () => {
    loadRow.style.display = loadRow.style.display === 'none' ? 'flex' : 'none';
    if (loadRow.style.display === 'flex') pathInput.focus();
  });
  wrap.append(loadRow);

  const note = el('p', null, 'Extensions are loaded when Constellations starts. Changes here apply the next time you start it.');
  note.style.cssText = 'font-size:13px;color:var(--text-muted);margin:0 0 20px 0;';
  wrap.append(note);

  // Extension list or empty state
  if (!extensions || extensions.length === 0) {
    const empty = el('div', 'extensions-empty');
    empty.style.cssText =
      'padding:48px;border:1px dashed var(--w-border);border-radius:14px;text-align:center;color:var(--text-muted);';
    empty.append(
      el('div', null, 'No extensions installed yet.'),
      el('div', null, 'You can load unpacked Manifest V3 extensions from a folder.')
    );
    empty.lastChild.style.cssText = 'font-size:13px;margin-top:6px;opacity:0.75;';
    wrap.append(empty);
  } else {
    const list = el('div', 'extensions-list');
    list.style.cssText = 'display:flex;flex-direction:column;gap:16px;';

    extensions.forEach((ext) => {
      const card = el('div', 'extension-card');
      card.style.cssText =
        'padding:20px 24px;background:var(--surface);border:1px solid var(--w-border);border-radius:14px;display:flex;justify-content:space-between;align-items:flex-start;';

      const left = el('div');
      left.style.cssText = 'display:flex;gap:18px;align-items:flex-start;';

      // Extension puzzle icon
      const iconWrap = el('div');
      iconWrap.style.cssText =
        'width:44px;height:44px;border-radius:10px;background:var(--w-active-row);display:flex;align-items:center;justify-content:center;color:var(--accent);';
      const iconSvg = svg('svg', {
        width: 22, height: 22, viewBox: '0 0 24 24', fill: 'none',
        stroke: 'currentColor', 'stroke-width': '1.7', 'stroke-linecap': 'round',
        'stroke-linejoin': 'round', 'aria-hidden': 'true',
      });
      iconSvg.append(svg('path', {
        d: 'M4 11a2 2 0 0 1 .4-1.2l2.6-3.8A2 2 0 0 1 8.6 5H19a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H8.6a2 2 0 0 1-1.6-.8L4.4 14.2A2 2 0 0 1 4 13v-2z',
      }));
      iconWrap.append(iconSvg);

      const infoCol = el('div');
      const nameRow = el('div');
      nameRow.style.cssText = 'display:flex;align-items:baseline;gap:8px;';
      const nameEl = el('span', null, ext.name);
      nameEl.style.cssText = 'font-size:16px;font-weight:600;color:var(--text-strong);';
      const verEl = el('span', null, `v${ext.version}`);
      verEl.style.cssText = 'font-size:12.5px;color:var(--text-muted);';
      nameRow.append(nameEl, verEl);

      const descEl = el('p', null, ext.description || 'No description provided.');
      descEl.style.cssText = 'font-size:13.5px;color:var(--text-muted);margin:6px 0 12px 0;line-height:1.45;';

      // P6-03 Cost per extension: shown only when it is really measured (it is not yet, so
      // no numbers are invented).
      const statsRow = el('div', 'extension-stats');
      statsRow.style.cssText = 'display:flex;gap:14px;font-size:12px;color:var(--text-soft);margin-bottom:10px;';
      if (ext.memory_mb > 0) {
        const memChip = el('span', null, `${ext.memory_mb.toFixed(1)} MB RAM`);
        memChip.style.cssText = 'background:var(--w-chip);padding:2px 8px;border-radius:4px;';
        statsRow.append(memChip);
      }
      if (ext.cpu_pct > 0) {
        const cpuChip = el('span', null, `${ext.cpu_pct.toFixed(1)}% CPU`);
        cpuChip.style.cssText = 'background:var(--w-chip);padding:2px 8px;border-radius:4px;';
        statsRow.append(cpuChip);
      }

      // Permissions pills
      const permsRow = el('div');
      permsRow.style.cssText = 'display:flex;flex-wrap:wrap;gap:6px;';
      (ext.permissions || []).forEach((p) => {
        const pill = el('span', null, p);
        pill.style.cssText =
          'font-size:11.5px;padding:2px 7px;border-radius:4px;border:1px solid var(--w-border);color:var(--text-muted);';
        permsRow.append(pill);
      });

      infoCol.append(nameRow, descEl, statsRow, permsRow);
      left.append(iconWrap, infoCol);

      // Controls (toggle switch, remove)
      const controls = el('div');
      controls.style.cssText = 'display:flex;align-items:center;gap:16px;';

      // Toggle switch
      const toggle = el('button', 'toggle-switch');
      toggle.setAttribute('role', 'switch');
      toggle.setAttribute('aria-checked', String(ext.enabled));
      toggle.style.cssText = `width:38px;height:22px;border-radius:11px;background:${ext.enabled ? 'var(--accent)' : 'var(--w-track)'};position:relative;cursor:pointer;border:none;`;
      const knob = el('span');
      knob.style.cssText = `position:absolute;top:2px;left:${ext.enabled ? '18px' : '2px'};width:18px;height:18px;border-radius:9px;background:var(--white);transition:left 0.15s ease;`;
      toggle.append(knob);
      toggle.addEventListener('click', () => {
        const next = !ext.enabled;
        if (actions.onToggle) actions.onToggle(ext.id, next);
      });

      // Remove button
      const removeBtn = el('button', null, 'Remove');
      removeBtn.style.cssText =
        'font-size:13px;color:var(--text-muted);background:transparent;border:none;cursor:pointer;padding:4px 8px;border-radius:6px;';
      removeBtn.addEventListener('click', () => {
        if (confirm(`Remove extension "${ext.name}"?`)) {
          if (actions.onRemove) actions.onRemove(ext.id);
        }
      });

      // Pin to the notch (only an enabled extension can be pinned): the notch grows to fit it.
      const isPinned = (actions.pinned || []).includes(ext.id);
      const pinBtn = el('button', 'ext-pin', isPinned ? 'Unpin' : 'Pin');
      pinBtn.setAttribute('aria-pressed', String(isPinned));
      pinBtn.title = isPinned ? 'Remove from the top bar' : 'Show in the top bar';
      pinBtn.disabled = !ext.enabled;
      pinBtn.style.cssText = `font-size:13px;color:${isPinned ? 'var(--accent)' : 'var(--text-muted)'};background:transparent;border:none;cursor:${ext.enabled ? 'pointer' : 'default'};padding:4px 8px;border-radius:6px;${ext.enabled ? '' : 'opacity:.4;'}`;
      pinBtn.addEventListener('click', () => { if (actions.onPin) actions.onPin(ext.id, !isPinned); });

      controls.append(toggle, pinBtn, removeBtn);
      card.append(left, controls);
      list.append(card);
    });

    wrap.append(list);
  }

  container.append(wrap);
}
