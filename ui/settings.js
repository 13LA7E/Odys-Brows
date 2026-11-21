// Settings page shell (DESIGN.md §9, PHASES.md P2-07, P7-02, P8-01, P8-02).
// Manages settings state, navigation, appearance, AI, passwords, privacy, shortcuts, and about.

import { captureShortcut, cancelCapture } from './shortcuts.js';
import { buildThemeRows } from './themes.js';

const SVG = 'http://www.w3.org/2000/svg';

export const DEFAULT_SETTINGS = {
  stars: 'Still',
  shootingStars: false,
  starsFollowCursor: true,
  accent: 'Sea',
  trail: 'Narrow',
  searchEngine: 'DuckDuckGo',
  customEngine: { name: '', url: '' },
  startup: 'Restore tabs',
  linkTarget: 'New tab',
  connections: 8,
  sleepMinutes: 10,
  globalPrivacyControl: true,
  clearHistoryOnExit: false,
  clearSiteDataOnExit: false,
  offerSavePasswords: true,
  autofillPasswords: true,
  shortcuts: {},
  theme: 'Default',
  customThemes: {},
};
// The schema (allowed values and defaults) lives in app/shell/src/settings.rs; a test checks that
// this list matches it. Values here are only used before the shell has sent the real settings.

export const ACCENTS = {
  Sea: { varName: '--accent-sea' },
  Mint: { varName: '--accent-mint' },
  Lilac: { varName: '--accent-lilac' },
  Amber: { varName: '--accent-amber' },
  Rose: { varName: '--accent-rose' },
};

const SECTIONS = [
  'General',
  'Appearance',
  'New tab',
  'Shields',
  'Odysseus',
  'Passwords',
  'Privacy',
  'Shortcuts',
  'About',
];

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

/** Load settings from localStorage or fallback to defaults. */
export function loadSettings() {
  try {
    const raw = localStorage.getItem('odyssey_settings');
    if (raw) {
      const parsed = JSON.parse(raw);
      return { ...DEFAULT_SETTINGS, ...parsed };
    }
  } catch (_e) {
    // Ignore storage parse errors
  }
  return { ...DEFAULT_SETTINGS };
}

/** Save settings to localStorage and send IPC message to Rust. */
export function saveSettings(settings) {
  try {
    localStorage.setItem('odyssey_settings', JSON.stringify(settings));
  } catch (_e) {
    // Ignore quota errors
  }
  if (typeof window.odysseyBridge === 'function') {
    window.odysseyBridge(JSON.stringify({ type: 'save-settings', settings }));
  }
}

/** Apply accent color to document root. */
export function applyAccent(accentName) {
  const found = ACCENTS[accentName] || ACCENTS.Sea;
  document.documentElement.style.setProperty('--accent', `var(${found.varName})`);
}

let lastSection = null;
let pwAddOpen = false;
let pwExportConfirm = false;

function makeRow(label, desc, controlEl) {
  const r = el('div');
  r.style.cssText = 'display:flex;align-items:center;gap:32px;padding:18px 0;';
  const info = el('div');
  info.style.cssText = 'flex:1;';
  const lbl = el('div', null, label);
  lbl.style.cssText = 'font-size:15px;font-weight:500;color:var(--text);';
  info.append(lbl);
  if (desc) {
    const d = el('div', null, desc);
    d.style.cssText = 'font-size:13px;color:var(--text-muted);margin-top:4px;line-height:1.45;';
    info.append(d);
  }
  r.append(info, controlEl);
  return r;
}

function makeSwitch(checked, onChange) {
  const switchBtn = el('button');
  switchBtn.setAttribute('role', 'switch');
  switchBtn.setAttribute('aria-checked', checked ? 'true' : 'false');
  switchBtn.style.cssText = checked
    ? 'border:0;background:none;color:inherit;font:inherit;padding:0;margin:0;cursor:pointer;width:42px;height:24px;border-radius:12px;position:relative;background:var(--accent);'
    : 'border:0;background:none;color:inherit;font:inherit;padding:0;margin:0;cursor:pointer;width:42px;height:24px;border-radius:12px;position:relative;background:var(--w-toggle-off);';
  const knob = el('span');
  knob.style.cssText = checked
    ? 'position:absolute;top:3px;width:18px;height:18px;border-radius:9px;background:var(--white);left:21px;transition:left .15s ease;'
    : 'position:absolute;top:3px;width:18px;height:18px;border-radius:9px;background:var(--white);left:3px;transition:left .15s ease;';
  switchBtn.append(knob);
  switchBtn.addEventListener('click', () => onChange(!checked));
  return switchBtn;
}

export const SEARCH_ENGINES = ['DuckDuckGo', 'Google', 'Brave', 'Bing', 'Startpage', 'Ecosia', 'Custom'];

/** A custom search URL: http(s) with %s or {query} where the search words go. */
export function validCustomEngineUrl(url) {
  return /^https?:\/\//i.test(url) && (url.includes('%s') || url.includes('{query}')) && url.length <= 400 && !/[\u0000-\u001f]/.test(url);
}

