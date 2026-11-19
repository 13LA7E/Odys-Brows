// New tab: clock, date, greeting, and constellation (DESIGN.md §9, PHASES.md P3-01).

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

/** Deterministic hash for layout consistency (never jumps between visits). */
export function hash(val) {
  const s = String(val);
  let h = 0;
  for (let i = 0; i < s.length; i++) {
    h = (Math.imul(31, h) + s.charCodeAt(i)) | 0;
  }
  return Math.abs(h);
}

/** Get time-of-day greeting. */
export function getGreeting(date = new Date()) {
  const h = date.getHours();
  if (h < 12) return 'Good morning';
  if (h < 18) return 'Good afternoon';
  if (h < 22) return 'Good evening';
  return 'Good night';
}

/** Format clock time (HH:mm). */
export function formatClock(date = new Date()) {
  const h = String(date.getHours()).padStart(2, '0');
  const m = String(date.getMinutes()).padStart(2, '0');
  return `${h}:${m}`;
}

/** Format date ("Wednesday, 18 November"). */
export function formatDate(date = new Date()) {
  const weekday = date.toLocaleDateString('en-US', { weekday: 'long' });
  const day = date.getDate();
  const month = date.toLocaleDateString('en-US', { month: 'long' });
  return `${weekday}, ${day} ${month}`;
}

/**
 * Generate deterministic layout for a list of tabs.
 * Keeps clear of clock/hint block (left < 450, top < 350) and Helm (bottom < 100).
 */
export function layoutConstellation(tabs = [], activeTabId = null) {
  if (!tabs || tabs.length === 0) return { points: [], edges: [] };

  const total = tabs.length;
  const points = [];
  const edges = [];
  const idToIndex = new Map();

  // Bounding box for constellation
  const minX = 500;
  const maxX = 1140;
  const minY = 200;
  const maxY = 600;

  if (total === 1) {
    const t = tabs[0];
    return {
      points: [{
        id: t.id,
        title: t.title || 'New tab',
        x: 720,
        y: 350,
        radius: 3.2,
        isCurrent: true,
        showLabel: true,
        opacity: 0.92,
      }],
      edges: [],
    };
  }

  // Separate roots and children
  const roots = [];
  const childrenByParent = new Map();

  tabs.forEach((tab) => {
    if (tab.parent && tab.parent !== tab.id && tabs.some((p) => p.id === tab.parent)) {
      if (!childrenByParent.has(tab.parent)) childrenByParent.set(tab.parent, []);
      childrenByParent.get(tab.parent).push(tab);
    } else {
      roots.push(tab);
    }
  });

  // Calculate layout rows for root trail (serpentine meandering trail)
  const rootCount = roots.length;
  const rows = Math.max(1, Math.min(4, Math.ceil(rootCount / 7)));
  const perRow = Math.ceil(rootCount / rows);

  roots.forEach((tab, i) => {
    const row = Math.floor(i / perRow);
    const col = i % perRow;
    let progX = perRow <= 1 ? 0.5 : col / (perRow - 1);
    if (row % 2 === 1) progX = 1.0 - progX;

    const progY = rows <= 1 ? 0.5 : row / (rows - 1);

    const waveY = Math.sin(progX * Math.PI) * 35;
    const jitterX = ((hash(tab.id) % 20) - 10);
    const jitterY = (((hash(tab.id) >> 2) % 20) - 10);

    const x = Math.round(minX + progX * (maxX - minX) + jitterX);
    const y = Math.round(minY + progY * (maxY - minY) + waveY + jitterY);

    const isCurrent = tab.id === activeTabId;
    const isRecent = (total - 1 - i) < 16;
    const opacity = total > 24 ? Math.max(0.22, 0.92 - (total - 1 - i) * 0.01) : 0.92;

    idToIndex.set(tab.id, points.length);
    points.push({
      id: tab.id,
      title: tab.title || `Tab ${tab.id}`,
      x: Math.max(minX, Math.min(maxX, x)),
      y: Math.max(minY, Math.min(maxY, y)),
      radius: 2.8 + (hash(tab.id) % 9) * 0.1,
      isCurrent,
      showLabel: isCurrent || isRecent,
      opacity,
    });

    if (i > 0) {
      edges.push([idToIndex.get(roots[i - 1].id), idToIndex.get(tab.id)]);
    }
  });

  // Position children branching from parents
  childrenByParent.forEach((children, parentId) => {
    let parentIdx = idToIndex.get(parentId);
    if (parentIdx == null) parentIdx = 0;
    const parentPt = points[parentIdx];

    children.forEach((child, j) => {
      const dir = (j % 2 === 0) ? 1 : -1;
      const angle = (Math.PI / 3.5) * dir + (((hash(child.id) % 15) - 7) * (Math.PI / 180));
      const dist = 75 + (j * 25);

      const cx = Math.round(parentPt.x + Math.cos(angle) * dist);
      const cy = Math.round(parentPt.y + Math.sin(angle) * dist);

      const childIdx = points.length;
      idToIndex.set(child.id, childIdx);

      const isCurrent = child.id === activeTabId;
      const isRecent = (total - 1 - childIdx) < 16;
      const opacity = total > 24 ? Math.max(0.22, 0.92 - (total - 1 - childIdx) * 0.01) : 0.92;

      points.push({
        id: child.id,
        title: child.title || `Tab ${child.id}`,
        x: Math.max(minX, Math.min(maxX, cx)),
        y: Math.max(minY, Math.min(maxY, cy)),
        radius: 2.8 + (hash(child.id) % 9) * 0.1,
        isCurrent,
        showLabel: isCurrent || isRecent,
        opacity,
      });

      edges.push([parentIdx, childIdx]);
    });
  });

  return { points, edges };
}

