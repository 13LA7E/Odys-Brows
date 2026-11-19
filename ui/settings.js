// Settings page shell (DESIGN.md §9, PHASES.md P2-07).
// Manages settings state, navigation, appearance controls, and persistence.

const SVG = 'http://www.w3.org/2000/svg';

export const DEFAULT_SETTINGS = {
  stars: 'Still',       // 'Off' | 'Still' | 'Drifting'
  shootingStars: false, // boolean
  accent: 'Sea',        // 'Sea' | 'Mint' | 'Lilac' | 'Amber' | 'Rose'
  trail: 'Narrow',      // 'Narrow' | 'Wide'
  connections: 8,       // 1 | 4 | 8 | 16
};

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

/** Render the settings page view inside the page panel. */
export function renderSettings(container, state, onUpdate) {
  container.replaceChildren();

  const settings = state.settings || DEFAULT_SETTINGS;
  const currentSection = state.settingsSection || state.settings?.section || 'Appearance';

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
  content.style.cssText = 'position:absolute;left:300px;top:64px;width:560px;';

  if (currentSection === 'Appearance') {
    const title = el('h1', null, 'Appearance');
    title.style.cssText = 'margin:0;font-size:32px;font-weight:600;letter-spacing:-.5px;color:var(--text-strong);';
    const sub = el('div', null, 'Make the browser feel like yours.');
    sub.style.cssText = 'margin-top:8px;font-size:15px;color:var(--text-muted);';
    content.append(title, sub);

    const rows = el('div');
    rows.style.cssText = 'margin-top:28px;';

    // Helper for rows
    function makeRow(label, desc, controlEl) {
      const r = el('div');
      r.style.cssText = 'display:flex;align-items:center;gap:32px;padding:18px 0;';
      const info = el('div');
      info.style.cssText = 'flex:1;';
      const lbl = el('div', null, label);
      lbl.style.cssText = 'font-size:15px;font-weight:500;color:var(--text);';
      const d = el('div', null, desc);
      d.style.cssText = 'font-size:13px;color:var(--text-muted);margin-top:4px;line-height:1.45;';
      info.append(lbl, d);
      r.append(info, controlEl);
      return r;
    }

    // 1. Stars (Off / Still / Drifting)
    const starsBox = el('div');
    starsBox.style.cssText = 'display:flex;padding:3px;border-radius:10px;background:var(--w-field);';
    ['Off', 'Still', 'Drifting'].forEach((opt) => {
      const optBtn = el('button', null, opt);
      const isSel = (settings.stars || 'Still') === opt;
      optBtn.style.cssText = isSel
        ? 'border:0;background:none;color:inherit;font:inherit;padding:0 14px;margin:0;cursor:pointer;height:30px;border-radius:7px;font-size:13px;background:var(--w-segment-selected);color:var(--text-strong);'
        : 'border:0;background:none;color:inherit;font:inherit;padding:0 14px;margin:0;cursor:pointer;height:30px;border-radius:7px;font-size:13px;background:transparent;color:var(--text-muted);';
      optBtn.addEventListener('click', () => {
        settings.stars = opt;
        saveSettings(settings);
        if (onUpdate) onUpdate('stars', opt);
        renderSettings(container, state, onUpdate);
      });
      starsBox.append(optBtn);
    });
    rows.append(makeRow('Stars', 'How the sky behaves behind the browser.', starsBox));

    // 2. Shooting stars (Switch)
    const switchBtn = el('button');
    switchBtn.setAttribute('role', 'switch');
    switchBtn.setAttribute('aria-checked', settings.shootingStars ? 'true' : 'false');
    switchBtn.setAttribute('aria-label', 'Shooting stars');
    switchBtn.style.cssText = settings.shootingStars
      ? 'border:0;background:none;color:inherit;font:inherit;padding:0;margin:0;cursor:pointer;width:42px;height:24px;border-radius:12px;position:relative;background:var(--accent);'
      : 'border:0;background:none;color:inherit;font:inherit;padding:0;margin:0;cursor:pointer;width:42px;height:24px;border-radius:12px;position:relative;background:var(--w-toggle-off);';
    const knob = el('span');
    knob.style.cssText = settings.shootingStars
      ? 'position:absolute;top:3px;width:18px;height:18px;border-radius:9px;background:var(--white);left:21px;transition:left .15s ease;'
      : 'position:absolute;top:3px;width:18px;height:18px;border-radius:9px;background:var(--white);left:3px;transition:left .15s ease;';
    switchBtn.append(knob);
    switchBtn.addEventListener('click', () => {
      settings.shootingStars = !settings.shootingStars;
      saveSettings(settings);
      if (onUpdate) onUpdate('shootingStars', settings.shootingStars);
      renderSettings(container, state, onUpdate);
    });
    rows.append(makeRow('Shooting stars', 'An occasional comet across the new tab.', switchBtn));

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
    rows.append(makeRow('Accent', 'Used for the active tab, progress and focus.', swatchesBox));

    // 4. Trail (Narrow / Wide)
    const trailBox = el('div');
    trailBox.style.cssText = 'display:flex;padding:3px;border-radius:10px;background:var(--w-field);';
    ['Narrow', 'Wide'].forEach((opt) => {
      const optBtn = el('button', null, opt);
      const isSel = (settings.trail || 'Narrow') === opt;
      optBtn.style.cssText = isSel
        ? 'border:0;background:none;color:inherit;font:inherit;padding:0 14px;margin:0;cursor:pointer;height:30px;border-radius:7px;font-size:13px;background:var(--w-segment-selected);color:var(--text-strong);'
        : 'border:0;background:none;color:inherit;font:inherit;padding:0 14px;margin:0;cursor:pointer;height:30px;border-radius:7px;font-size:13px;background:transparent;color:var(--text-muted);';
      optBtn.addEventListener('click', () => {
        settings.trail = opt;
        saveSettings(settings);
        if (onUpdate) onUpdate('trail', opt);
        renderSettings(container, state, onUpdate);
      });
      trailBox.append(optBtn);
    });
    rows.append(makeRow('Trail', 'Show the tab trail wide with titles, or narrow with stars only.', trailBox));

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

    content.append(rows);
  } else {
    // Non-Appearance section: title only (DESIGN.md §9)
    const title = el('h1', null, currentSection);
    title.style.cssText = 'margin:0;font-size:32px;font-weight:600;letter-spacing:-.5px;color:var(--text-strong);';
    const sub = el('div', null, `Settings for ${currentSection}.`);
    sub.style.cssText = 'margin-top:8px;font-size:15px;color:var(--text-muted);';
    content.append(title, sub);
  }

  container.append(content);
}
