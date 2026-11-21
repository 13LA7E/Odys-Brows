// Themes (AUDIT-35): dark colour sets for the whole browser, plus an editor for your own.
// "Default" is the approved design and adds no overrides at all, so nothing changes until you pick
// another theme. Light themes are not offered (D-025). A theme is six colours:
//   bg (behind everything), surface (cards, menus), panel (settings and other pages),
//   text, accent, star (the sky). Everything else (strong/soft/muted text, text on the accent) is
// worked out from those. The Rust side (settings.rs) checks saved themes again.

export const THEME_KEYS = ['bg', 'surface', 'panel', 'text', 'accent', 'star'];
export const THEME_LABELS = {
  bg: ['Background', 'Behind everything.'],
  surface: ['Cards and menus', 'The Helm, menus and cards.'],
  panel: ['Pages', 'Settings, History and other Constellations pages.'],
  text: ['Text', 'Main text colour.'],
  accent: ['Accent', 'Active tab, progress and focus.'],
  star: ['Stars', 'The colour of the sky.'],
};

export const DEFAULT_THEME_NAME = 'Default';

// Presets (names and moods follow Odyssey's; colours are adapted to Constellations' six).
export const THEME_PRESETS = {
  Void: { bg: '#000000', surface: '#141414', panel: '#0a0a0a', text: '#e8e8e8', accent: '#ffffff', star: '#ffffff' },
  Nebula: { bg: '#03000a', surface: '#170f3a', panel: '#0e0828', text: '#ece8ff', accent: '#7aa2ff', star: '#d9d2ff' },
  Aurora: { bg: '#000204', surface: '#0b1b18', panel: '#06110f', text: '#e4f5ee', accent: '#3fe0a0', star: '#f2fffa' },
  Lagoon: { bg: '#000000', surface: '#0a2226', panel: '#061416', text: '#e0f5f1', accent: '#2ee6c8', star: '#eafffb' },
  Ember: { bg: '#050200', surface: '#20110a', panel: '#150a06', text: '#f6e9e0', accent: '#ff8a3d', star: '#fff1e6' },
  Graphite: { bg: '#0b0c0e', surface: '#1c1f24', panel: '#14161a', text: '#e6e8eb', accent: '#9bb0c6', star: '#dfe6ee' },
};

const HEX = /^#[0-9a-fA-F]{6}$/;

