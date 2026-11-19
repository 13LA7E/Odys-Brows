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
  loadUnpackedBtn.addEventListener('click', () => {
    const path = window.prompt('Enter the absolute folder path containing manifest.json (Manifest V3):');
    if (path && actions.onLoadUnpacked) {
      actions.onLoadUnpacked(path.trim());
    }
  });
  topActions.append(loadUnpackedBtn);
  header.append(titleCol, topActions);
  wrap.append(header);

  // Extension list or empty state
  if (!extensions || extensions.length === 0) {
    const empty = el('div', 'extensions-empty');
    empty.style.cssText =
      'padding:48px;border:1px dashed var(--w-border);border-radius:14px;text-align:center;color:var(--text-muted);';
    empty.append(
      el('div', null, 'No extensions installed yet.'),
      el('div', null, 'You can load unpacked Manifest V3 extensions or install directly from the Chrome Web Store.')
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

      // P6-03 Cost per extension: memory and CPU
      const statsRow = el('div', 'extension-stats');
      statsRow.style.cssText = 'display:flex;gap:14px;font-size:12px;color:var(--text-soft);margin-bottom:10px;';
      const memChip = el('span', null, `${ext.memory_mb.toFixed(1)} MB RAM`);
      const cpuChip = el('span', null, `${ext.cpu_pct.toFixed(1)}% CPU`);
      memChip.style.cssText = 'background:var(--w-chip);padding:2px 8px;border-radius:4px;';
      cpuChip.style.cssText = 'background:var(--w-chip);padding:2px 8px;border-radius:4px;';
      statsRow.append(memChip, cpuChip);

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

      controls.append(toggle, removeBtn);
      card.append(left, controls);
      list.append(card);
    });

    wrap.append(list);
  }

  container.append(wrap);
}