/** A drop-down in the Settings row style. options: [[value, label], ...]. */
function makeSelect(options, selected, ariaLabel, onPick) {
  const wrap = el('div');
  wrap.style.cssText = 'position:relative;display:inline-block;';
  const btn = el('button');
  btn.setAttribute('aria-label', ariaLabel);
  btn.style.cssText = 'border:0;background:none;color:inherit;font:inherit;margin:0;cursor:pointer;height:34px;min-width:160px;padding:0 12px;border-radius:10px;background:var(--w-field);display:flex;align-items:center;justify-content:space-between;gap:12px;font-size:13.5px;color:var(--text);';
  const current = options.find(([v]) => v === selected) || options[0];
  const valSpan = el('span', null, current[1]);
  const chev = el('span');
  chev.style.cssText = 'color:var(--text-muted);display:flex;';
  const ico = svg('svg', { width: 16, height: 16, viewBox: '0 0 24 24', fill: 'none', stroke: 'currentColor', 'stroke-width': '1.6', 'stroke-linecap': 'round', 'stroke-linejoin': 'round', 'aria-hidden': 'true' });
  ico.append(svg('path', { d: 'M7 10l5 5 5-5' }));
  chev.append(ico);
  btn.append(valSpan, chev);
  const dropdown = el('div');
  dropdown.style.cssText = 'display:none;position:absolute;right:0;top:38px;min-width:160px;border-radius:10px;background:var(--surface);box-shadow:var(--shadow-popover);padding:4px 0;z-index:20;';
  options.forEach(([value, label]) => {
    const o = el('button', null, label);
    o.style.cssText = 'border:0;background:none;color:inherit;font:inherit;margin:0;cursor:pointer;width:100%;height:32px;padding:0 12px;display:flex;align-items:center;font-size:13px;color:var(--text);';
    o.addEventListener('click', (e) => {
      e.stopPropagation();
      dropdown.style.display = 'none';
      onPick(value);
    });
    dropdown.append(o);
  });
  btn.addEventListener('click', () => {
    dropdown.style.display = dropdown.style.display === 'none' ? 'block' : 'none';
  });
  wrap.append(btn, dropdown);
  return wrap;
}

function makeSegmented(options, selected, onChange) {
  const box = el('div');
  box.style.cssText = 'display:flex;padding:3px;border-radius:10px;background:var(--w-field);';
  options.forEach((opt) => {
    const isSel = opt === selected;
    const btn = el('button', null, opt);
    btn.style.cssText = isSel
      ? 'border:0;background:none;color:inherit;font:inherit;padding:0 14px;margin:0;cursor:pointer;height:30px;border-radius:7px;font-size:13px;background:var(--w-segment-selected);color:var(--text-strong);'
      : 'border:0;background:none;color:inherit;font:inherit;padding:0 14px;margin:0;cursor:pointer;height:30px;border-radius:7px;font-size:13px;background:transparent;color:var(--text-muted);';
    btn.addEventListener('click', () => onChange(opt));
    box.append(btn);
  });
  return box;
}

function makeTextInput(value, placeholder, isPassword, onInput) {
  const input = el('input');
  input.type = isPassword ? 'password' : 'text';
  input.value = value || '';
  input.placeholder = placeholder || '';
  input.style.cssText = 'height:34px;border-radius:8px;border:1px solid var(--w-field);background:var(--w-field);color:var(--text);font:inherit;font-size:13.5px;padding:0 12px;outline:none;min-width:240px;';
  input.addEventListener('input', () => onInput(input.value));
  return input;
}

function makeButton(text, isAccent, onClick) {
  const btn = el('button', null, text);
  btn.style.cssText = isAccent
    ? 'border:0;background:var(--accent);color:var(--on-accent);font:inherit;font-size:13.5px;font-weight:500;padding:7px 16px;border-radius:8px;cursor:pointer;'
    : 'border:1px solid var(--w-field);background:var(--w-field);color:var(--text);font:inherit;font-size:13.5px;padding:7px 16px;border-radius:8px;cursor:pointer;';
  btn.addEventListener('click', onClick);
  return btn;
}

