// Trail rail (collapsed and expanded, DESIGN.md §6).
// Handles the tab tree, active state, pinned apps, new-tab ring, spaces and settings.

const SVG = 'http://www.w3.org/2000/svg';

export const STAR_PATH = 'M12 2C12.8 8.5 15.5 11.2 22 12C15.5 12.8 12.8 15.5 12 22C11.2 15.5 8.5 12.8 2 12C8.5 11.2 11.2 8.5 12 2Z';

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

function star(size = 20, fill = 'var(--text-strong)', isPrivate = false) {
  const s = svg('svg', {
    width: size, height: size, viewBox: '0 0 24 24', 'aria-hidden': 'true',
  });
  if (isPrivate) {
    s.append(svg('path', {
      d: STAR_PATH, fill: 'none', stroke: fill, 'stroke-width': '1.8', 'stroke-linejoin': 'round',
    }));
  } else {
    s.append(svg('path', { d: STAR_PATH, fill }));
  }
  return s;
}

/** Render collapsed rail (56px wide). */
export function renderCollapsedRail(railEl, options = {}) {
  const {
    tabs = [],
    activeTab = 1,
    isPrivate = false,
    onSelectTab,
    onNewTab,
    onOpenSettings,
  } = options;

  railEl.replaceChildren();

  // Tab count
  const n = tabs.length || 1;
  const y0 = 214;
  const step = 44;
  const cys = [];
  for (let i = 0; i < n; i++) cys.push(y0 + i * step);
  const plusCy = cys[cys.length - 1] + step;

  // Background SVG with trunk, new-tab ring and plus
  const bgSvg = svg('svg', {
    width: 56, height: 800, viewBox: '0 0 56 800',
    class: 'rail-bg-svg', 'aria-hidden': 'true',
  });

  // Trunk
  bgSvg.append(svg('line', {
    x1: 28, y1: cys[0], x2: 28, y2: plusCy,
    stroke: 'var(--w-trail)', 'stroke-width': 1,
  }));

  // New tab ring (dashed)
  bgSvg.append(svg('circle', {
    cx: 28, cy: plusCy, r: 10, fill: 'var(--bg)',
    stroke: 'var(--w-newtab-ring)', 'stroke-dasharray': '2.5 3',
  }));

  // Centered plus (DESIGN.md §6: fixes sketch flaw where plus was drawn as a T)
  bgSvg.append(svg('path', {
    d: `M24.5 ${plusCy} h7 M28 ${plusCy - 3.5} v7`,
    stroke: 'var(--text-soft)', 'stroke-width': '1.4', 'stroke-linecap': 'round',
  }));

  railEl.append(bgSvg);

  // Logo button at (12, 14)
  const logo = el('button', 'rail-logo');
  logo.setAttribute('aria-label', 'Odyssey');
  logo.style.cssText = 'position:absolute;left:12px;top:14px;width:32px;height:32px;border-radius:8px;display:flex;align-items:center;justify-content:center;';
  logo.append(star(20, 'var(--text-strong)', isPrivate));
  railEl.append(logo);

  // Pinned apps: Mail (top 68), Calendar (top 104), Notes (top 140)
  const pins = [
    { label: 'Mail', d: 'M4 7l8 6 8-6', extra: [svg('rect', { x: 3, y: 5, width: 18, height: 14, rx: 3 })] },
    { label: 'Calendar', d: 'M4 10h16M8 3v4M16 3v4', extra: [svg('rect', { x: 4, y: 5, width: 16, height: 15, rx: 3 })] },
    { label: 'Notes', d: 'M6 4h9l3 3v13H6zM9 11h6M9 15h6', extra: null },
  ];

  pins.forEach((p, i) => {
    const b = el('button', 'rail-pin');
    b.setAttribute('aria-label', p.label);
    b.style.cssText = `position:absolute;left:12px;top:${68 + 36 * i}px;width:32px;height:32px;border-radius:8px;display:flex;align-items:center;justify-content:center;color:var(--pin);`;
    b.append(ico(p.d, 18, p.extra));
    railEl.append(b);
  });

  // Tab buttons
  tabs.forEach((tab, i) => {
    const cy = cys[i];
    const b = el('button', 'rail-tab-btn');
    const tabNum = i + 1;
    b.setAttribute('aria-label', `Tab ${tabNum}`);
    b.style.cssText = `position:absolute;left:12px;top:${cy - 16}px;width:32px;height:32px;border-radius:16px;display:flex;align-items:center;justify-content:center;color:var(--text-soft);`;

    const isActive = (tab.id != null && tab.id === activeTab) || (tab.id == null && tabNum === activeTab);
    if (isActive) {
      const activeSvg = svg('svg', { width: 32, height: 32, viewBox: '0 0 32 32', 'aria-hidden': 'true' });
      activeSvg.append(svg('circle', {
        cx: 16, cy: 16, r: 13, fill: 'var(--bg)', stroke: 'var(--accent)', 'stroke-opacity': '.4',
      }));
      const innerStar = svg('svg', { x: 7, y: 7, width: 18, height: 18, viewBox: '0 0 24 24' });
      innerStar.append(svg('path', { d: STAR_PATH, fill: 'var(--accent)' }));
      activeSvg.append(innerStar);
      b.append(activeSvg);
    } else {
      const dot = el('span');
      dot.style.cssText = 'width:7px;height:7px;border-radius:4px;background:var(--node);display:block;';
      b.append(dot);
    }

    b.addEventListener('click', () => {
      if (onSelectTab) onSelectTab(tab.id ?? tabNum);
    });

    railEl.append(b);
  });

  // New tab ring button
  const newTabBtn = el('button', 'rail-newtab-btn');
  newTabBtn.setAttribute('aria-label', 'New tab');
  newTabBtn.style.cssText = `position:absolute;left:12px;top:${plusCy - 16}px;width:32px;height:32px;border-radius:16px;display:flex;align-items:center;justify-content:center;`;
  newTabBtn.addEventListener('click', () => {
    if (onNewTab) onNewTab();
  });
  railEl.append(newTabBtn);

  // Spaces column at left 21, top height - 122
  const spaces = el('div', 'rail-spaces');
  spaces.style.cssText = 'position:absolute;left:21px;top:calc(100% - 122px);display:flex;flex-direction:column;gap:12px;align-items:center;width:14px;';
  spaces.append(star(13, 'var(--text-strong)', isPrivate));
  const sdot1 = el('span');
  sdot1.style.cssText = 'width:5px;height:5px;border-radius:3px;background:var(--space-dot);display:block;';
  const sdot2 = el('span');
  sdot2.style.cssText = 'width:5px;height:5px;border-radius:3px;background:var(--space-dot);display:block;';
  spaces.append(sdot1, sdot2);
  railEl.append(spaces);

  // Settings button at left 12, top height - 52
  const settingsBtn = el('button', 'rail-settings-btn');
  settingsBtn.setAttribute('aria-label', 'Settings');
  settingsBtn.style.cssText = 'position:absolute;left:12px;top:calc(100% - 52px);width:32px;height:32px;border-radius:8px;display:flex;align-items:center;justify-content:center;color:var(--text-soft);';
  const settingsIcon = ico('M4 7h9M17 7h3M4 17h3M11 17h9', 18, [
    svg('circle', { cx: 15, cy: 7, r: 2 }),
    svg('circle', { cx: 9, cy: 17, r: 2 }),
  ]);
  settingsBtn.append(settingsIcon);
  settingsBtn.addEventListener('click', () => {
    if (onOpenSettings) onOpenSettings();
  });
  railEl.append(settingsBtn);
}
