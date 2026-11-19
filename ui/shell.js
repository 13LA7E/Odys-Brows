// The browser chrome's layout: sky, rail area, page panel, Helm, notch (DESIGN.md §4).
// Used by fixture mode now and by the live UI from Phase 2. Everything is built with DOM
// methods (never innerHTML with data), except the fixture's sample website, which is our own
// file from design/preview.

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

/** The still sky from a star list (DESIGN.md §5). P2-01 replaces this with the canvas sky. */
function sky(stars) {
  const s = svg('svg', { class: 'sky', width: 1280, height: 800, viewBox: '0 0 1280 800', 'aria-hidden': 'true' });
  for (const st of stars) {
    s.append(st.ring
      ? svg('circle', { cx: st.x, cy: st.y, r: st.r, fill: 'none', stroke: st.stroke, 'stroke-opacity': st.o, 'stroke-width': st.w })
      : svg('circle', { cx: st.x, cy: st.y, r: st.r, fill: st.fill || '#fff', opacity: st.o }));
  }
  return s;
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
  const shields = el('button', 'notch-shields', String(count));
  shields.setAttribute('aria-label', `Shields, ${count} blocked`);
  n.append(shields);
  for (const label of ['Minimize', 'Maximize', 'Close']) {
    const b = el('button', 'notch-button');
    b.setAttribute('aria-label', label);
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

  root.append(sky(state.sky || []));
  const rail = el('nav', `rail rail-${state.rail}`);
  rail.setAttribute('aria-label', 'Trail');
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
    if (state.dimPage) root.append(el('div', 'page-dim'));
  }

  root.append(helm(state.helm || { kind: 'empty' }));
  root.append(notch(state.shields ?? 0));
}