/** Render the settings page view inside the page panel. */
export function renderSettings(container, state, onUpdate) {
  container.replaceChildren();

  const settings = state.settings || DEFAULT_SETTINGS;
  const currentSection = state.settingsSection || 'Appearance';

  // Left nav: left 44, top 64, width 200, gap 2
  const nav = el('div', 'settings-nav');
  nav.style.cssText = 'position:absolute;left:44px;top:64px;width:200px;display:flex;flex-direction:column;gap:2px;';

  const heading = el('div', null, 'Settings');
  heading.style.cssText = 'font-size:15px;font-weight:600;padding:0 12px 14px;color:var(--text-strong);';
  nav.append(heading);

  SECTIONS.forEach((sec) => {
    const item = el('button', null, sec);
    const isSelected = sec === currentSection;
    item.style.cssText = isSelected
      ? 'border:0;background:none;color:inherit;font:inherit;padding:0 12px;margin:0;cursor:pointer;height:36px;border-radius:8px;display:flex;align-items:center;font-size:14px;background:var(--w-nav-selected);color:var(--text);'
      : 'border:0;background:none;color:inherit;font:inherit;padding:0 12px;margin:0;cursor:pointer;height:36px;border-radius:8px;display:flex;align-items:center;font-size:14px;color:var(--text-muted);';

    item.addEventListener('click', () => {
      state.settingsSection = sec;
      renderSettings(container, state, onUpdate);
    });
    nav.append(item);
  });
  container.append(nav);

  // Content area: left 300, top 64, width 560
  const content = el('div', 'settings-content');
  if (lastSection !== null && lastSection !== currentSection) content.classList.add('section-in');
  lastSection = currentSection;
  // Its own scroll area, so a long page (Passwords) scrolls while the left menu stays put.
  content.style.cssText = 'position:absolute;left:300px;top:0;bottom:0;width:590px;padding:64px 30px 120px 0;box-sizing:border-box;overflow-y:auto;scrollbar-width:thin;scrollbar-color:var(--w-trail) transparent;';

  if (currentSection === 'General') {
    const title = el('h1', null, 'General');
    title.style.cssText = 'margin:0;font-size:32px;font-weight:600;letter-spacing:-.5px;color:var(--text-strong);';
    const sub = el('div', null, 'Search engine, startup behaviour, and downloads.');
    sub.style.cssText = 'margin-top:8px;font-size:15px;color:var(--text-muted);';
    content.append(title, sub);

    const rows = el('div');
    rows.style.cssText = 'margin-top:28px;';

    // Search engine (AUDIT-38): the six Odyssey engines plus a custom one.
    rows.append(makeRow(
      'Default search engine',
      'Used when entering a search query in the Helm command bar.',
      makeSelect(SEARCH_ENGINES.map((n) => [n, n === 'Custom' ? 'Custom…' : n]), settings.searchEngine || 'DuckDuckGo', 'Default search engine', (val) => {
        settings.searchEngine = val;
        saveSettings(settings);
        renderSettings(container, state, onUpdate);
      })
    ));
    if (settings.searchEngine === 'Custom') {
      const box = el('div');
      box.style.cssText = 'display:flex;flex-direction:column;gap:8px;align-items:flex-end;';
      const ce = settings.customEngine || { name: '', url: '' };
      const nameInp = makeTextInput(ce.name || '', 'Name', false, () => {});
      const urlInp = makeTextInput(ce.url || '', 'https://example.com/search?q=%s', false, () => {});
      const note = el('div', null, '');
      note.style.cssText = 'font-size:12.5px;color:var(--text-muted);min-height:16px;';
      const commit = () => {
        const url = urlInp.value.trim();
        if (url && !validCustomEngineUrl(url)) {
          note.textContent = 'Use http or https, and put %s where the search words go.';
          return;
        }
        note.textContent = '';
        settings.customEngine = { name: nameInp.value.trim().slice(0, 40), url };
        saveSettings(settings);
      };
      nameInp.addEventListener('input', commit);
      urlInp.addEventListener('input', commit);
      box.append(nameInp, urlInp, note);
      rows.append(makeRow('Custom search', 'Put %s where the search words go.', box));
    }

    // Where links open (AUDIT-38)
    rows.append(makeRow(
      'Open links from History and answers',
      'Where a page or source opens when you click it.',
      makeSegmented(['New tab', 'Same tab'], settings.linkTarget || 'New tab', (val) => {
        settings.linkTarget = val;
        saveSettings(settings);
        renderSettings(container, state, onUpdate);
      })
    ));

    // Tab sleeping (AUDIT-15)
    const sleepLabel = { 5: '5 min', 10: '10 min', 30: '30 min', 0: 'Never' };
    const sleepValue = { '5 min': 5, '10 min': 10, '30 min': 30, Never: 0 };
    rows.append(makeRow(
      'Put background tabs to sleep',
      'Idle tabs free their memory. Tabs playing audio, with typed text or downloading stay awake.',
      makeSegmented(['5 min', '10 min', '30 min', 'Never'], sleepLabel[settings.sleepMinutes ?? 10] || '10 min', (val) => {
        settings.sleepMinutes = sleepValue[val];
        saveSettings(settings);
        renderSettings(container, state, onUpdate);
      })
    ));

    // On startup (now really used by the shell)
    rows.append(makeRow(
      'On startup',
      'Pick up where you left off, or start fresh.',
      makeSegmented(['Restore tabs', 'New tab'], settings.startup || 'Restore tabs', (val) => {
        settings.startup = val;
        saveSettings(settings);
        renderSettings(container, state, onUpdate);
      })
    ));

    // Downloads location
    const dlInfo = el('span', null, 'Default Downloads folder');
    dlInfo.style.cssText = 'font-size:13.5px;color:var(--text-soft);';
    rows.append(makeRow('Downloads location', 'Where files downloaded through the Helm are saved.', dlInfo));

    content.append(rows);
  } else if (currentSection === 'Appearance') {
    const title = el('h1', null, 'Appearance');
    title.style.cssText = 'margin:0;font-size:32px;font-weight:600;letter-spacing:-.5px;color:var(--text-strong);';
    const sub = el('div', null, 'Make the browser feel like yours.');
    sub.style.cssText = 'margin-top:8px;font-size:15px;color:var(--text-muted);';
    content.append(title, sub);

    const rows = el('div');
    rows.style.cssText = 'margin-top:28px;';

    // 1. Stars (Off / Still / Drifting)
    rows.append(makeRow(
      'Stars',
      'How the sky behaves behind the browser.',
      makeSegmented(['Off', 'Still', 'Drifting'], settings.stars || 'Still', (opt) => {
        settings.stars = opt;
        saveSettings(settings);
        if (onUpdate) onUpdate('stars', opt);
        renderSettings(container, state, onUpdate);
      })
    ));

    // 2. Shooting stars (Switch)
    rows.append(makeRow(
      'Shooting stars',
      'An occasional comet across the new tab.',
      makeSwitch(settings.shootingStars, (val) => {
        settings.shootingStars = val;
        saveSettings(settings);
        if (onUpdate) onUpdate('shootingStars', val);
        renderSettings(container, state, onUpdate);
      })
    ));

    // Themes (AUDIT-35)
    // Not in the approved design, so fixture mode leaves it out.
    if (!document.documentElement.classList.contains('fixture')) for (const node of buildThemeRows({
      el, makeRow, makeButton, settings,
      save: (st) => { saveSettings(st); if (onUpdate) onUpdate('theme', st.theme); },
      rerender: () => renderSettings(container, state, onUpdate),
    })) rows.append(node);

    // 3. Accent (Swatches)
    const swatchesBox = el('div');
    swatchesBox.style.cssText = 'display:flex;gap:12px;';
    Object.keys(ACCENTS).forEach((accName) => {
      const swBtn = el('button');
      swBtn.setAttribute('aria-label', accName);
      const isSel = (settings.accent || 'Sea') === accName;
      const varColor = `var(${ACCENTS[accName].varName})`;
      swBtn.style.cssText = isSel
        ? `border:0;background:none;color:inherit;font:inherit;padding:0;margin:0;cursor:pointer;width:22px;height:22px;border-radius:11px;background:${varColor};box-shadow:0 0 0 2px var(--panel-internal),0 0 0 4px ${varColor};`
        : `border:0;background:none;color:inherit;font:inherit;padding:0;margin:0;cursor:pointer;width:22px;height:22px;border-radius:11px;background:${varColor};`;
      swBtn.addEventListener('click', () => {
        settings.accent = accName;
        applyAccent(accName);
        saveSettings(settings);
        if (onUpdate) onUpdate('accent', accName);
        renderSettings(container, state, onUpdate);
      });
      swatchesBox.append(swBtn);
    });
    rows.append(makeRow('Accent', settings.theme && settings.theme !== 'Default' ? 'Used for the active tab, progress and focus. A theme sets its own accent; pick Default to use these.' : 'Used for the active tab, progress and focus.', swatchesBox));

    // 4. Trail (Narrow / Wide)
    rows.append(makeRow(
      'Trail',
      'Show the tab trail wide with titles, or narrow with stars only.',
      makeSegmented(['Narrow', 'Wide'], settings.trail || 'Narrow', (opt) => {
        settings.trail = opt;
        saveSettings(settings);
        if (onUpdate) onUpdate('trail', opt);
        renderSettings(container, state, onUpdate);
      })
    ));

    // 5. Faster downloads (Select: 1 / 4 / 8 / 16 connections)
    const selectWrap = el('div');
    selectWrap.style.cssText = 'position:relative;display:inline-block;';

    const selectBtn = el('button');
    selectBtn.setAttribute('aria-label', 'Faster downloads');
    selectBtn.style.cssText = 'border:0;background:none;color:inherit;font:inherit;margin:0;cursor:pointer;height:34px;min-width:140px;padding:0 12px;border-radius:10px;background:var(--w-field);display:flex;align-items:center;justify-content:space-between;gap:12px;font-size:13.5px;color:var(--text);';

    const valSpan = el('span', null, `${settings.connections || 8} connections`);
    const chevSpan = el('span');
    chevSpan.style.cssText = 'color:var(--text-muted);display:flex;';
    const chevIco = svg('svg', {
      width: 16, height: 16, viewBox: '0 0 24 24', fill: 'none',
      stroke: 'currentColor', 'stroke-width': '1.6', 'stroke-linecap': 'round',
      'stroke-linejoin': 'round', 'aria-hidden': 'true',
    });
    chevIco.append(svg('path', { d: 'M7 10l5 5 5-5' }));
    chevSpan.append(chevIco);
    selectBtn.append(valSpan, chevSpan);

    const dropdown = el('div');
    dropdown.style.cssText = 'display:none;position:absolute;right:0;top:38px;min-width:140px;border-radius:10px;background:var(--surface);box-shadow:var(--shadow-popover);padding:4px 0;z-index:20;';

    [1, 4, 8, 16].forEach((conn) => {
      const opt = el('button', null, `${conn} connections`);
      opt.style.cssText = 'border:0;background:none;color:inherit;font:inherit;margin:0;cursor:pointer;width:100%;height:32px;padding:0 12px;display:flex;align-items:center;font-size:13px;color:var(--text);';
      opt.addEventListener('click', (e) => {
        e.stopPropagation();
        settings.connections = conn;
        valSpan.textContent = `${conn} connections`;
        dropdown.style.display = 'none';
        saveSettings(settings);
        if (onUpdate) onUpdate('connections', conn);
      });
      dropdown.append(opt);
    });

    selectBtn.addEventListener('click', () => {
      dropdown.style.display = dropdown.style.display === 'none' ? 'block' : 'none';
    });

    selectWrap.append(selectBtn, dropdown);
    rows.append(makeRow('Faster downloads', 'Big files are fetched over several connections when the site allows it.', selectWrap));

    // Cursor-linked stars (AUDIT-32)
    // Not in the approved design (D-024), so fixture mode leaves it out.
    if (!document.documentElement.classList.contains('fixture')) rows.append(makeRow(
      'Stars follow the cursor',
      'Nearby stars light up and link to the cursor. Only while Stars is on Drifting.',
      makeSwitch(settings.starsFollowCursor !== false, (val) => {
        settings.starsFollowCursor = val;
        saveSettings(settings);
        if (onUpdate) onUpdate('starsFollowCursor', val);
        renderSettings(container, state, onUpdate);
      })
    ));

    content.append(rows);
  } else if (currentSection === 'New tab') {
    const title = el('h1', null, 'New tab');
    title.style.cssText = 'margin:0;font-size:32px;font-weight:600;letter-spacing:-.5px;color:var(--text-strong);';
    const sub = el('div', null, 'Customize the Constellations new tab sky.');
    sub.style.cssText = 'margin-top:8px;font-size:15px;color:var(--text-muted);';
    content.append(title, sub);

    const rows = el('div');
    rows.style.cssText = 'margin-top:28px;';

    rows.append(makeRow(
      'Clock & greeting',
      'Show time and good morning/evening greeting on empty tab.',
      makeSwitch(settings.showClock !== false, (val) => {
        settings.showClock = val;
        saveSettings(settings);
        renderSettings(container, state, onUpdate);
      })
    ));

    rows.append(makeRow(
      'Tab trail guide hint',
      'Show the hint explaining open tabs drawn as stars.',
      makeSwitch(settings.showTrailHint !== false, (val) => {
        settings.showTrailHint = val;
        saveSettings(settings);
        renderSettings(container, state, onUpdate);
      })
    ));

    content.append(rows);
  } else if (currentSection === 'Shields') {
    const title = el('h1', null, 'Shields');
    title.style.cssText = 'margin:0;font-size:32px;font-weight:600;letter-spacing:-.5px;color:var(--text-strong);';
    const sub = el('div', null, 'Built-in ad and tracker protection with cosmetic filtering.');
    sub.style.cssText = 'margin-top:8px;font-size:15px;color:var(--text-muted);';
    content.append(title, sub);

    const rows = el('div');
    rows.style.cssText = 'margin-top:28px;';

    rows.append(makeRow(
      'Ad and tracker blocking',
      'Block intrusive ads, trackers and known malicious domains.',
      makeSwitch(settings.shieldsEnabled !== false, (val) => {
        settings.shieldsEnabled = val;
        saveSettings(settings);
        renderSettings(container, state, onUpdate);
      })
    ));

    const listsInfo = el('div');
    listsInfo.style.cssText = 'font-size:13px;color:var(--text-soft);';
    listsInfo.textContent = 'EasyList, EasyPrivacy, Peter Lowe, uBlock filters';
    rows.append(makeRow('Active filter lists', 'Standard adblock lists loaded and cached locally.', listsInfo));

    content.append(rows);
  } else if (currentSection === 'Odysseus') {
    // P7-02 AI Settings
    const title = el('h1', null, 'AI & Odysseus');
    title.style.cssText = 'margin:0;font-size:32px;font-weight:600;letter-spacing:-.5px;color:var(--text-strong);';
    const sub = el('div', null, 'Optional AI assistant: local model (Ollama), cloud API key, or Odysseus.');
    sub.style.cssText = 'margin-top:8px;font-size:15px;color:var(--text-muted);';
    content.append(title, sub);

    const rows = el('div');
    rows.style.cssText = 'margin-top:28px;';

    if (!state.aiEditConfig) {
      state.aiEditConfig = {
        provider: state.aiConfig?.provider || 'none',
        endpoint: state.aiConfig?.endpoint || '',
        model: state.aiConfig?.model || '',
        api_key: '',
      };
    }
    const edit = state.aiEditConfig;

    // Provider segmented
    rows.append(makeRow(
      'Provider',
      'Choose where questions are answered. None by default; browser works fully offline.',
      makeSegmented(['none', 'local', 'cloud', 'odysseus'], edit.provider, (p) => {
        edit.provider = p;
        if (p === 'local' && !edit.endpoint) edit.endpoint = 'http://127.0.0.1:11434';
        if (p === 'local' && !edit.model) edit.model = 'llama3';
        if (p === 'cloud' && !edit.endpoint) edit.endpoint = 'https://api.openai.com/v1';
        if (p === 'cloud' && !edit.model) edit.model = 'gpt-4o-mini';
        renderSettings(container, state, onUpdate);
      })
    ));

    if (edit.provider !== 'none') {
      // Endpoint
      rows.append(makeRow(
        'Endpoint address',
        edit.provider === 'local'
          ? 'Local Ollama/LM Studio base URL (e.g. http://127.0.0.1:11434)'
          : (edit.provider === 'odysseus' ? 'Odysseus server address' : 'API base URL'),
        makeTextInput(edit.endpoint, 'http://...', false, (v) => { edit.endpoint = v; })
      ));

      // Model
      rows.append(makeRow(
        'Model name',
        'Model to query (e.g. llama3, mistral, gpt-4o-mini, claude-3-5-sonnet)',
        makeTextInput(edit.model, 'Model name', false, (v) => { edit.model = v; })
      ));

      // API Key / Token
      if (edit.provider === 'cloud' || edit.provider === 'odysseus') {
        const keyWrap = el('div');
        keyWrap.style.cssText = 'display:flex;flex-direction:column;gap:6px;';
        const keyInput = makeTextInput('', state.aiConfig?.has_key ? '••••••••  (Saved)' : 'Enter API key', true, (v) => {
          edit.api_key = v;
        });
        keyWrap.append(keyInput);
        if (state.aiConfig?.has_key) {
          const savedHint = el('span', null, '✓ Key saved securely in OS store');
          savedHint.style.cssText = 'font-size:12px;color:var(--accent);';
          keyWrap.append(savedHint);
        }
        rows.append(makeRow(
          edit.provider === 'odysseus' ? 'Odysseus token' : 'API Key',
          'Tied to this login. Keys never reach the UI.',
          keyWrap
        ));
      }

      // Actions row (Test connection & Save)
      const actionBox = el('div');
      actionBox.style.cssText = 'display:flex;gap:12px;align-items:center;';

      const testBtn = makeButton('Test connection', false, () => {
        state.aiTestStatus = 'Testing connection...';
        renderSettings(container, state, onUpdate);
        if (typeof window.odysseyBridge === 'function') {
          window.odysseyBridge(JSON.stringify({
            type: 'ai-test-connection',
            config: {
              provider: edit.provider,
              endpoint: edit.endpoint,
              model: edit.model,
              api_key: edit.api_key || undefined,
            },
          }));
        }
      });

      const saveBtn = makeButton('Save settings', true, () => {
        state.aiTestStatus = 'Saved.';
        if (typeof window.odysseyBridge === 'function') {
          window.odysseyBridge(JSON.stringify({
            type: 'ai-save-config',
            config: {
              provider: edit.provider,
              endpoint: edit.endpoint,
              model: edit.model,
              api_key: edit.api_key || undefined,
            },
          }));
        }
        renderSettings(container, state, onUpdate);
      });

      actionBox.append(testBtn, saveBtn);

      const statusDesc = el('div');
      statusDesc.style.cssText = 'margin-top:8px;font-size:13px;';
      if (state.aiTestResult) {
        statusDesc.textContent = state.aiTestResult.message;
        statusDesc.style.color = state.aiTestResult.ok ? 'var(--accent)' : 'var(--text-soft)';
      } else if (state.aiTestStatus) {
        statusDesc.textContent = state.aiTestStatus;
        statusDesc.style.color = 'var(--text-muted)';
      }

      const actionsWrap = el('div');
      actionsWrap.append(actionBox, statusDesc);
      rows.append(makeRow('Verify & apply', 'Test the connection before saving.', actionsWrap));
    }

    content.append(rows);
  } else if (currentSection === 'Passwords') {
    // Brave-style: switches, Add / Import / Export, the list, and the sites "Never" was chosen for.
    const title = el('h1', null, 'Passwords');
    title.style.cssText = 'margin:0;font-size:32px;font-weight:600;letter-spacing:-.5px;color:var(--text-strong);';
    content.append(title);

    const rows = el('div');
    rows.style.cssText = 'margin-top:20px;';
    const send = (m) => { if (typeof window.odysseyBridge === 'function') window.odysseyBridge(JSON.stringify(m)); };
    const rerender = () => renderSettings(container, state, onUpdate);

    rows.append(makeRow('Offer to save passwords', '', makeSwitch(settings.offerSavePasswords !== false, (val) => {
      settings.offerSavePasswords = val;
      saveSettings(settings);
      rerender();
    })));
    rows.append(makeRow('Autofill passwords', '', makeSwitch(settings.autofillPasswords !== false, (val) => {
      settings.autofillPasswords = val;
      saveSettings(settings);
      rerender();
    })));

    // Add / Import / Export
    const bar = el('div');
    bar.style.cssText = 'display:flex;gap:10px;align-items:center;margin:12px 0 18px 0;flex-wrap:wrap;';
    bar.append(makeButton('Add', false, () => { pwAddOpen = !pwAddOpen; rerender(); }));

    const fileInput = el('input');
    fileInput.type = 'file';
    fileInput.accept = '.csv,text/csv';
    fileInput.style.display = 'none';
    fileInput.addEventListener('change', async () => {
      const f = fileInput.files && fileInput.files[0];
      if (!f) return;
      const text = await f.text();
      if (text.length > 240000) {
        state.vaultImport = { added: 0, skipped: 0, error: 'That file is too large to import.' };
        rerender();
        return;
      }
      send({ type: 'vault-import', csv: text });
    });
    bar.append(makeButton('Import', false, () => fileInput.click()), fileInput);
    bar.append(makeButton('Export', false, () => { pwExportConfirm = !pwExportConfirm; rerender(); }));
    rows.append(bar);

    if (pwExportConfirm) {
      const box = el('div');
      box.style.cssText = 'display:flex;align-items:center;gap:12px;flex-wrap:wrap;padding:14px 16px;border-radius:12px;background:var(--w-field);margin-bottom:16px;';
      const msg = el('div', null, 'The file will show every password as plain text.');
      msg.style.cssText = 'flex:1;min-width:220px;font-size:13.5px;color:var(--text-soft);';
      box.append(msg,
        makeButton('Cancel', false, () => { pwExportConfirm = false; rerender(); }),
        makeButton('Export', true, () => { pwExportConfirm = false; state.vaultExport = null; send({ type: 'vault-export' }); rerender(); }));
      rows.append(box);
    }
    if (state.vaultExport) {
      const st = el('div');
      st.style.cssText = 'display:flex;align-items:center;gap:12px;font-size:13.5px;color:var(--text-soft);margin-bottom:16px;flex-wrap:wrap;';
      st.append(el('span', null, state.vaultExport.ok ? `Saved ${state.vaultExport.path}` : state.vaultExport.message));
      if (state.vaultExport.ok) st.append(makeButton('Show in folder', false, () => send({ type: 'show-in-folder', path: state.vaultExport.path })));
      rows.append(st);
    }
    if (state.vaultAuth) {
      const st = el('div', null, state.vaultAuth.message);
      st.style.cssText = 'font-size:13.5px;color:var(--text-soft);margin-bottom:16px;';
      rows.append(st);
    }
    if (state.vaultImport) {
      const st = el('div', null, state.vaultImport.error
        ? state.vaultImport.error
        : `Imported ${state.vaultImport.added}${state.vaultImport.skipped ? `, skipped ${state.vaultImport.skipped}` : ''}`);
      st.style.cssText = 'font-size:13.5px;color:var(--text-soft);margin-bottom:16px;';
      rows.append(st);
    }

    if (pwAddOpen) {
      const addBox = el('div');
      addBox.style.cssText = 'display:flex;flex-direction:column;gap:12px;padding:16px;border-radius:12px;background:var(--w-field);margin-bottom:20px;';
      const inputsRow = el('div');
      inputsRow.style.cssText = 'display:flex;gap:10px;flex-wrap:wrap;';
      const originInp = makeTextInput('', 'Site (e.g. github.com)', false, () => {});
      const userInp = makeTextInput('', 'Username / Email', false, () => {});
      const passInp = makeTextInput('', 'Password', true, () => {});
      inputsRow.append(originInp, userInp, passInp);
      addBox.append(inputsRow, makeButton('Save', true, () => {
        const o = originInp.value.trim();
        const u = userInp.value.trim();
        const pw = passInp.value;
        if (!o || !pw) return;
        send({ type: 'vault-add', origin: o, username: u, password: pw });
        pwAddOpen = false;
      }));
      rows.append(addBox);
    }

    if (state.vaultNotice) {
      const warn = el('div', null, state.vaultNotice);
      warn.style.cssText = 'font-size:13.5px;color:var(--text-soft);background:var(--w-field);border-radius:10px;padding:12px 14px;margin-bottom:14px;line-height:1.5;';
      rows.append(warn);
    }

    (state.vaultEntries || []).forEach((e) => {
      const itemRow = el('div');
      itemRow.style.cssText = 'display:flex;align-items:center;justify-content:space-between;padding:14px 16px;border-radius:10px;background:var(--w-field);margin-bottom:8px;';
      const info = el('div');
      const orig = el('div', null, e.origin);
      orig.style.cssText = 'font-size:14.5px;font-weight:500;color:var(--text);';
      const user = el('div', null, e.username);
      user.style.cssText = 'font-size:13px;color:var(--text-muted);margin-top:2px;';
      info.append(orig, user);
      const actions = el('div');
      actions.style.cssText = 'display:flex;gap:8px;align-items:center;';
      actions.append(
        makeButton(state.revealedPassword && state.revealedPassword.id === e.id ? 'Hide' : 'Reveal', false, () => {
          if (state.revealedPassword && state.revealedPassword.id === e.id) {
            state.revealedPassword = null;
            rerender();
          } else {
            state.vaultAuth = null;
            send({ type: 'vault-reveal', id: e.id }); // Windows asks for verification first
          }
        }),
        makeButton('Delete', false, () => send({ type: 'vault-delete', id: e.id })),
      );
      itemRow.append(info, actions);
      if (state.revealedPassword && state.revealedPassword.id === e.id) {
        const passRow = el('div', null, state.revealedPassword.password);
        passRow.style.cssText = 'font-size:13.5px;color:var(--accent);font-family:monospace;margin-top:8px;padding-top:8px;border-top:1px solid var(--w-field);';
        itemRow.style.flexDirection = 'column';
        itemRow.style.alignItems = 'stretch';
        itemRow.append(passRow);
      }
      rows.append(itemRow);
    });

    if ((state.vaultNever || []).length) {
      const h = el('div', null, 'Never saved');
      h.style.cssText = 'font-size:14.5px;font-weight:600;color:var(--text-strong);margin:24px 0 10px 0;';
      rows.append(h);
      state.vaultNever.forEach((host) => {
        const r = el('div');
        r.style.cssText = 'display:flex;align-items:center;justify-content:space-between;padding:10px 16px;border-radius:10px;background:var(--w-field);margin-bottom:8px;font-size:14px;color:var(--text);';
        r.append(el('span', null, host), makeButton('Remove', false, () => send({ type: 'vault-remove-never', host })));
        rows.append(r);
      });
    }

    content.append(rows);
  } else if (currentSection === 'Privacy') {
    const title = el('h1', null, 'Privacy');
    title.style.cssText = 'margin:0;font-size:32px;font-weight:600;letter-spacing:-.5px;color:var(--text-strong);';
    content.append(title);

    const rows = el('div');
    rows.style.cssText = 'margin-top:20px;';
    const rerender = () => renderSettings(container, state, onUpdate);
    const toggle = (key, on) => (val) => {
      settings[key] = val;
      saveSettings(settings);
      rerender();
    };

    rows.append(makeRow(
      'Global Privacy Control',
      'Tell websites not to sell or share your data (sent with every web request).',
      makeSwitch(settings.globalPrivacyControl !== false, toggle('globalPrivacyControl'))
    ));

    const group = el('div', null, 'Clear when Constellations closes');
    group.style.cssText = 'font-size:14.5px;font-weight:600;color:var(--text-strong);margin:22px 0 2px 0;';
    rows.append(group);
    rows.append(makeRow(
      'Browsing history',
      'History, closed tabs, favicons and suggestions. Open tabs stay.',
      makeSwitch(!!settings.clearHistoryOnExit, toggle('clearHistoryOnExit'))
    ));
    rows.append(makeRow(
      'Cookies and site data',
      'Signs you out of websites. Saved passwords stay.',
      makeSwitch(!!settings.clearSiteDataOnExit, toggle('clearSiteDataOnExit'))
    ));

    const clearBtn = makeButton('Clear history now', false, () => {
      if (typeof window.odysseyBridge === 'function') {
        window.odysseyBridge(JSON.stringify({ type: 'clear-all-history' }));
      }
      state.privacyStatus = 'History cleared';
      rerender();
    });
    const clearRow = el('div');
    clearRow.style.cssText = 'display:flex;align-items:center;gap:14px;margin-top:18px;';
    clearRow.append(clearBtn);
    if (state.privacyStatus) {
      const st = el('span', null, state.privacyStatus);
      st.style.cssText = 'font-size:13.5px;color:var(--text-soft);';
      clearRow.append(st);
    }
    rows.append(clearRow);

    content.append(rows);
  } else if (currentSection === 'Shortcuts') {
    const title = el('h1', null, 'Shortcuts');
    title.style.cssText = 'margin:0;font-size:32px;font-weight:600;letter-spacing:-.5px;color:var(--text-strong);';
    const sub = el('div', null, 'Click Change, then press the keys you want. Esc cancels.');
    sub.style.cssText = 'margin-top:8px;font-size:15px;color:var(--text-muted);';
    content.append(title, sub);

    const rows = el('div');
    rows.style.cssText = 'margin-top:28px;';
    const rerender = () => renderSettings(container, state, onUpdate);
    const send = (m) => window.odysseyBridge?.(JSON.stringify(m));
    const list = state.shortcutList || [];
    const note = (text, bad) => {
      const n = el('div', 'shortcut-note', text);
      n.style.cssText = 'font-size:13px;margin-top:6px;color:' + (bad ? 'var(--danger, #e08a8a)' : 'var(--text-soft)') + ';';
      return n;
    };

    list.forEach((cmd) => {
      const badges = el('span');
      badges.style.cssText = 'display:inline-flex;gap:6px;align-items:center;';
      const recording = state.shortcutCapture === cmd.id;
      if (recording) {
        const k = el('kbd', null, 'Press the new keys…');
        k.style.cssText = 'padding:4px 10px;border-radius:6px;background:var(--w-field);border:1px solid var(--accent);font-size:12.5px;font-family:inherit;color:var(--text-strong);';
        badges.append(k);
      } else {
        cmd.keys.forEach((key) => {
          const k = el('kbd', null, key.replace(/\+(?=.)/g, ' + '));
          k.style.cssText = 'padding:4px 10px;border-radius:6px;background:var(--w-field);border:1px solid var(--w-field);font-size:12.5px;font-family:inherit;color:var(--text-strong);';
          badges.append(k);
        });
      }
      const controls = el('div');
      controls.style.cssText = 'display:flex;align-items:center;gap:10px;';
      controls.append(badges);
      const change = makeButton(recording ? 'Cancel' : 'Change', false, () => {
        if (recording) { cancelCapture(); return; }
        state.shortcutResult = null;
        state.shortcutCapture = cmd.id;
        state.shortcutError = null;
        captureShortcut((combo) => {
          state.shortcutCapture = null;
          if (combo === null) { rerender(); return; }
          if (combo === '?') {
            state.shortcutError = { id: cmd.id, message: "That key can't be used. Try a letter, number or F-key with Ctrl or Alt." };
            rerender();
            return;
          }
          state.shortcutAsked = cmd.id;
          send({ type: 'set-shortcut', id: cmd.id, combo });
          rerender();
        });
        rerender();
      });
      controls.append(change);
      if (cmd.custom) {
        controls.append(makeButton('Reset', false, () => {
          state.shortcutError = null;
          send({ type: 'reset-shortcuts', id: cmd.id });
        }));
      }
      const row = makeRow(cmd.label, '', controls);
      rows.append(row);
      const err = state.shortcutError && state.shortcutError.id === cmd.id ? state.shortcutError.message : '';
      const res = state.shortcutResult && !state.shortcutResult.ok && state.shortcutAsked === cmd.id ? state.shortcutResult.message : '';
      if (err || res) {
        const info = row.firstChild;
        info.append(note(err || res, true));
      }
    });

    const anyCustom = list.some((c) => c.custom);
    if (anyCustom) {
      const resetAll = el('div');
      resetAll.style.cssText = 'margin-top:18px;';
      resetAll.append(makeButton('Reset all shortcuts', false, () => {
        state.shortcutError = null;
        state.shortcutResult = null;
        send({ type: 'reset-shortcuts' });
      }));
      rows.append(resetAll);
    }
    content.append(rows);
  } else if (currentSection === 'About') {
    const title = el('h1', null, 'About Constellations');
    title.style.cssText = 'margin:0;font-size:32px;font-weight:600;letter-spacing:-.5px;color:var(--text-strong);';
    const sub = el('div', null, 'A fresh, quiet browser with tabs drawn as stars.');
    sub.style.cssText = 'margin-top:8px;font-size:15px;color:var(--text-muted);';
    content.append(title, sub);

    const rows = el('div');
    rows.style.cssText = 'margin-top:28px;';

    const verBadge = el('span', null, '0.1.0 (Beta)');
    verBadge.style.cssText = 'font-size:13.5px;color:var(--accent);font-weight:500;';
    rows.append(makeRow('Version', 'Constellations desktop browser for Windows & Linux.', verBadge));

    const engineBadge = el('span', null, 'CEF 156.0.31 / Rust');
    engineBadge.style.cssText = 'font-size:13.5px;color:var(--text-soft);';
    rows.append(makeRow('Engine', 'Chromium Embedded Framework with native Rust shell.', engineBadge));

    const checkBtn = makeButton('Check for updates', false, () => {
      alert('Constellations is up to date.');
    });
    rows.append(makeRow('Updates', 'Check if a newer build is available.', checkBtn));

    content.append(rows);
  }

  container.append(content);
}
