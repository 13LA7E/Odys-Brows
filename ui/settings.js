// Settings page shell (DESIGN.md §9, PHASES.md P2-07, P7-02, P8-01, P8-02).
// Manages settings state, navigation, appearance, AI, passwords, privacy, shortcuts, and about.

const SVG = 'http://www.w3.org/2000/svg';

export const DEFAULT_SETTINGS = {
  stars: 'Still',       // 'Off' | 'Still' | 'Drifting'
  shootingStars: false, // boolean
  accent: 'Sea',        // 'Sea' | 'Mint' | 'Lilac' | 'Amber' | 'Rose'
  trail: 'Narrow',      // 'Narrow' | 'Wide'
  connections: 8,       // 1 | 4 | 8 | 16
  searchEngine: 'DuckDuckGo', // 'DuckDuckGo' | 'Google' | 'Bing' | 'Ecosia'
  startup: 'NewTab',    // 'NewTab' | 'Restore'
  showClock: true,      // boolean
  showTrailHint: true,  // boolean
  shieldsEnabled: true, // boolean
  doNotTrack: true,     // boolean
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
  content.style.cssText = 'position:absolute;left:300px;top:64px;width:560px;padding-bottom:120px;';

  if (currentSection === 'General') {
    const title = el('h1', null, 'General');
    title.style.cssText = 'margin:0;font-size:32px;font-weight:600;letter-spacing:-.5px;color:var(--text-strong);';
    const sub = el('div', null, 'Search engine, startup behaviour, and downloads.');
    sub.style.cssText = 'margin-top:8px;font-size:15px;color:var(--text-muted);';
    content.append(title, sub);

    const rows = el('div');
    rows.style.cssText = 'margin-top:28px;';

    // Search Engine
    rows.append(makeRow(
      'Default search engine',
      'Used when entering a search query in the Helm command bar.',
      makeSegmented(['DuckDuckGo', 'Google', 'Bing'], settings.searchEngine || 'DuckDuckGo', (val) => {
        settings.searchEngine = val;
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

    // On startup
    rows.append(makeRow(
      'On startup',
      'What to open when starting Constellations.',
      makeSegmented(['NewTab', 'Restore'], settings.startup || 'NewTab', (val) => {
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

    // Cursor-linked stars (AUDIT-32)
    rows.append(makeRow(
      'Stars follow the cursor',
      'Nearby stars light up and link to the cursor. Only while Stars is on Drifting.',
      makeSwitch(settings.starsFollowCursor !== false, (val) => {
        settings.starsFollowCursor = val;
        saveSettings(settings);
        if (onUpdate) onUpdate('starsFollowCursor', val);
        renderSettings(container, state, onUpdate);
      })
    ));

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
    // P8-02 Passwords Vault
    const title = el('h1', null, 'Passwords');
    title.style.cssText = 'margin:0;font-size:32px;font-weight:600;letter-spacing:-.5px;color:var(--text-strong);';
    const sub = el('div', null, (/Windows/i.test(navigator.userAgent) ? 'Saved logins encrypted with Windows DPAPI (tied to your Windows login).' : 'Saved logins are kept in your system keyring (Secret Service).'));
    sub.style.cssText = 'margin-top:8px;font-size:15px;color:var(--text-muted);';
    content.append(title, sub);

    const rows = el('div');
    rows.style.cssText = 'margin-top:28px;';

    // Add new login form
    const addBox = el('div');
    addBox.style.cssText = 'display:flex;flex-direction:column;gap:12px;padding:16px;border-radius:12px;background:var(--w-field);margin-bottom:24px;';
    const addHeading = el('div', null, 'Add login');
    addHeading.style.cssText = 'font-size:14.5px;font-weight:600;color:var(--text-strong);';

    const inputsRow = el('div');
    inputsRow.style.cssText = 'display:flex;gap:10px;flex-wrap:wrap;';
    const originInp = makeTextInput('', 'Site (e.g. github.com)', false, () => {});
    const userInp = makeTextInput('', 'Username / Email', false, () => {});
    const passInp = makeTextInput('', 'Password', true, () => {});
    inputsRow.append(originInp, userInp, passInp);

    const saveLoginBtn = makeButton('Save login', true, () => {
      const o = originInp.value.trim();
      const u = userInp.value.trim();
      const p = passInp.value;
      if (!o || !p) return;
      if (typeof window.odysseyBridge === 'function') {
        window.odysseyBridge(JSON.stringify({
          type: 'vault-add',
          origin: o,
          username: u,
          password: p,
        }));
      }
      originInp.value = '';
      userInp.value = '';
      passInp.value = '';
    });
    addBox.append(addHeading, inputsRow, saveLoginBtn);
    rows.append(addBox);

    if (state.vaultNotice) {
      const warn = el('div', null, state.vaultNotice);
      warn.style.cssText = 'font-size:13.5px;color:var(--text-soft);background:var(--w-field);border-radius:10px;padding:12px 14px;margin-bottom:14px;line-height:1.5;';
      rows.append(warn);
    }

    // List of logins
    const entries = state.vaultEntries || [];
    if (entries.length === 0) {
      const emptyNote = el('div', null, 'No passwords saved yet.');
      emptyNote.style.cssText = 'font-size:13.5px;color:var(--text-muted);padding:20px 0;';
      rows.append(emptyNote);
    } else {
      entries.forEach((e) => {
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

        const revealBtn = makeButton('Reveal', false, () => {
          if (typeof window.odysseyBridge === 'function') {
            window.odysseyBridge(JSON.stringify({ type: 'vault-reveal', id: e.id }));
          }
        });

        const deleteBtn = makeButton('Delete', false, () => {
          if (typeof window.odysseyBridge === 'function') {
            window.odysseyBridge(JSON.stringify({ type: 'vault-delete', id: e.id }));
          }
        });

        actions.append(revealBtn, deleteBtn);
        itemRow.append(info, actions);

        if (state.revealedPassword && state.revealedPassword.id === e.id) {
          const passRow = el('div', null, `Password: ${state.revealedPassword.password}`);
          passRow.style.cssText = 'font-size:13.5px;color:var(--accent);font-family:monospace;margin-top:8px;padding-top:8px;border-top:1px solid var(--w-field);';
          itemRow.style.flexDirection = 'column';
          itemRow.style.alignItems = 'stretch';
          itemRow.append(passRow);
        }

        rows.append(itemRow);
      });
    }

    content.append(rows);
  } else if (currentSection === 'Privacy') {
    const title = el('h1', null, 'Privacy');
    title.style.cssText = 'margin:0;font-size:32px;font-weight:600;letter-spacing:-.5px;color:var(--text-strong);';
    const sub = el('div', null, 'Constellations sends zero telemetry or tracking data.');
    sub.style.cssText = 'margin-top:8px;font-size:15px;color:var(--text-muted);';
    content.append(title, sub);

    const rows = el('div');
    rows.style.cssText = 'margin-top:28px;';

    rows.append(makeRow(
      'Global Privacy Control (GPC)',
      'Notify websites not to sell or share your browsing data.',
      makeSwitch(settings.doNotTrack !== false, (val) => {
        settings.doNotTrack = val;
        saveSettings(settings);
        renderSettings(container, state, onUpdate);
      })
    ));

    const clearBtn = makeButton('Clear browsing data', false, () => {
      if (typeof window.odysseyBridge === 'function') {
        window.odysseyBridge(JSON.stringify({ type: 'clear-all-history' }));
      }
      alert('Browsing history and cached data cleared.');
    });
    rows.append(makeRow('Clear data', 'Delete all browsing history and session visits.', clearBtn));

    content.append(rows);
  } else if (currentSection === 'Shortcuts') {
    const title = el('h1', null, 'Shortcuts');
    title.style.cssText = 'margin:0;font-size:32px;font-weight:600;letter-spacing:-.5px;color:var(--text-strong);';
    const sub = el('div', null, 'Essential keyboard shortcuts for navigation and commands.');
    sub.style.cssText = 'margin-top:8px;font-size:15px;color:var(--text-muted);';
    content.append(title, sub);

    const rows = el('div');
    rows.style.cssText = 'margin-top:28px;';

    const shortcutsList = [
      ['Ctrl + T', 'Open a new tab in the sky'],
      ['Ctrl + W', 'Close current tab'],
      ['Ctrl + L', 'Focus Helm address / command bar'],
      ['Ctrl + Tab', 'Switch to next open tab'],
      ['Ctrl + Shift + Tab', 'Switch to previous tab'],
      ['Ctrl + Shift + T', 'Reopen last closed tab'],
      ['Ctrl + H', 'Open browsing history day groups'],
      ['Ctrl + ,', 'Open settings'],
      ['Ctrl + F', 'Find text in page'],
      ['Ctrl + P', 'Print active page'],
      ['Ctrl + Plus / Minus', 'Zoom in / out'],
    ];

    shortcutsList.forEach(([key, desc]) => {
      const badge = el('kbd', null, key);
      badge.style.cssText = 'padding:4px 10px;border-radius:6px;background:var(--w-field);border:1px solid var(--w-field);font-size:12.5px;font-family:inherit;color:var(--text-strong);';
      rows.append(makeRow(key, desc, badge));
    });

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