function rgb(hex) {
  const n = parseInt(hex.slice(1), 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}
function toHex(c) {
  return '#' + c.map((v) => Math.max(0, Math.min(255, Math.round(v))).toString(16).padStart(2, '0')).join('');
}
export function mixHex(a, b, t) {
  const x = rgb(a);
  const y = rgb(b);
  return toHex(x.map((v, i) => v + (y[i] - v) * t));
}
export function luminance(hex) {
  const f = (v) => { v /= 255; return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4; };
  const [r, g, b] = rgb(hex).map(f);
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}
export function contrast(a, b) {
  const x = luminance(a);
  const y = luminance(b);
  return (Math.max(x, y) + 0.05) / (Math.min(x, y) + 0.05);
}

/** A sentence about the first problem with a theme, or null when it is fine to use. */
export function themeProblem(t) {
  if (!t || typeof t !== 'object') return 'That is not a theme.';
  const name = String(t.name || '').trim();
  if (!name) return 'Give the theme a name.';
  if (name.length > 30) return 'The name is too long (30 letters at most).';
  for (const k of THEME_KEYS) {
    if (typeof t[k] !== 'string' || !HEX.test(t[k])) return `${THEME_LABELS[k][0]} needs a colour like #1a2b3c.`;
  }
  if (luminance(t.bg) > 0.1 || luminance(t.panel) > 0.12 || luminance(t.surface) > 0.2) {
    return 'Constellations is dark only: pick darker colours for the background, cards and pages.';
  }
  if (contrast(t.text, t.bg) < 4.5 || contrast(t.text, t.panel) < 4.5 || contrast(t.text, t.surface) < 4.5) {
    return 'The text would be hard to read on these backgrounds. Make the text lighter or the backgrounds darker.';
  }
  if (contrast(t.accent, t.bg) < 3 || contrast(t.accent, t.panel) < 3) {
    return 'The accent is too dark to see on the background. Pick a lighter accent.';
  }
  return null;
}

export function newThemeId() {
  return 't' + Date.now().toString(36) + Math.floor(Math.random() * 1296).toString(36);
}

/** The colours of the chosen theme, or null for Default (and for anything unknown or unusable). */
export function resolveTheme(settings) {
  const name = settings?.theme || DEFAULT_THEME_NAME;
  const t = THEME_PRESETS[name] || settings?.customThemes?.[name];
  return t && !themeProblem({ name: 'x', ...t }) ? t : null;
}

const VARS = ['--bg', '--surface', '--panel-internal', '--text', '--text-strong', '--text-soft', '--text-muted', '--accent', '--on-accent', '--star'];

/** Paints colours on the document (null = back to the design). Private windows keep their own look. */
export function paintTheme(t, isPrivate) {
  const root = document.documentElement;
  if (!t || isPrivate) {
    for (const v of VARS) root.style.removeProperty(v);
    root.dataset.theme = DEFAULT_THEME_NAME;
    return;
  }
  const set = (k, v) => root.style.setProperty(k, v);
  set('--bg', t.bg);
  set('--surface', t.surface);
  set('--panel-internal', t.panel);
  set('--text', t.text);
  set('--text-strong', mixHex(t.text, '#ffffff', 0.5));
  set('--text-soft', mixHex(t.text, t.bg, 0.25));
  set('--text-muted', mixHex(t.text, t.bg, 0.5));
  set('--accent', t.accent);
  set('--on-accent', luminance(t.accent) > 0.18 ? '#000000' : '#ffffff');
  set('--star', t.star);
  root.dataset.theme = 'custom';
}

/** The star colour the sky should use (null = white). */
export function starColour(settings, isPrivate) {
  if (isPrivate) return null;
  return resolveTheme(settings)?.star || null;
}

/** Applies the saved theme (called on every render; cheap when nothing changed). */
let lastKey = '';
export function applySavedTheme(settings, isPrivate) {
  const t = resolveTheme(settings);
  const key = JSON.stringify([t, !!isPrivate]);
  if (key === lastKey) return;
  lastKey = key;
  paintTheme(t, isPrivate);
}
/** The editor changes colours without saving; forget the cache so Cancel restores the saved look. */
export function forgetAppliedTheme() { lastKey = ''; }

/** Pasted or loaded theme text -> { theme } or { error }. Never trusts the contents. */
export function parseThemeJson(text) {
  let o;
  try { o = JSON.parse(String(text)); } catch (_) { return { error: 'That is not valid theme text.' }; }
  if (!o || typeof o !== 'object' || Array.isArray(o)) return { error: 'That is not a theme.' };
  const theme = { name: String(o.name ?? '').replace(/[\u0000-\u001f]/g, '').trim().slice(0, 30) };
  for (const k of THEME_KEYS) theme[k] = typeof o[k] === 'string' ? o[k].toLowerCase() : '';
  const problem = themeProblem(theme);
  return problem ? { error: problem } : { theme };
}

export function themeToJson(t) {
  const o = { name: t.name };
  for (const k of THEME_KEYS) o[k] = t[k];
  return JSON.stringify(o, null, 2);
}

// ---- Settings UI -----------------------------------------------------------------------------

let importOpen = false;
let draft = null; // { id, name, colours..., isNew }
let message = '';
let transfer = ''; // text shown in the export / import box

/**
 * Builds the Theme rows for Settings > Appearance.
 * ctx: { el, makeRow, makeButton, settings, save(settings), rerender() }
 */
export function buildThemeRows(ctx) {
  const { el, makeRow, makeButton, settings, save, rerender } = ctx;
  const out = [];
  const current = settings.theme || DEFAULT_THEME_NAME;
  const customs = settings.customThemes || {};
  const entries = [[DEFAULT_THEME_NAME, null], ...Object.entries(THEME_PRESETS), ...Object.entries(customs)];
  const isCustom = (id) => !!customs[id];

  const choose = (id) => {
    settings.theme = id;
    forgetAppliedTheme();
    save(settings);
    message = '';
    rerender();
  };

  // --- cards
  const grid = el('div');
  grid.style.cssText = 'display:flex;flex-wrap:wrap;gap:10px;';
  for (const [id, t] of entries) {
    const selected = id === current || (!resolveTheme(settings) && id === DEFAULT_THEME_NAME);
    const card = el('button');
    card.setAttribute('aria-label', 'Theme ' + (t?.name || id));
    card.setAttribute('aria-pressed', selected ? 'true' : 'false');
    card.style.cssText = 'border:0;font:inherit;cursor:pointer;padding:8px;border-radius:12px;background:var(--w-field);color:var(--text);display:flex;flex-direction:column;gap:6px;align-items:flex-start;min-width:96px;'
      + (selected ? 'box-shadow:0 0 0 2px var(--accent);' : '');
    const sw = el('div');
    sw.style.cssText = 'display:flex;gap:0;border-radius:6px;overflow:hidden;width:80px;height:22px;';
    const cols = t ? [t.bg, t.surface, t.panel, t.accent, t.star] : null;
    if (cols) {
      for (const c of cols) { const s = el('span'); s.style.cssText = `flex:1;background:${c};`; sw.append(s); }
    } else {
      sw.style.cssText += 'background:var(--w-field);align-items:center;justify-content:center;font-size:11px;color:var(--text-muted);display:flex;';
      sw.textContent = 'Design';
    }
    const label = el('span', null, t?.name || id);
    label.style.cssText = 'font-size:13px;';
    card.append(sw, label);
    card.addEventListener('click', () => choose(id));
    grid.append(card);
  }
  out.push(makeRow('Theme', 'Dark colour sets. Default is the original look.', el('span')));
  const gridRow = el('div');
  gridRow.style.cssText = 'padding:0 0 14px 0;';
  gridRow.append(grid);
  out.push(gridRow);

  // --- actions
  const actions = el('div');
  actions.style.cssText = 'display:flex;gap:10px;flex-wrap:wrap;padding-bottom:6px;';
  const startDraft = (base, isNew) => {
    const b = base || resolveTheme(settings) || { bg: '#000000', surface: '#13161a', panel: '#0a0c0f', text: '#e8ecef', accent: '#5ad0f0', star: '#ffffff' };
    draft = { id: isNew ? newThemeId() : current, name: isNew ? 'My theme' : (b.name || 'My theme'), isNew, ...Object.fromEntries(THEME_KEYS.map((k) => [k, b[k]])) };
    message = '';
    paintTheme(draft, false);
    rerender();
  };
  actions.append(makeButton('New theme', false, () => startDraft(null, true)));
  if (isCustom(current)) {
    actions.append(makeButton('Edit', false, () => startDraft(customs[current], false)));
    actions.append(makeButton('Delete', false, () => {
      delete customs[current];
      settings.customThemes = customs;
      settings.theme = DEFAULT_THEME_NAME;
      forgetAppliedTheme();
      save(settings);
      rerender();
    }));
  } else if (THEME_PRESETS[current]) {
    actions.append(makeButton('Make a copy to edit', false, () => startDraft({ ...THEME_PRESETS[current], name: current + ' copy' }, true)));
  }
  if (resolveTheme(settings)) {
    actions.append(makeButton('Export', false, () => {
      transfer = themeToJson({ name: current, ...resolveTheme(settings), ...(customs[current] ? { name: customs[current].name } : {}) });
      message = 'Copy this text to share the theme.';
      rerender();
    }));
  }
  actions.append(makeButton('Import', false, () => { transfer = ''; message = 'Paste theme text below, then press Import now.'; draft = null; importOpen = true; forgetAppliedTheme(); applySavedTheme(settings, false); rerender(); }));
  out.push(actions);

  // --- import / export box
  if (transfer || importOpen) {
    const box = el('textarea');
    box.setAttribute('aria-label', 'Theme text');
    box.value = transfer;
    box.rows = 8;
    box.style.cssText = 'width:100%;margin-top:10px;box-sizing:border-box;border-radius:10px;border:1px solid var(--w-field);background:var(--w-field);color:var(--text);font:12.5px ui-monospace,Consolas,monospace;padding:10px;';
    out.push(box);
    if (importOpen) {
      const go = makeButton('Import now', true, () => {
        const r = parseThemeJson(box.value);
        if (r.error) { message = r.error; transfer = box.value; rerender(); return; }
        if (Object.keys(customs).length >= 12) { message = 'You already have 12 themes. Delete one first.'; transfer = box.value; rerender(); return; }
        const id = newThemeId();
        settings.customThemes = { ...customs, [id]: r.theme };
        settings.theme = id;
        importOpen = false; transfer = ''; message = '';
        forgetAppliedTheme();
        save(settings);
        rerender();
      });
      go.style.marginTop = '10px';
      out.push(go);
    }
  }

  // --- editor
  if (draft) {
    const ed = el('div');
    ed.style.cssText = 'margin-top:14px;padding:16px;border-radius:14px;background:var(--w-field);';
    const nameRow = el('div');
    nameRow.style.cssText = 'display:flex;align-items:center;gap:14px;margin-bottom:10px;';
    const nl = el('span', null, 'Name');
    nl.style.cssText = 'font-size:14px;width:150px;';
    const name = el('input');
    name.value = draft.name;
    name.maxLength = 30;
    name.setAttribute('aria-label', 'Theme name');
    name.style.cssText = 'flex:1;height:32px;border-radius:8px;border:1px solid var(--w-field);background:var(--panel-internal);color:var(--text);font:inherit;font-size:13.5px;padding:0 10px;';
    name.addEventListener('input', () => { draft.name = name.value; });
    nameRow.append(nl, name);
    ed.append(nameRow);
    const problemBox = el('div');
    problemBox.style.cssText = 'font-size:13px;color:var(--text-soft);margin-top:10px;min-height:18px;';
    const refreshProblem = () => { problemBox.textContent = themeProblem(draft) || ''; };
    for (const k of THEME_KEYS) {
      const r = el('div');
      r.style.cssText = 'display:flex;align-items:center;gap:14px;padding:6px 0;';
      const l = el('span', null, THEME_LABELS[k][0]);
      l.title = THEME_LABELS[k][1];
      l.style.cssText = 'font-size:14px;width:150px;';
      const pick = el('input');
      pick.type = 'color';
      pick.value = draft[k];
      pick.setAttribute('aria-label', THEME_LABELS[k][0] + ' colour');
      pick.style.cssText = 'width:40px;height:30px;border:0;background:none;padding:0;cursor:pointer;';
      const hex = el('input');
      hex.value = draft[k];
      hex.maxLength = 7;
      hex.setAttribute('aria-label', THEME_LABELS[k][0] + ' hex');
      hex.style.cssText = 'width:96px;height:32px;border-radius:8px;border:1px solid var(--w-field);background:var(--panel-internal);color:var(--text);font:inherit;font-size:13px;padding:0 10px;';
      const apply = (v) => {
        draft[k] = v;
        pick.value = v;
        hex.value = v;
        paintTheme(HEX.test(draft.bg) && THEME_KEYS.every((x) => HEX.test(draft[x])) ? draft : resolveTheme(settings), false);
        refreshProblem();
      };
      pick.addEventListener('input', () => apply(pick.value));
      hex.addEventListener('change', () => {
        const v = hex.value.trim().toLowerCase();
        if (HEX.test(v)) apply(v); else { hex.value = draft[k]; }
      });
      r.append(l, pick, hex);
      ed.append(r);
    }
    refreshProblem();
    ed.append(problemBox);
    const btns = el('div');
    btns.style.cssText = 'display:flex;gap:10px;margin-top:12px;';
    btns.append(makeButton('Save theme', true, () => {
      const p = themeProblem(draft);
      if (p) { message = p; rerender(); return; }
      const t = { name: draft.name.trim() };
      for (const k of THEME_KEYS) t[k] = draft[k].toLowerCase();
      if (draft.isNew && Object.keys(customs).length >= 12) { message = 'You already have 12 themes. Delete one first.'; rerender(); return; }
      settings.customThemes = { ...customs, [draft.id]: t };
      settings.theme = draft.id;
      draft = null; message = '';
      forgetAppliedTheme();
      save(settings);
      rerender();
    }));
    btns.append(makeButton('Cancel', false, () => { draft = null; message = ''; forgetAppliedTheme(); applySavedTheme(settings, false); rerender(); }));
    ed.append(btns);
    out.push(ed);
  }

  if (message) {
    const m = el('div', 'theme-message', message);
    m.style.cssText = 'font-size:13px;margin-top:10px;color:var(--text-soft);';
    out.push(m);
  }
  return out;
}

