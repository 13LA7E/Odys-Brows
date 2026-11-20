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

/** Re-attach children to parent's parent when closing a tab (P2-04 / EDGE_CASES §2). */
export function reattachChildrenOnClose(tabs, closingTabId) {
  const closingTab = tabs.find((t) => t.id === closingTabId);
  const newParent = closingTab?.parent ?? null;
  return tabs
    .filter((t) => t.id !== closingTabId)
    .map((t) => (t.parent === closingTabId ? { ...t, parent: newParent } : t));
}

/** Render collapsed rail (56px wide). */
export function renderCollapsedRail(railEl, options = {}) {
  const {
    tabs = [],
    activeTab = 1,
    activeSpace = 0,
    isPrivate = false,
    onSelectTab,
    onNewTab,
    onOpenSettings,
    onSwitchSpace,
    onOpenPinnedSite,
    onExpand,
  } = options;

  railEl.replaceChildren();

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
  if (isPrivate) {
    bgSvg.append(svg('line', {
      x1: 28, y1: cys[0], x2: 28, y2: plusCy,
      stroke: 'var(--private-ink)', 'stroke-opacity': '.25', 'stroke-dasharray': '3 4',
    }));
    // New tab ring (dashed)
    bgSvg.append(svg('circle', {
      cx: 28, cy: plusCy, r: 10, fill: 'var(--bg)',
      stroke: 'var(--private-ink)', 'stroke-opacity': '.3', 'stroke-dasharray': '2.5 3',
    }));
    // Centered plus (DESIGN.md §6)
    bgSvg.append(svg('path', {
      d: `M24.5 ${plusCy} h7 M28 ${plusCy - 3.5} v7`,
      stroke: 'var(--text-soft)', 'stroke-width': '1.4', 'stroke-linecap': 'round',
    }));

    // Private tab nodes in SVG
    tabs.forEach((tab, i) => {
      const cy = cys[i];
      const tabNum = i + 1;
      const isActive = (tab.id != null && tab.id === activeTab) || (tab.id == null && tabNum === activeTab);
      if (isActive) {
        bgSvg.append(svg('circle', {
          cx: 28, cy, r: 13, fill: 'var(--bg)', stroke: 'var(--accent)', 'stroke-opacity': '.45',
        }));
        const innerStar = svg('svg', { x: 19, y: cy - 9, width: 18, height: 18, viewBox: '0 0 24 24' });
        innerStar.append(svg('path', {
          d: STAR_PATH, fill: 'none', stroke: 'var(--accent)', 'stroke-width': '1.8', 'stroke-linejoin': 'round',
        }));
        bgSvg.append(innerStar);
      } else {
        bgSvg.append(svg('circle', {
          cx: 28, cy, r: 4, fill: 'none', stroke: 'var(--private-node)', 'stroke-width': '1.4',
        }));
      }
    });
  } else {
    if (cys.length > 0) {
      bgSvg.append(svg('line', {
        x1: 28, y1: cys[0], x2: 28, y2: plusCy,
        stroke: 'var(--w-trail)', 'stroke-width': 1,
      }));
    }
    bgSvg.append(svg('circle', {
      cx: 28, cy: plusCy, r: 10, fill: 'var(--bg)',
      stroke: 'var(--w-newtab-ring)', 'stroke-dasharray': '2.5 3',
    }));
    bgSvg.append(svg('path', {
      d: `M24.5 ${plusCy} h7 M28 ${plusCy - 3.5} v7`,
      stroke: 'var(--text-soft)', 'stroke-width': '1.4', 'stroke-linecap': 'round',
    }));
  }

  railEl.append(bgSvg);

  // Logo button at (12, 14)
  const logo = el('button', 'rail-logo');
  logo.setAttribute('aria-label', 'Odyssey');
  logo.style.cssText = 'position:absolute;left:12px;top:14px;width:32px;height:32px;border-radius:8px;display:flex;align-items:center;justify-content:center;color:var(--text-soft);background:transparent;';
  if (isPrivate) {
    const logoSvg = svg('svg', { width: 20, height: 20, viewBox: '0 0 24 24', 'aria-hidden': 'true' });
    logoSvg.append(svg('path', {
      d: STAR_PATH, fill: 'none', stroke: 'var(--private-ink)', 'stroke-width': '1.6', 'stroke-linejoin': 'round',
    }));
    logo.append(logoSvg);
  } else {
    logo.append(star(20, 'var(--text-strong)', false));
  }
  logo.addEventListener('click', () => {
    if (onExpand) onExpand();
  });
  railEl.append(logo);

  if (isPrivate) {
    // Moon icon at left 18, top 70 replaces pinned apps
    const moon = svg('svg', {
      width: 20, height: 20, viewBox: '0 0 24 24',
      style: 'position:absolute;left:18px;top:70px;',
      fill: 'none', stroke: 'var(--accent)', 'stroke-width': '1.7',
      'stroke-linejoin': 'round', 'aria-hidden': 'true',
    });
    moon.append(svg('path', { d: 'M20 14.5A8 8 0 1 1 9.5 4a6.5 6.5 0 0 0 10.5 10.5z' }));
    railEl.append(moon);
  } else {
    // Pinned apps: Mail (top 68), Calendar (top 104), Notes (top 140)
    const pins = [
      { label: 'Mail', d: 'M4 7l8 6 8-6', extra: [svg('rect', { x: 3, y: 5, width: 18, height: 14, rx: 3 })] },
      { label: 'Calendar', d: 'M4 10h16M8 3v4M16 3v4', extra: [svg('rect', { x: 4, y: 5, width: 16, height: 15, rx: 3 })] },
      { label: 'Notes', d: 'M6 4h9l3 3v13H6zM9 11h6M9 15h6', extra: null },
    ];

    pins.forEach((p, i) => {
      const b = el('button', 'rail-pin');
      b.setAttribute('aria-label', p.label);
      b.style.cssText = `position:absolute;left:12px;top:${68 + 36 * i}px;width:32px;height:32px;border-radius:8px;display:flex;align-items:center;justify-content:center;color:var(--pin);background:transparent;cursor:pointer;`;
      b.append(ico(p.d, 18, p.extra));
      b.addEventListener('click', () => {
        if (typeof onOpenPinnedSite === 'function') onOpenPinnedSite(p.label);
      });
      railEl.append(b);
    });
  }

  // Tab buttons
  tabs.forEach((tab, i) => {
    const cy = cys[i];
    const b = el('button', 'rail-tab-btn');
    const tabNum = i + 1;
    b.setAttribute('aria-label', `Tab ${tabNum}`);
    b.style.cssText = `position:absolute;left:12px;top:${cy - 16}px;width:32px;height:32px;border-radius:16px;display:flex;align-items:center;justify-content:center;color:var(--text-soft);background:transparent;`;

    if (!isPrivate) {
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
        if (tab.sleeping) {
          dot.style.cssText = 'width:7px;height:7px;border-radius:4px;border:1px solid var(--node);background:transparent;opacity:0.4;display:block;';
          b.setAttribute('title', 'Sleeping tab (click to wake)');
        } else {
          dot.style.cssText = 'width:7px;height:7px;border-radius:4px;background:var(--node);display:block;';
        }
        b.append(dot);
      }
    }

    b.addEventListener('click', () => {
      if (onSelectTab) onSelectTab(tab.id ?? tabNum);
    });

    railEl.append(b);
  });

  // New tab ring button
  const newTabBtn = el('button', 'rail-newtab-btn');
  newTabBtn.setAttribute('aria-label', 'New tab');
  newTabBtn.style.cssText = `position:absolute;left:12px;top:${plusCy - 16}px;width:32px;height:32px;border-radius:16px;display:flex;align-items:center;justify-content:center;background:transparent;`;
  newTabBtn.addEventListener('click', () => {
    if (onNewTab) onNewTab();
  });
  railEl.append(newTabBtn);

  // Spaces column at left 21, top height - 122 (not shown in private mode)
  if (!isPrivate) {
    const spaces = el('div', 'rail-spaces');
    spaces.style.cssText = 'position:absolute;left:21px;top:calc(100% - 122px);display:flex;flex-direction:column;gap:12px;align-items:center;width:14px;';
    for (let sIdx = 0; sIdx < 3; sIdx++) {
      if (sIdx === activeSpace) {
        const s = star(13, 'var(--text-strong)', isPrivate);
        s.style.cursor = 'pointer';
        s.setAttribute('role', 'button');
        s.setAttribute('aria-label', `Space ${sIdx + 1}`);
        s.addEventListener('click', () => {
          if (typeof onSwitchSpace === 'function') onSwitchSpace(sIdx);
        });
        spaces.append(s);
      } else {
        const sdot = el('span');
        sdot.style.cssText = 'width:5px;height:5px;border-radius:3px;background:var(--space-dot);display:block;cursor:pointer;';
        sdot.setAttribute('role', 'button');
        sdot.setAttribute('aria-label', `Space ${sIdx + 1}`);
        sdot.addEventListener('click', () => {
          if (typeof onSwitchSpace === 'function') onSwitchSpace(sIdx);
        });
        spaces.append(sdot);
      }
    }
    railEl.append(spaces);
  }

  // Settings button at left 12, top height - 52
  const settingsBtn = el('button', 'rail-settings-btn');
  settingsBtn.setAttribute('aria-label', 'Settings');
  settingsBtn.style.cssText = 'position:absolute;left:12px;top:calc(100% - 52px);width:32px;height:32px;border-radius:8px;display:flex;align-items:center;justify-content:center;color:var(--text-soft);background:transparent;';
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

