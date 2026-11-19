// The browser chrome's layout: sky, rail area, page panel, Helm, notch (DESIGN.md §4).
// Used by fixture mode now and by the live UI from Phase 2. Everything is built with DOM
// methods (never innerHTML with data), except the fixture's sample website, which is our own
// file from design/preview.

import { createSky } from './sky.js';
import { renderCollapsedRail, renderExpandedRail } from './rail.js';

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

function helm(state) {
  const h = el('div', 'helm');
  h.setAttribute('role', 'toolbar');
  h.setAttribute('aria-label', 'Helm');
  const field = el('div', 'helm-address');
  if (state.kind === 'empty') {
    field.append(el('span', 'muted', 'Ask or enter an address'));
  } else if (state.kind === 'address' && state.address.kind === 'web') {
    field.append(el('span', 'host', state.address.host), el('span', 'muted', state.address.path));
  } else if (state.kind === 'address' && state.address.kind === 'internal') {
    field.append(el('span', 'muted', 'odyssey://'), el('span', 'host', state.address.name));
  } else if (state.kind === 'asking') {
    field.append(el('span', 'host', state.question));
  }
  h.append(field);
  return h;
}

function notch(count) {
  const n = el('div', 'notch');
  const shields = el('button', 'notch-shields');
  shields.setAttribute('aria-label', `Shields, ${count} blocked`);
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

/** Render the whole window for one state (a fixture now, live state later). */
export function renderShell(root, state) {
  root.replaceChildren();
  root.classList.toggle('private', !!state.private);
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
  if (state.rail === 'collapsed') {
    const tabs = state.tabs || Array.from({ length: state.tabCount || 1 }, (_, i) => ({ id: i + 1 }));
    renderCollapsedRail(rail, {
      tabs,
      activeTab: state.activeTab ?? 1,
      isPrivate: !!state.private,
      onSelectTab: (id) => {
        state.activeTab = id;
        renderShell(root, state);
      },
      onNewTab: () => {
        const nextId = (state.tabs ? Math.max(...state.tabs.map((t) => t.id), 0) : (state.tabCount || 1)) + 1;
        if (state.tabs) {
          state.tabs.push({ id: nextId, title: 'New tab' });
        } else {
          state.tabCount = (state.tabCount || 1) + 1;
        }
        state.activeTab = nextId;
        renderShell(root, state);
      },
      onOpenSettings: () => {
        if (typeof window.odysseyBridge === 'function') {
          window.odysseyBridge(JSON.stringify({ type: 'open', url: 'odyssey://settings' }));
        }
      },
    });
  } else if (state.rail === 'expanded') {
    renderExpandedRail(rail, {
      tabs: state.tabs || [],
      activeTab: state.activeTab ?? 1,
      closed: state.closed || [],
      isPrivate: !!state.private,
      onSelectTab: (id) => {
        state.activeTab = id;
        renderShell(root, state);
      },
      onNewTab: () => {
        const nextId = (state.tabs ? Math.max(...state.tabs.map((t) => t.id), 0) : 0) + 1;
        state.tabs.push({ id: nextId, title: 'New tab' });
        state.activeTab = nextId;
        renderShell(root, state);
      },
      onCollapse: () => {
        state.rail = 'collapsed';
        renderShell(root, state);
      },
      onOpenSettings: () => {
        if (typeof window.odysseyBridge === 'function') {
          window.odysseyBridge(JSON.stringify({ type: 'open', url: 'odyssey://settings' }));
        }
      },
    });
  }
  root.append(rail);

  if (state.page !== 'none') {
    const panel = el('section', `page page-${state.page}`);
    panel.setAttribute('aria-label', 'Page');
    if (state.page === 'web' && state.pageHtml) {
      // The fixture's sample website: our own file extracted from design/preview.
      const site = el('div', 'page-site');
      site.innerHTML = state.pageHtml;
      panel.append(site);
    }
    root.append(panel);
    if (state.dimPage && !state.pageHtml?.includes('rgba(0,0,0,.5)')) {
      root.append(el('div', 'page-dim'));
    }
  }

  root.append(helm(state.helm || { kind: 'empty' }));
  root.append(notch(state.shields ?? 0));
}