/** Render New Tab greeting, clock, date, hint, and constellation. */
export function renderNewTab(container, state, onSelectTab) {
  const data = state.newTab || {};
  const tabs = state.tabs || [];
  const activeTabId = state.activeTab ?? (tabs[0]?.id || 1);

  // 1. Clock, greeting, date block (left 120, top 92)
  const header = el('div', 'newtab-header');
  header.style.cssText = 'position:absolute;left:120px;top:92px;pointer-events:none;';

  const greetingEl = el('div', null, data.greeting || getGreeting());
  greetingEl.style.cssText = 'font-size:15px;color:var(--text-muted);';

  const clockEl = el('div', null, data.clock || formatClock());
  clockEl.style.cssText = 'font-size:84px;font-weight:400;letter-spacing:-3px;line-height:1.05;color:var(--text-strong);margin-top:4px;';

  const dateEl = el('div', null, data.date || formatDate());
  dateEl.style.cssText = 'font-size:15px;color:var(--text-muted);margin-top:2px;';

  header.append(greetingEl, clockEl, dateEl);
  container.append(header);

  // 2. Hint text (left 120, top 262, width 260)
  const hintEl = el('div', 'newtab-hint', data.hint || 'Your open tabs, drawn as a trail. Select a star to jump back to it.');
  hintEl.style.cssText = 'position:absolute;left:120px;top:262px;font-size:13px;color:var(--text-muted);width:260px;line-height:1.5;pointer-events:none;';
  container.append(hintEl);

  // 3. Constellation points and edges
  let points = [];
  let edges = [];

  if (data.points && data.labels) {
    // Exact fixture data (Main board)
    points = data.points.map((pt, i) => {
      const isCurrent = i === (data.current ?? 0);
      const fixtureRadii = [14, 3, 3.6, 3, 3.3, 2.8];
      return {
        id: tabs[i]?.id ?? (i + 1),
        title: data.labels[i] || `Tab ${i + 1}`,
        x: pt[0],
        y: pt[1],
        radius: fixtureRadii[i] || 3.0,
        isCurrent,
        showLabel: true,
        opacity: 0.92,
      };
    });
    edges = data.edges || [];
  } else {
    // Deterministic generated layout from tab tree
    const generated = layoutConstellation(tabs, activeTabId);
    points = generated.points;
    edges = generated.edges;
  }

  // Constellation SVG overlay
  const constelSvg = svg('svg', {
    width: 1280,
    height: 800,
    viewBox: '0 0 1280 800',
    style: 'position:absolute;left:0;top:0;pointer-events:none;',
    'aria-hidden': 'true',
  });

  // Edges
  edges.forEach(([fromIdx, toIdx]) => {
    const p1 = points[fromIdx];
    const p2 = points[toIdx];
    if (p1 && p2) {
      const line = svg('line', {
        x1: p1.x,
        y1: p1.y,
        x2: p2.x,
        y2: p2.y,
        stroke: 'var(--white)',
        'stroke-opacity': '.18',
        'stroke-width': 1,
      });
      constelSvg.append(line);
    }
  });

  // Stars in SVG
  points.forEach((pt) => {
    if (pt.isCurrent) {
      // Active tab: ring r 14, stroke-opacity .4, accent star 20 at center
      const ring = svg('circle', {
        cx: pt.x,
        cy: pt.y,
        r: 14,
        fill: 'none',
        stroke: 'var(--accent)',
        'stroke-opacity': '.4',
      });
      constelSvg.append(ring);

      const starSvg = svg('svg', {
        x: pt.x - 10,
        y: pt.y - 10,
        width: 20,
        height: 20,
        viewBox: '0 0 24 24',
      });
      starSvg.append(svg('path', { d: STAR_PATH, fill: 'var(--accent)' }));
      constelSvg.append(starSvg);
    } else {
      // Other tabs: white dot at .92 opacity (faded for older tabs in 100+ mode)
      const dot = svg('circle', {
        cx: pt.x,
        cy: pt.y,
        r: pt.radius,
        fill: 'var(--white)',
        opacity: String(pt.opacity ?? 0.92),
      });
      constelSvg.append(dot);
    }
  });

  container.append(constelSvg);

  // 4. Interactive labels and click targets
  // Label at x + 16, y - 9 (DESIGN.md §9)
  points.forEach((pt) => {
    const isCurrent = pt.isCurrent;

    // Interactive clickable star button
    const hitBtn = el('button', 'constellation-star');
    hitBtn.setAttribute('aria-label', pt.title);
    hitBtn.style.cssText = `position:absolute;left:${pt.x - 16}px;top:${pt.y - 16}px;width:32px;height:32px;border-radius:16px;background:transparent;cursor:pointer;`;
    hitBtn.addEventListener('click', () => {
      if (onSelectTab) onSelectTab(pt.id);
    });
    container.append(hitBtn);

    // Label (if showLabel is true)
    if (pt.showLabel !== false) {
      const labelSpan = el('span', 'constellation-label', pt.title);
      labelSpan.style.cssText = `position:absolute;left:${pt.x + 16}px;top:${pt.y - 9}px;font-size:13px;color:${isCurrent ? 'var(--text)' : 'var(--text-muted)'};white-space:nowrap;cursor:pointer;opacity:${pt.opacity ?? 1};`;
      labelSpan.addEventListener('click', () => {
        if (onSelectTab) onSelectTab(pt.id);
      });
      container.append(labelSpan);
    }
  });
}