/** Render expanded rail (264px wide) with branches, wordmark, collapse button and pinned apps. */
export function renderExpandedRail(railEl, options = {}) {
  const {
    tabs = [],
    activeTab = 1,
    activeSpace = 0,
    closed = [],
    isPrivate = false,
    onSelectTab,
    onNewTab,
    onCollapse,
    onOpenSettings,
    onRelightClosedTab,
    onForgetClosedTab,
    onSwitchSpace,
    onOpenPinnedSite,
  } = options;

  railEl.replaceChildren();

  const W = 264;
  const y0 = 150;
  const step = 44;
  const cys = tabs.map((_, i) => y0 + i * step);

  // Determine tree levels and parent indices
  const rows = tabs.map((t) => ({
    tab: t,
    lvl: t.parent ? 1 : 0,
    active: t.id === activeTab,
  }));

  const roots = [];
  rows.forEach((r, i) => {
    if (r.lvl === 0) roots.push(i);
  });
  const lastRootIdx = roots.length > 0 ? roots[roots.length - 1] : 0;
  const lastRootCy = cys[lastRootIdx] ?? cys[0];

  const pcy = cys.length > 0 ? cys[cys.length - 1] + step : y0;

  // Background SVG with trunk, branches, new-tab ring and nodes
  const bgSvg = svg('svg', {
    width: W, height: 800, viewBox: `0 0 ${W} 800`,
    style: 'position:absolute;left:0;top:0;pointer-events:none;',
    'aria-hidden': 'true',
  });

  // Main trunk line from first row to last top-level row
  if (closed && closed.length > 0) {
    // When closed tabs follow (ClosedTabs board), trunk continues to new tab row
    bgSvg.append(svg('line', {
      x1: 28, y1: cys[0], x2: 28, y2: pcy,
      stroke: 'var(--white)', 'stroke-opacity': '.16',
    }));
  } else {
    bgSvg.append(svg('line', {
      x1: 28, y1: cys[0], x2: 28, y2: lastRootCy,
      stroke: 'var(--white)', 'stroke-opacity': '.16',
    }));
  }

  // Branch lines: for each child, branch down from parent's y to child's y
  let currentParentIdx = null;
  rows.forEach((r, i) => {
    if (r.lvl === 0) {
      currentParentIdx = i;
    } else {
      const parentIdx = r.tab.parent ? rows.findIndex((row) => row.tab.id === r.tab.parent) : currentParentIdx;
      const pIdx = parentIdx >= 0 ? parentIdx : currentParentIdx;
      const py = cys[pIdx] ?? cys[0];
      const cy = cys[i];
      bgSvg.append(svg('path', {
        d: `M28 ${py} V${cy - 8} Q28 ${cy} 36 ${cy} H46`,
        fill: 'none', stroke: 'var(--white)', 'stroke-opacity': '.16',
      }));
    }
  });

  // New tab ring (dashed)
  bgSvg.append(svg('circle', {
    cx: 28, cy: pcy, r: 10, fill: 'var(--bg)',
    stroke: 'var(--w-newtab-ring)', 'stroke-dasharray': '2.5 3',
  }));

  // Centered plus (DESIGN.md §6)
  bgSvg.append(svg('path', {
    d: `M24.5 ${pcy} h7 M28 ${pcy - 3.5} v7`,
    stroke: 'var(--text-soft)', 'stroke-width': '1.4', 'stroke-linecap': 'round',
  }));

  // Closed tabs trunk line (dotted) if closed tabs exist
  let fcys = [];
  if (closed && closed.length > 0) {
    const fy0 = pcy + step;
    fcys = closed.map((_, j) => fy0 + j * step);
    bgSvg.append(svg('line', {
      x1: 28, y1: fcys[0], x2: 28, y2: fcys[fcys.length - 1],
      stroke: 'var(--white)', 'stroke-opacity': '.12', 'stroke-dasharray': '2 5',
    }));
  }

  // Nodes for open tabs
  rows.forEach((r, i) => {
    const cx = r.lvl === 0 ? 28 : 46;
    const cy = cys[i];
    if (r.active) {
      bgSvg.append(svg('circle', {
        cx, cy, r: 12, fill: 'var(--bg)', stroke: 'var(--accent)', 'stroke-opacity': '.4',
      }));
      const innerStar = svg('svg', {
        x: cx - 8, y: cy - 8, width: 16, height: 16, viewBox: '0 0 24 24',
      });
      innerStar.append(svg('path', { d: STAR_PATH, fill: 'var(--accent)' }));
      bgSvg.append(innerStar);
    } else {
      bgSvg.append(svg('circle', {
        cx, cy, r: 3.5, fill: 'var(--node)',
      }));
    }
  });

  // Nodes for closed tabs
  if (closed && closed.length > 0) {
    const ops = [0.4, 0.27, 0.16];
    closed.forEach((c, j) => {
      const cy = fcys[j];
      if (c.hovered) {
        bgSvg.append(svg('circle', {
          cx: 28, cy, r: 11, fill: 'var(--bg)', stroke: 'var(--accent)', 'stroke-opacity': '.5',
        }));
        bgSvg.append(svg('circle', {
          cx: 28, cy, r: 4, fill: 'var(--accent)',
        }));
      } else {
        bgSvg.append(svg('circle', {
          cx: 28, cy, r: 3.5, fill: 'var(--white)', opacity: String(ops[j] ?? 0.2),
        }));
      }
    });
  }

  railEl.append(bgSvg);

  // Header: Star logo at (12, 18), wordmark at (44, 17), collapse button at (220, 12)
  const logoStar = star(20, 'var(--text-strong)', isPrivate);
  logoStar.style.cssText = 'position:absolute;left:12.0px;top:18.0px;';
  railEl.append(logoStar);

  const wordmark = el('span', null, 'odyssey');
  wordmark.style.cssText = 'position:absolute;left:44px;top:17px;font-size:17px;font-weight:600;letter-spacing:-.2px;color:var(--text-strong);';
  railEl.append(wordmark);

  const collapseBtn = el('button', 'rail-collapse-btn');
  collapseBtn.setAttribute('aria-label', 'Collapse trail');
  collapseBtn.style.cssText = 'position:absolute;left:220px;top:12px;width:32px;height:32px;border-radius:8px;display:flex;align-items:center;justify-content:center;color:var(--text-soft);background:transparent;';
  collapseBtn.append(ico('M9 4v16', 18, [svg('rect', { x: 3, y: 4, width: 18, height: 16, rx: 3.5 })]));
  collapseBtn.addEventListener('click', () => {
    if (onCollapse) onCollapse();
  });
  railEl.append(collapseBtn);

  // Pinned apps in one row at top 68px
  const pinnedList = [
    { label: 'Mail', d: 'M4 7l8 6 8-6', extra: [svg('rect', { x: 3, y: 5, width: 18, height: 14, rx: 3 })] },
    { label: 'Calendar', d: 'M4 10h16M8 3v4M16 3v4', extra: [svg('rect', { x: 4, y: 5, width: 16, height: 15, rx: 3 })] },
    { label: 'Notes', d: 'M6 4h9l3 3v13H6zM9 11h6M9 15h6', extra: null },
    { label: 'Files', d: 'M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z', extra: null },
    { label: 'Maps', d: 'M9 4l-5 2v14l5-2 6 2 5-2V4l-5 2zM9 4v14M15 6v14', extra: null },
    { label: 'Odysseus', star: true },
  ];

  pinnedList.forEach((p, i) => {
    const b = el('button', 'rail-pin');
    b.setAttribute('aria-label', p.label);
    b.style.cssText = `position:absolute;left:${12 + 38 * i}px;top:68px;width:32px;height:32px;border-radius:8px;display:flex;align-items:center;justify-content:center;color:var(--pin);background:transparent;cursor:pointer;`;
    if (p.star) {
      b.append(star(16, 'var(--accent)'));
    } else {
      b.append(ico(p.d, 18, p.extra));
    }
    b.addEventListener('click', () => {
      if (typeof onOpenPinnedSite === 'function') onOpenPinnedSite(p.label);
    });
    railEl.append(b);
  });

  // Tab rows
  rows.forEach((r, i) => {
    const cx = r.lvl === 0 ? 28 : 46;
    const cy = cys[i];
    const b = el('button', 'rail-tab-row');
    const isSleeping = Boolean(r.tab.sleeping && !r.active);
    const bg = r.active
      ? 'background:var(--w-active-row);color:var(--text);'
      : isSleeping
        ? 'color:var(--text-muted);opacity:0.6;'
        : 'color:var(--text-soft);';
    b.style.cssText = `position:absolute;left:8px;top:${cy - 18}px;width:248px;height:36px;border-radius:8px;text-align:left;padding-left:${cx + 10}px;font-size:14px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;box-sizing:border-box;${bg}`;
    b.textContent = r.tab.title || `Tab ${r.tab.id}`;
    if (isSleeping) {
      b.setAttribute('title', 'Sleeping tab (click to wake)');
    }
    b.addEventListener('click', () => {
      if (onSelectTab) onSelectTab(r.tab.id);
    });
    railEl.append(b);
  });

  // New tab row
  const newTabRow = el('button', 'rail-newtab-row');
  newTabRow.style.cssText = `position:absolute;left:8px;top:${pcy - 18}px;width:248px;height:36px;border-radius:8px;text-align:left;padding-left:40px;font-size:14px;color:var(--text-muted);box-sizing:border-box;`;
  newTabRow.textContent = 'New tab';
  newTabRow.addEventListener('click', () => {
    if (onNewTab) onNewTab();
  });
  railEl.append(newTabRow);

  // Closed tabs if present
  if (closed && closed.length > 0) {
    const closedLabel = el('span', 'rail-closed-label', 'Closed earlier');
    closedLabel.style.cssText = `position:absolute;left:48px;top:${pcy + step / 2 - 22 + 6}px;font-size:12.5px;color:var(--text-muted);`;
    railEl.append(closedLabel);

    closed.forEach((c, j) => {
      const cy = fcys[j];
      const b = el('button', 'rail-closed-row');
      const bg = c.hovered ? 'background:var(--w-hover-row);color:var(--text);' : 'color:var(--node-faded-text);';
      b.style.cssText = `position:absolute;left:8px;top:${cy - 18}px;width:248px;height:36px;border-radius:8px;text-align:left;padding-left:38px;font-size:14px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;box-sizing:border-box;${bg}`;
      b.textContent = c.title;
      railEl.append(b);

      // Hovered preview card beside rail
      if (c.hovered) {
        const card = el('div', 'closed-preview-card');
        card.style.cssText = `position:absolute;left:276px;top:${cy - 76}px;width:296px;border-radius:14px;background:var(--surface);box-shadow:var(--shadow-popover);overflow:hidden;z-index:10;`;

        // Thumbnail
        const thumb = el('div', 'closed-thumb');
        thumb.style.cssText = 'height:112px;border-radius:14px 14px 0 0;background:#dfe3e6;position:relative;overflow:hidden;';
        const b1 = el('div');
        b1.style.cssText = 'position:absolute;left:18px;top:18px;width:96px;height:10px;border-radius:5px;background:#b9c0c6;';
        const b2 = el('div');
        b2.style.cssText = 'position:absolute;left:18px;top:42px;width:240px;height:8px;border-radius:4px;background:#cbd1d6;';
        const b3 = el('div');
        b3.style.cssText = 'position:absolute;left:18px;top:58px;width:210px;height:8px;border-radius:4px;background:#cbd1d6;';
        const b4 = el('div');
        b4.style.cssText = 'position:absolute;left:18px;top:74px;width:150px;height:8px;border-radius:4px;background:#cbd1d6;';
        thumb.append(b1, b2, b3, b4);
        card.append(thumb);

        // Body
        const body = el('div', 'closed-body');
        body.style.cssText = 'padding:14px 16px 12px;';
        const titleEl = el('div', null, c.title);
        titleEl.style.cssText = 'font-size:15px;font-weight:500;color:var(--text);';
        const hostEl = el('div', null, c.host || 'tides.example');
        hostEl.style.cssText = 'font-size:12.5px;color:var(--text-muted);margin-top:3px;';
        const timeEl = el('div', null, c.closed || 'Closed 12 minutes ago');
        timeEl.style.cssText = 'font-size:12.5px;color:var(--text-muted);margin-top:10px;';

        const btns = el('div');
        btns.style.cssText = 'display:flex;gap:18px;margin-top:12px;';
        const relightBtn = el('button', null, 'Relight');
        relightBtn.style.cssText = 'color:var(--accent);font-size:13.5px;font-weight:500;cursor:pointer;';
        relightBtn.addEventListener('click', (e) => {
          e.stopPropagation();
          if (typeof onRelightClosedTab === 'function') {
            onRelightClosedTab(c.id);
          } else if (typeof window.odysseyBridge === 'function') {
            window.odysseyBridge(JSON.stringify({ type: 'relight-closed-tab', id: c.id }));
          }
        });

        const forgetBtn = el('button', null, 'Forget');
        forgetBtn.style.cssText = 'color:var(--text-soft);font-size:13.5px;cursor:pointer;';
        forgetBtn.addEventListener('click', (e) => {
          e.stopPropagation();
          if (typeof onForgetClosedTab === 'function') {
            onForgetClosedTab(c.id);
          } else if (typeof window.odysseyBridge === 'function') {
            window.odysseyBridge(JSON.stringify({ type: 'forget-closed-tab', id: c.id }));
          }
        });
        btns.append(relightBtn, forgetBtn);

        body.append(titleEl, hostEl, timeEl, btns);
        card.append(body);
        railEl.append(card);
      }
    });
  }

  // Bottom: Home + spaces at left 20, top calc(100% - 40px)
  const spaceNames = ['Home', 'Work', 'Personal'];
  const bottom = el('div', 'rail-bottom');
  bottom.style.cssText = 'position:absolute;left:20px;top:calc(100% - 40px);display:flex;align-items:center;gap:10px;font-size:14px;color:var(--text-soft);';
  if (activeSpace === 0) {
    const s = star(13, 'var(--text-strong)', isPrivate);
    s.style.cursor = 'pointer';
    s.setAttribute('role', 'button');
    s.setAttribute('aria-label', 'Space 1 (active)');
    s.addEventListener('click', () => {
      if (typeof onSwitchSpace === 'function') onSwitchSpace(0);
    });
    const nameEl = el('span', null, 'Home');
    nameEl.style.cursor = 'pointer';
    nameEl.addEventListener('click', () => {
      if (typeof onSwitchSpace === 'function') onSwitchSpace(0);
    });
    bottom.append(s, nameEl);

    const sdot1 = el('span');
    sdot1.style.cssText = 'width:5px;height:5px;border-radius:3px;background:var(--space-dot);margin-left:2px;display:block;cursor:pointer;';
    sdot1.setAttribute('role', 'button');
    sdot1.setAttribute('aria-label', 'Space 2');
    sdot1.addEventListener('click', () => {
      if (typeof onSwitchSpace === 'function') onSwitchSpace(1);
    });

    const sdot2 = el('span');
    sdot2.style.cssText = 'width:5px;height:5px;border-radius:3px;background:var(--space-dot);display:block;cursor:pointer;';
    sdot2.setAttribute('role', 'button');
    sdot2.setAttribute('aria-label', 'Space 3');
    sdot2.addEventListener('click', () => {
      if (typeof onSwitchSpace === 'function') onSwitchSpace(2);
    });

    bottom.append(sdot1, sdot2);
  } else {
    for (let sIdx = 0; sIdx < 3; sIdx++) {
      if (sIdx === activeSpace) {
        const s = star(13, 'var(--text-strong)', isPrivate);
        s.style.cursor = 'pointer';
        s.setAttribute('role', 'button');
        s.setAttribute('aria-label', `Space ${sIdx + 1} (active)`);
        s.addEventListener('click', () => {
          if (typeof onSwitchSpace === 'function') onSwitchSpace(sIdx);
        });
        const nameEl = el('span', null, spaceNames[sIdx] || `Space ${sIdx + 1}`);
        nameEl.style.cursor = 'pointer';
        nameEl.addEventListener('click', () => {
          if (typeof onSwitchSpace === 'function') onSwitchSpace(sIdx);
        });
        bottom.append(s, nameEl);
      } else {
        const sdot = el('span');
        sdot.style.cssText = 'width:5px;height:5px;border-radius:3px;background:var(--space-dot);display:block;cursor:pointer;';
        sdot.setAttribute('role', 'button');
        sdot.setAttribute('aria-label', `Space ${sIdx + 1}`);
        sdot.addEventListener('click', () => {
          if (typeof onSwitchSpace === 'function') onSwitchSpace(sIdx);
        });
        bottom.append(sdot);
      }
    }
  }
  railEl.append(bottom);

  // Settings button at left 220, top calc(100% - 52px)
  const settingsBtn = el('button', 'rail-settings-btn');
  settingsBtn.setAttribute('aria-label', 'Settings');
  settingsBtn.style.cssText = 'position:absolute;left:220px;top:calc(100% - 52px);width:32px;height:32px;border-radius:8px;display:flex;align-items:center;justify-content:center;color:var(--text-soft);';
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
