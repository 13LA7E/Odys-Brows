// Odyssey UI: tabs, smart address bar, answers + web results, assistant, Shields,
// settings, history, bookmarks and window chrome.
const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const $ = (s, root = document) => root.querySelector(s);
const $$ = (s, root = document) => Array.from(root.querySelectorAll(s));
const uid = () => Math.random().toString(36).slice(2, 10);
const ICON = {
  close: '<svg viewBox="0 0 16 16"><path d="m4 4 8 8M12 4l-8 8"/></svg>',
  gear: '<svg viewBox="0 0 16 16"><circle cx="8" cy="8" r="2.2"/><path d="M8 1.5v2M8 12.5v2M1.5 8h2M12.5 8h2M3.4 3.4l1.4 1.4M11.2 11.2l1.4 1.4M3.4 12.6l1.4-1.4M11.2 4.8l1.4-1.4"/></svg>',
  clock: '<svg viewBox="0 0 16 16"><path d="M2.5 8a5.5 5.5 0 1 0 1.6-3.9M2.5 2.5v2.8h2.8M8 5v3.2l2.2 1.4"/></svg>',
  shield: '<svg viewBox="0 0 16 16"><path d="M8 1.8 13 3.6v3.9c0 3.2-2.1 5.6-5 6.7-2.9-1.1-5-3.5-5-6.7V3.6z"/></svg>',
  palette: '<svg viewBox="0 0 16 16"><path d="M8 2a6 6 0 1 0 0 12c1 0 1.3-.8.9-1.6-.5-.9.1-1.9 1.1-1.9H12a2 2 0 0 0 2-2C14 4.7 11.3 2 8 2Z"/><circle cx="5" cy="7" r=".8"/><circle cx="8" cy="5" r=".8"/><circle cx="11" cy="7" r=".8"/></svg>',
  spark: '<svg viewBox="0 0 16 16"><path d="M8 1.5 9.4 6.6 14.5 8 9.4 9.4 8 14.5 6.6 9.4 1.5 8 6.6 6.6Z"/></svg>',
  lock: '<svg viewBox="0 0 16 16"><rect x="3" y="7" width="10" height="7" rx="1.5"/><path d="M5.5 7V5a2.5 2.5 0 0 1 5 0v2"/></svg>',
  key: '<svg viewBox="0 0 16 16"><circle cx="5" cy="5" r="2.6"/><path d="M6.8 6.8 13 13M9.5 9.5l1.3 1.3M11.3 7.7l1.3 1.3"/></svg>',
  keys: '<svg viewBox="0 0 16 16"><rect x="1.5" y="4" width="13" height="8" rx="1.5"/><path d="M4 7h.01M6.5 7h.01M9 7h.01M11.5 7h.01M5 9.5h6"/></svg>',
  info: '<svg viewBox="0 0 16 16"><circle cx="8" cy="8" r="6.2"/><path d="M8 7.2v3.6M8 5h.01"/></svg>',
  home: '<svg viewBox="0 0 16 16"><path d="M2.5 7.5 8 3l5.5 4.5M4 6.5V13h8V6.5"/></svg>',
  plus: '<svg viewBox="0 0 16 16"><path d="M8 3v10M3 8h10"/></svg>',
  trash: '<svg viewBox="0 0 16 16"><path d="M3 5h10M5 5v8M7 5v8M9 5v8"/></svg>',
  edit: '<svg viewBox="0 0 16 16"><path d="M12 3l3 3-8 8-4 1 1-4z"/></svg>',
  grip: '<svg viewBox="0 0 16 16"><circle cx="4" cy="4" r="1"/><circle cx="4" cy="8" r="1"/><circle cx="4" cy="12" r="1"/></svg>',
  folder: '<svg viewBox="0 0 16 16"><path d="M3 5h10a1 1 0 0 1 1 1v7a1 1 0 0 1-1 1H3a1 1 0 0 1-1-1V6a1 1 0 0 1 1-1z"/></svg>',
  download: '<svg viewBox="0 0 16 16"><path d="M8 3v10M4 7l4 4 4-4"/></svg>',
  upload: '<svg viewBox="0 0 16 16"><path d="M8 13V3M4 7l4-4 4 4"/></svg>',
  eye: '<svg viewBox="0 0 16 16"><path d="M2 8a6 6 0 0 1 12 0"/><circle cx="8" cy="8" r="3"/></svg>',
  eyeOff: '<svg viewBox="0 0 16 16"><path d="M2 8a6 6 0 0 1 12 0"/><path d="M9.5 9.5 13.5 13.5M2.5 2.5l3 3"/></svg>',
  sun: '<svg viewBox="0 0 16 16"><circle cx="8" cy="8" r="4"/><path d="M8 2v2M8 12v2M2 8h2M12 8h2M3.4 3.4l1.4 1.4M11.2 11.2l1.4 1.4M3.4 12.6l1.4-1.4M11.2 4.8l1.4-1.4"/></svg>',
  moon: '<svg viewBox="0 0 16 16"><path d="M8 2a6 6 0 0 0 0 12c1 0 1.3-.8.9-1.6-.5-.9.1-1.9 1.1-1.9H12a2 2 0 0 0 2-2C14 4.7 11.3 2 8 2Z"/></svg>',
  image: '<svg viewBox="0 0 16 16"><rect x="2" y="2" width="12" height="12" rx="2"/><circle cx="7" cy="7" r="2"/><path d="M14 2L2 14"/></svg>',
  sliders: '<svg viewBox="0 0 16 16"><path d="M4 4h8M4 8h8M4 12h8"/></svg>',
  search: '<svg viewBox="0 0 16 16"><circle cx="11" cy="11" r="7"/><path d="M16 16l-4-4"/></svg>',
  chevronDown: '<svg viewBox="0 0 16 16"><path d="M4 6l4 4 4-4"/></svg>',
  chevronUp: '<svg viewBox="0 0 16 16"><path d="M4 10l4-4 4 4"/></svg>',
  chevronLeft: '<svg viewBox="0 0 16 16"><path d="M10 3 5 8l5 5"/></svg>',
  chevronRight: '<svg viewBox="0 0 16 16"><path d="m6 3 5 5-5 5"/></svg>',
  refresh: '<svg viewBox="0 0 16 16"><path d="M13 8a5 5 0 1 1-1.6-3.7M13 2.5v3h-3"/></svg>',
  star: '<svg viewBox="0 0 16 16"><path d="m8 2 1.8 3.8 4.2.5-3.1 2.9.8 4.1L8 11.3l-3.7 2 .8-4.1L2 6.3l4.2-.5z"/></svg>',
  link: '<svg viewBox="0 0 16 16"><path d="M8 3l3 3-3 3M11 6l3-3-3-3"/></svg>',
};

// ---------------------------------------------------------------- persistence
const store = {
  get(k, d) { try { const v = localStorage.getItem("odyssey." + k); return v ? JSON.parse(v) : d; } catch { return d; } },
  set(k, v) { try { localStorage.setItem("odyssey." + k, JSON.stringify(v)); } catch {} },
};

// Theme presets - all 6 themes with complete color tokens
const THEME_PRESETS = {
  "night-sea": {
    name: "Night Sea",
    description: "The default nautical chart theme",
    deep: "#0a1620", sea: "#0e1c27", hull: "#142633", "hull-2": "#1a3040",
    rope: "#24404f", "rope-soft": "#1c3341", foam: "#e3ebee", mist: "#8ea4af",
    accent: "#cfa75e", "accent-ink": "#0a1620", glass: "#6bbfb0", coral: "#ef8a7a",
    colorScheme: "dark"
  },
  "abyss": {
    name: "Abyss",
    description: "Near-black blue depths",
    deep: "#04080c", sea: "#060d14", hull: "#0a141e", "hull-2": "#0e1c2a",
    rope: "#142838", "rope-soft": "#0f1f2e", foam: "#c8d4db", mist: "#6b8a96",
    accent: "#cfa75e", "accent-ink": "#04080c", glass: "#5aa89c", coral: "#e07a6a",
    colorScheme: "dark"
  },
  "chart-paper": {
    name: "Chart Paper",
    description: "Warm off-white paper with ink-blue text",
    deep: "#f5f0e8", sea: "#faf7f0", hull: "#efe8dc", "hull-2": "#e6ddd0",
    rope: "#d4c8b0", "rope-soft": "#ddd4c4", foam: "#1a2a3a", mist: "#4a5a6a",
    accent: "#b8954a", "accent-ink": "#faf7f0", glass: "#4a9b8e", coral: "#d46b5a",
    colorScheme: "light"
  },
  "dawn": {
    name: "Dawn",
    description: "Warm horizon tones",
    deep: "#1a1210", sea: "#241814", hull: "#33241e", "hull-2": "#402d24",
    rope: "#5c4436", "rope-soft": "#4a382c", foam: "#fdf6f0", mist: "#c9b8a8",
    accent: "#d4a574", "accent-ink": "#1a1210", glass: "#c8967a", coral: "#e88d6a",
    colorScheme: "dark"
  },
  "fathom": {
    name: "Fathom",
    description: "Deep teal waters",
    deep: "#061416", sea: "#081c1f", hull: "#0c2a2e", "hull-2": "#10383d",
    rope: "#1a5056", "rope-soft": "#144045", foam: "#d0efe8", mist: "#7ac8b8",
    accent: "#4fd1c5", "accent-ink": "#061416", glass: "#2dd4bf", coral: "#f08a7a",
    colorScheme: "dark"
  },
  "graphite": {
    name: "Graphite",
    description: "Neutral grayscale with brass accent",
    deep: "#0d0d0d", sea: "#141414", hull: "#1e1e1e", "hull-2": "#282828",
    rope: "#3a3a3a", "rope-soft": "#2e2e2e", foam: "#e8e8e8", mist: "#9e9e9e",
    accent: "#cfa75e", "accent-ink": "#0d0d0d", glass: "#888888", coral: "#b88888",
    colorScheme: "dark"
  }
};

// Theme token keys in order
const THEME_TOKENS = ["deep", "sea", "hull", "hull-2", "rope", "rope-soft", "foam", "mist", "accent", "accent-ink", "glass", "coral"];

// Default preferences with all new appearance options
const DEFAULT_PREFS = {
  // Theme
  theme: "night-sea",
  customThemes: {},
  // Accent (legacy, kept for compatibility)
  accent: "#cfa75e",
  // Home page
  background: "compass",
  showClock: true,
  clockFormat: "24h", // "12h" | "24h"
  showGreeting: false,
  greetingName: "",
  showPinned: true,
  showSuggestions: true,
  showRecentQuestions: true,
  searchBoxStyle: "centered", // "centered" | "minimal" | "hero"
  // Wallpaper
  customWallpaper: null, // base64 data URL
  wallpaperDim: 0.4,
  wallpaperBlur: 0,
  // Pinned sites
  pinnedSites: [], // array of {id, name, url, folderId}
  pinnedFolders: [], // array of {id, name, order}
  // General
  restoreTabs: true,
  answerWeb: true,
  assistantPage: true,
  linkTarget: "new",
  searchEngine: "duckduckgo",
  // Appearance - Answer
  answerFont: "serif", // "serif" | "sans" | "mono"
  answerFontSize: 17,
  answerWidth: 760,
  // Appearance - UI
  uiDensity: "comfortable", // "comfortable" | "compact"
  cornerRadius: "default", // "sharp" | "default" | "round"
  tabStyle: "classic", // "classic" | "compact" | "pills"
  tabBarPosition: "top", // "top" | "sidebar"
  toolbarButtons: ["back", "forward", "reload", "omni", "star", "shield", "vault", "connect", "assist", "history", "settings"],
  toolbarOrder: ["back", "forward", "reload", "omni", "star", "shield", "vault", "connect", "assist", "history", "settings"],
  sidebarSide: "left", // "left" | "right"
  sidebarWidth: 400,
  uiScale: 1,
  windowOpacity: 1,
  windowBlur: 0,
  // Motion
  reduceMotion: false,
  animationSpeed: 1,
  // Privacy
  loadSiteIcons: true,
};

const SEARCH_ENGINES = {
  duckduckgo: { name: "DuckDuckGo", url: "https://duckduckgo.com/?q=" },
  google: { name: "Google", url: "https://www.google.com/search?q=" },
  brave: { name: "Brave Search", url: "https://search.brave.com/search?q=" },
  bing: { name: "Bing", url: "https://www.bing.com/search?q=" },
  startpage: { name: "Startpage", url: "https://www.startpage.com/do/search?q=" },
  ecosia: { name: "Ecosia", url: "https://www.ecosia.org/search?q=" },
};
const ACCENTS = [["#cfa75e", "Brass"], ["#6bbfb0", "Sea glass"], ["#e46f78", "Odysseus coral"], ["#a99cf0", "Lavender"], ["#86bff2", "Ice"]];

// Built-in nautical wallpapers (SVG data URLs)
const BUILTIN_WALLPAPERS = [
  { id: "compass", name: "Compass Chart", preview: "compass", type: "builtin" },
  { id: "stars", name: "Night Sky", preview: "stars", type: "builtin" },
  { id: "waves", name: "Ocean Waves", preview: "waves", type: "builtin" },
  { id: "chart-lines", name: "Chart Lines", preview: "chart-lines", type: "builtin" },
  { id: "nautical-map", name: "Nautical Map", preview: "nautical-map", type: "builtin" },
];

const state = {
  tabs: [],
  activeId: null,
  sidebarOpen: false,
  sidePane: "assistant",
  config: null,
  prefs: { ...DEFAULT_PREFS, ...store.get("prefs", {}) },
  history: store.get("history", []),
  bookmarks: store.get("bookmarks", []),
  asked: store.get("asked", []),
  closed: [],
  maximized: true,
  settingsSection: "general",
  settingsSearch: "",
  shields: null,
  side: { conv: uid(), thread: [] },
  vault: [],
  vaultAdding: false,
  // Theme editor state
  themeEditor: { editingId: null, editingTheme: null, previewTheme: null },
  // Pinned sites editor state
  pinnedEditor: { draggingId: null, editingId: null },
};
const convHandlers = new Map();
const savePrefs = () => store.set("prefs", state.prefs);

// ---------------------------------------------------------------- helpers
const activeTab = () => state.tabs.find((t) => t.id === state.activeId) || null;
const engine = () => SEARCH_ENGINES[state.prefs.searchEngine] || SEARCH_ENGINES.duckduckgo;
const searchUrl = (q) => engine().url + encodeURIComponent(q);
const baseUrl = () => (state.config && state.config.base_url) || "http://127.0.0.1:7000";

function toUrl(raw) {
  const s = raw.trim();
  if (!s) return null;
  if (/^[a-z][a-z0-9+.-]*:\/\//i.test(s)) return s;
  if (/^(about|data):/i.test(s)) return s;
  if (/\s/.test(s)) return null;
  if (/^(localhost|127\.0\.0\.1|\[::1\])(:\d+)?([/?#].*)?$/i.test(s)) return "http://" + s;
  if (/^\d{1,3}(\.\d{1,3}){3}(:\d+)?([/?#].*)?$/.test(s)) return "http://" + s;
  if (/^[^/?#\s]+\.[a-z][a-z0-9-]{1,23}(:\d+)?([/?#].*)?$/i.test(s) && !/\?$/.test(s)) return "https://" + s;
  return null;
}
function classify(raw) {
  const s = raw.trim();
  if (s.startsWith("!")) return "search";
  if (s.startsWith("?") || !s) return "ask";
  return toUrl(s) ? "go" : "ask";
}
function hostOf(url) { try { return new URL(url).hostname.replace(/^www\./, ""); } catch { return url || ""; } }
function isLocal(url) { const h = hostOf(url); return !h || /^(127\.|localhost|192\.168\.|10\.|100\.)/.test(h) || !/^https?:/.test(url || ""); }
function escapeHtml(s) {
  return String(s ?? "").replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));
}
function favicon(url, size = 16, withLetter = true) {
  const h = hostOf(url);
  const letter = withLetter ? `<span>${escapeHtml((h[0] || "•").toUpperCase())}</span>` : "";
  if (isLocal(url) || !state.prefs.loadSiteIcons) return letter;
  return `${letter}<img src="https://icons.duckduckgo.com/ip3/${encodeURIComponent(h)}.ico" width="${size}" height="${size}" alt="" loading="lazy" onerror="this.remove()">`;
}
const fmtNum = (n) => Number(n || 0).toLocaleString();
let toastTimer;
function notify(msg) {
  const t = activeTab();
  if (t && t.kind === "web") {
    const omni = $("#omni");
    omni.placeholder = msg; omni.value = "";
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => { omni.placeholder = "Ask anything or enter a website"; renderOmni(); }, 3500);
    return;
  }
  let el = $(".toast");
  if (!el) { el = document.createElement("div"); el.className = "toast"; document.body.appendChild(el); }
  el.classList.remove("toast-out");
  el.textContent = msg;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => {
    el.classList.add("toast-out");
    setTimeout(() => el.remove(), 160);
  }, 3000);
}

// Small, safe markdown renderer (escapes everything first).
function md(src) {
  const blocks = [];
  let s = src.replace(/```[\w-]*\n?([\s\S]*?)(```|$)/g, (_, code) => {
    blocks.push("<pre><code>" + escapeHtml(code.replace(/\n$/, "")) + "</code></pre>");
    return "\u0000" + (blocks.length - 1) + "\u0000";
  });
  s = escapeHtml(s);
  const inline = (t) => t
    .replace(/`([^`]+)`/g, "<code>$1</code>")
    .replace(/\*\*([^*]+)\*\*/g, "<b>$1</b>")
    .replace(/(^|[^*])\*([^*\n]+)\*/g, "$1<i>$2</i>")
    .replace(/\[([^\]]+)\]\((https?:\/\/[^)\s]+)\)/g, '<a data-href="$2">$1</a>')
    .replace(/(^|[\s(])(https?:\/\/[^\s<)]+)/g, '$1<a data-href="$2">$2</a>');
  const out = [];
  let list = null;
  const closeList = () => { if (list) { out.push(`</${list}>`); list = null; } };
  for (const line of s.split("\n")) {
    let m;
    if (/^\u0000\d+\u0000$/.test(line.trim())) { closeList(); out.push(line.trim()); continue; }
    if ((m = line.match(/^(#{1,4})\s+(.*)$/))) { closeList(); out.push(`<h3>${inline(m[2])}</h3>`); continue; }
    if ((m = line.match(/^\s*[-*•]\s+(.*)$/))) { if (list !== "ul") { closeList(); out.push("<ul>"); list = "ul"; } out.push(`<li>${inline(m[1])}</li>`); continue; }
    if ((m = line.match(/^\s*\d+[.)]\s+(.*)$/))) { if (list !== "ol") { closeList(); out.push("<ol>"); list = "ol"; } out.push(`<li>${inline(m[1])}</li>`); continue; }
    if (!line.trim()) { closeList(); continue; }
    closeList();
    out.push(`<p>${inline(line)}</p>`);
  }
  closeList();
  return out.join("").replace(/\u0000(\d+)\u0000/g, (_, i) => blocks[+i]);
}
function setMode(el, mode) {
  el.className = "mode " + mode;
  el.querySelector("span").textContent = mode === "go" ? "Go" : mode === "search" ? "Search" : "Ask";
}

// ---------------------------------------------------------------- Theme System
function applyTheme(themeId, customTheme = null) {
  const root = document.documentElement;
  let theme;
  
  if (themeId === "custom" && customTheme) {
    theme = customTheme;
    root.dataset.theme = "custom";
  } else if (THEME_PRESETS[themeId]) {
    theme = THEME_PRESETS[themeId];
    root.dataset.theme = themeId;
  } else {
    theme = THEME_PRESETS["night-sea"];
    root.dataset.theme = "night-sea";
  }
  
  // Apply all theme tokens as CSS custom properties for crossfade transition
  for (const token of THEME_TOKENS) {
    root.style.setProperty(`--theme-${themeId}-${token}`, theme[token]);
    root.style.setProperty(`--${token}`, theme[token]);
  }
  // Accent-dim is computed
  root.style.setProperty("--accent-dim", `color-mix(in srgb, ${theme.accent} 18%, transparent)`);
  root.style.setProperty("--accent-ink", theme["accent-ink"]);
  root.style.setProperty("--custom-color-scheme", theme.colorScheme);
  root.style.colorScheme = theme.colorScheme;
  
  // Update accent in prefs for compatibility
  state.prefs.accent = theme.accent;
  
  // Re-render home background if on home
  renderHomeBackground();
  render();
}

function getCurrentTheme() {
  const themeId = state.prefs.theme;
  if (themeId === "custom") {
    return state.prefs.customThemes[state.prefs.customThemeId] || THEME_PRESETS["night-sea"];
  }
  return THEME_PRESETS[themeId] || THEME_PRESETS["night-sea"];
}

function initTheme() {
  applyTheme(state.prefs.theme, state.prefs.customThemes[state.prefs.customThemeId]);
  // Apply all appearance CSS variables
  applyAppearanceVariables();
}

function applyAppearanceVariables() {
  const root = document.documentElement;
  const p = state.prefs;
  
  // Answer font
  const fontMap = { serif: "var(--read)", sans: "var(--ui)", mono: "var(--mono)" };
  root.style.setProperty("--answer-font", fontMap[p.answerFont] || "var(--read)");
  root.style.setProperty("--answer-font-size", `${p.answerFontSize}px`);
  root.style.setProperty("--answer-width", `${p.answerWidth}px`);
  
  // UI density
  root.style.setProperty("--ui-density", p.uiDensity);
  
  // Corner radius
  root.style.setProperty("--corner-radius", p.cornerRadius);
  
  // Tab style
  root.style.setProperty("--tab-style", p.tabStyle);
  root.dataset.tabStyle = p.tabStyle;
  
  // Tab bar position
  root.dataset.tabBarPosition = p.tabBarPosition;
  
  // Toolbar buttons visibility
  updateToolbarVisibility();
  
  // Sidebar
  root.dataset.sidebarSide = p.sidebarSide;
  root.style.setProperty("--sidebar-width", `${p.sidebarWidth}px`);
  
  // UI scale
  root.style.setProperty("--ui-scale", p.uiScale);
  
  // Window opacity/blur
  root.style.setProperty("--window-opacity", p.windowOpacity);
  root.style.setProperty("--window-blur", `${p.windowBlur}px`);
  if (p.windowBlur > 0 || p.windowOpacity < 1) {
    root.dataset.windowBlur = "true";
  } else {
    root.dataset.windowBlur = "false";
  }
  
  // Motion
  root.style.setProperty("--animation-speed", p.animationSpeed);
  root.dataset.reduceMotion = p.reduceMotion ? "true" : "false";
  
  // Home page clock format
  tickClock();
}

function updateToolbarVisibility() {
  const buttons = ["back", "forward", "reload", "omni", "star", "shield", "vault", "connect", "assist", "history", "settings"];
  for (const btn of buttons) {
    const el = $(`#${btn}-btn`) || $(`#${btn}-form`) || $(`.${btn}-btn`);
    if (el) {
      if (state.prefs.toolbarButtons.includes(btn)) {
        el.classList.remove("hidden-btn");
      } else {
        el.classList.add("hidden-btn");
      }
    }
  }
  // Re-order toolbar buttons
  const toolbar = $("#toolbar");
  if (toolbar) {
    const order = state.prefs.toolbarOrder;
    for (let i = 0; i < order.length; i++) {
      const btn = order[i];
      const el = $(`#${btn}-btn`) || $(`#${btn}-form`) || $(`.${btn}-btn`);
      if (el && el.parentNode === toolbar) {
        el.style.order = i;
      }
    }
  }
}

// Theme Editor Functions
function openThemeEditor(themeId = null) {
  state.settingsSection = "appearance";
  state.themeEditor.editingId = themeId;
  
  if (themeId && themeId !== "new") {
    if (THEME_PRESETS[themeId]) {
      state.themeEditor.editingTheme = { ...THEME_PRESETS[themeId], id: themeId, isPreset: true };
    } else if (state.prefs.customThemes[themeId]) {
      state.themeEditor.editingTheme = { ...state.prefs.customThemes[themeId], id: themeId, isPreset: false };
    }
  } else {
    // New custom theme based on current
    const current = getCurrentTheme();
    state.themeEditor.editingTheme = {
      ...current,
      id: uid(),
      name: "New Theme",
      description: "",
      isPreset: false
    };
    state.themeEditor.editingId = "new";
  }
  
  state.themeEditor.previewTheme = { ...state.themeEditor.editingTheme };
  renderSettings();
}

function closeThemeEditor() {
  state.themeEditor.editingId = null;
  state.themeEditor.editingTheme = null;
  state.themeEditor.previewTheme = null;
  renderSettings();
}

function updateThemeEditorPreview(token, value) {
  if (!state.themeEditor.previewTheme) return;
  state.themeEditor.previewTheme[token] = value;
  state.themeEditor.previewTheme["accent-dim"] = `color-mix(in srgb, ${value} 18%, transparent)`;
  
  // Apply to preview immediately
  const root = document.documentElement;
  root.style.setProperty(`--${token}`, value);
  if (token === "accent") {
    root.style.setProperty("--accent-dim", `color-mix(in srgb, ${value} 18%, transparent)`);
  }
}

function saveThemeEditorTheme() {
  const theme = state.themeEditor.previewTheme;
  if (!theme) return;
  
  if (state.themeEditor.editingId === "new" || !theme.isPreset) {
    // Save as custom theme
    const customThemes = { ...state.prefs.customThemes };
    customThemes[theme.id] = theme;
    state.prefs.customThemes = customThemes;
    state.prefs.customThemeId = theme.id;
    state.prefs.theme = "custom";
    savePrefs();
    applyTheme("custom", theme);
  } else {
    // Can't save over presets, create new custom instead
    const newId = uid();
    const customThemes = { ...state.prefs.customThemes };
    customThemes[newId] = { ...theme, id: newId, isPreset: false };
    state.prefs.customThemes = customThemes;
    state.prefs.customThemeId = newId;
    state.prefs.theme = "custom";
    savePrefs();
    applyTheme("custom", customThemes[newId]);
  }
  
  closeThemeEditor();
  notify(`Theme "${theme.name}" saved`);
}

function deleteCustomTheme(themeId) {
  if (THEME_PRESETS[themeId]) return; // Can't delete presets
  const customThemes = { ...state.prefs.customThemes };
  delete customThemes[themeId];
  state.prefs.customThemes = customThemes;
  if (state.prefs.customThemeId === themeId) {
    state.prefs.theme = "night-sea";
    state.prefs.customThemeId = null;
    applyTheme("night-sea");
  }
  savePrefs();
  renderSettings();
  notify("Theme deleted");
}

function exportTheme(themeId) {
  let theme;
  if (THEME_PRESETS[themeId]) {
    theme = { ...THEME_PRESETS[themeId], id: themeId, exportedAt: Date.now() };
  } else if (state.prefs.customThemes[themeId]) {
    theme = { ...state.prefs.customThemes[themeId], exportedAt: Date.now() };
  } else return;
  
  const json = JSON.stringify(theme, null, 2);
  navigator.clipboard.writeText(json).then(() => {
    notify("Theme exported to clipboard");
  }).catch(() => {
    // Fallback: download as file
    const blob = new Blob([json], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `odyssey-theme-${theme.name.toLowerCase().replace(/\s+/g, "-")}.json`;
    a.click();
    URL.revokeObjectURL(url);
    notify("Theme downloaded as file");
  });
}

function importThemeFromJson(json) {
  try {
    const theme = JSON.parse(json);
    if (!theme.id || !theme.name || !theme.deep) throw new Error("Invalid theme format");
    
    const newId = uid();
    const customThemes = { ...state.prefs.customThemes };
    customThemes[newId] = { ...theme, id: newId, isPreset: false };
    state.prefs.customThemes = customThemes;
    state.prefs.customThemeId = newId;
    state.prefs.theme = "custom";
    savePrefs();
    applyTheme("custom", customThemes[newId]);
    renderSettings();
    notify(`Theme "${theme.name}" imported`);
  } catch (e) {
    notify("Failed to import theme: " + e.message);
  }
}

function importThemeFromFile(file) {
  const reader = new FileReader();
  reader.onload = (e) => importThemeFromJson(e.target.result);
  reader.readAsText(file);
}

// ---------------------------------------------------------------- appearance
function applyAppearance() {
  applyAppearanceVariables();
  renderHomeBackground();
}
function compassSvg(animate = true) {
  const rays = [], ticks = [];
  for (let i = 0; i < 32; i++) {
    const a = (i / 32) * Math.PI * 2, r1 = i % 4 === 0 ? 120 : 300;
    rays.push(`<line x1="${(400 + Math.cos(a) * r1).toFixed(1)}" y1="${(400 + Math.sin(a) * r1).toFixed(1)}" x2="${(400 + Math.cos(a) * 380).toFixed(1)}" y2="${(400 + Math.sin(a) * 380).toFixed(1)}"/>`);
  }
  for (let i = 0; i < 120; i++) {
    const a = (i / 120) * Math.PI * 2, r2 = i % 10 === 0 ? 362 : 372;
    ticks.push(`<line x1="${(400 + Math.cos(a) * 380).toFixed(1)}" y1="${(400 + Math.sin(a) * 380).toFixed(1)}" x2="${(400 + Math.cos(a) * r2).toFixed(1)}" y2="${(400 + Math.sin(a) * r2).toFixed(1)}"/>`);
  }
  return `<svg class="chart" viewBox="0 0 800 800" aria-hidden="true">
    <g class="rings"><circle cx="400" cy="400" r="380"/><circle cx="400" cy="400" r="300"/><circle cx="400" cy="400" r="210"/><circle cx="400" cy="400" r="120"/></g>
    <g class="ticks">${ticks.join("")}</g>
    <g class="rays">${rays.join("")}</g>
    <g class="rose-group"${animate ? "" : ' style="animation:none"'}>
      <path class="rose fill-half" d="M400 70 425 375 400 400ZM730 400 425 425 400 400ZM400 730 375 425 400 400ZM70 400 375 375 400 400Z"/>
      <path class="rose" d="M400 70 425 375 730 400 425 425 400 730 375 425 70 400 375 375Z"/>
      <path class="rose small" d="M400 220 410 390 580 400 410 410 400 580 390 410 220 400 390 390Z" transform="rotate(45 400 400)"/>
      <g class="letters"><text x="400" y="44">N</text><text x="756" y="400">E</text><text x="400" y="756">S</text><text x="44" y="400">W</text></g>
    </g>
  </svg>`;
}
function starsSvg() {
  let seed = 7;
  const rnd = () => ((seed = (seed * 9301 + 49297) % 233280) / 233280);
  const dots = [];
  for (let i = 0; i < 260; i++) dots.push(`<circle cx="${(rnd() * 1600).toFixed(1)}" cy="${(rnd() * 900).toFixed(1)}" r="${(rnd() * 1.3 + 0.3).toFixed(2)}" opacity="${(rnd() * 0.6 + 0.15).toFixed(2)}"/>`);
  const p = [[1150, 150], [1230, 180], [1300, 230], [1380, 260], [1395, 340], [1500, 350], [1510, 270]];
  const lines = p.slice(1).map((q, i) => `<line x1="${p[i][0]}" y1="${p[i][1]}" x2="${q[0]}" y2="${q[1]}"/>`).join("") + `<line x1="${p[3][0]}" y1="${p[3][1]}" x2="${p[6][0]}" y2="${p[6][1]}"/>`;
  const big = p.map(([x, y]) => `<circle class="bright" cx="${x}" cy="${y}" r="2.6"/>`).join("");
  return `<svg class="stars" viewBox="0 0 1600 900" preserveAspectRatio="xMidYMid slice" style="width:100%;height:100%;left:0;top:0;transform:none">${dots.join("")}${lines}${big}</svg>`;
}
function wavesSvg() {
  return `<svg viewBox="0 0 800 400" preserveAspectRatio="none" style="width:100%;height:100%;">
    <defs>
      <linearGradient id="waveGrad" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0%" stop-color="var(--sea)"/>
        <stop offset="100%" stop-color="var(--deep)"/>
      </linearGradient>
    </defs>
    <rect width="800" height="400" fill="url(#waveGrad)"/>
    <path d="M0,200 Q200,150 400,200 T800,200 L800,400 L0,400 Z" fill="var(--hull)" opacity="0.6"/>
    <path d="M0,250 Q200,300 400,250 T800,250 L800,400 L0,400 Z" fill="var(--hull-2)" opacity="0.4"/>
    <path d="M0,300 Q200,250 400,300 T800,300 L800,400 L0,400 Z" fill="var(--rope)" opacity="0.3"/>
  </svg>`;
}
function chartLinesSvg() {
  return `<svg viewBox="0 0 800 800" style="width:100%;height:100%;">
    <rect width="800" height="800" fill="var(--sea)"/>
    <g stroke="var(--rope)" stroke-width="0.5" fill="none">
      ${Array.from({length: 40}, (_, i) => `<line x1="${i * 20}" y1="0" x2="${i * 20}" y2="800"/>`).join("")}
      ${Array.from({length: 40}, (_, i) => `<line x1="0" y1="${i * 20}" x2="800" y2="${i * 20}"/>`).join("")}
    </g>
    <g stroke="var(--accent)" stroke-width="1" fill="none" opacity="0.3">
      <path d="M0,400 Q200,200 400,400 T800,400"/>
      <path d="M0,500 Q200,600 400,500 T800,500"/>
      <path d="M0,300 Q200,400 400,300 T800,300"/>
    </g>
  </svg>`;
}
function nauticalMapSvg() {
  return `<svg viewBox="0 0 800 800" style="width:100%;height:100%;">
    <rect width="800" height="800" fill="var(--sea)"/>
    <g fill="var(--hull)" opacity="0.15">
      ${Array.from({length: 20}, () => {
        const x = Math.random() * 800;
        const y = Math.random() * 800;
        const r = 20 + Math.random() * 100;
        const pts = Array.from({length: 8}, (_, i) => {
          const a = (i / 8) * Math.PI * 2;
          return `${x + Math.cos(a) * r},${y + Math.sin(a) * r * 0.6}`;
        }).join(" ");
        return `<polygon points="${pts}"/>`;
      }).join("")}
    </g>
    <g stroke="var(--rope)" stroke-width="0.5" fill="none" opacity="0.3">
      ${Array.from({length: 10}, (_, i) => {
        const cx = 400 + (Math.random() - 0.5) * 400;
        const cy = 400 + (Math.random() - 0.5) * 400;
        const r = 50 + Math.random() * 150;
        return `<circle cx="${cx}" cy="${cy}" r="${r}"/>`;
      }).join("")}
    </g>
  </svg>`;
}
function renderHomeBackground() {
  const bg = $("#home-bg");
  const p = state.prefs;
  
  // Custom wallpaper
  if (p.customWallpaper) {
    bg.className = "home-bg custom-wallpaper";
    bg.style.backgroundImage = `url(${p.customWallpaper})`;
    bg.style.setProperty("--wallpaper-dim", p.wallpaperDim);
    bg.style.setProperty("--wallpaper-blur", `${p.wallpaperBlur}px`);
    if (p.wallpaperBlur > 0) bg.classList.add("blur");
    else bg.classList.remove("blur");
    return;
  }
  
  bg.className = "home-bg";
  bg.style.backgroundImage = "";
  bg.classList.remove("blur");
  
  // Built-in backgrounds
  switch (p.background) {
    case "compass": bg.innerHTML = compassSvg(true); break;
    case "stars": bg.innerHTML = starsSvg(); break;
    case "waves": bg.innerHTML = wavesSvg(); break;
    case "chart-lines": bg.innerHTML = chartLinesSvg(); break;
    case "nautical-map": bg.innerHTML = nauticalMapSvg(); break;
    default: bg.innerHTML = "";
  }
}
function tickClock() {
  const el = $("#clock");
  if (!state.prefs.showClock) { el.innerHTML = ""; return; }
  const d = new Date();
  const timeStr = state.prefs.clockFormat === "12h" 
    ? d.toLocaleTimeString([], { hour: "numeric", minute: "2-digit", hour12: true })
    : d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", hour12: false });
  const greeting = state.prefs.showGreeting && state.prefs.greetingName 
    ? `<div class="greeting">Good ${d.getHours() < 12 ? "morning" : d.getHours() < 18 ? "afternoon" : "evening"}, ${escapeHtml(state.prefs.greetingName)}</div>`
    : "";
  el.innerHTML = `${greeting}<b>${timeStr}</b>${d.toLocaleDateString([], { weekday: "long", day: "numeric", month: "long" })}`;
}
setInterval(tickClock, 15000);

// ---------------------------------------------------------------- layout / native webviews
function reportLayout() {
  const r = $("#content").getBoundingClientRect();
  invoke("set_layout", { rect: { x: r.left, y: r.top, w: r.width, h: r.height } }).catch(() => {});
}
new ResizeObserver(reportLayout).observe($("#content"));
window.addEventListener("resize", reportLayout);

async function syncNative() {
  const t = activeTab();
  if (t && t.kind === "web" && !t.created && t.url) {
    t.created = true;
    t.loading = true;
    try { await invoke("tab_create", { id: t.id, url: t.url }); }
    catch (e) { t.created = false; notify("Couldn't open that page: " + e); }
  }
  const showWeb = t && t.kind === "web";
  invoke("tab_activate", { id: showWeb ? t.id : null }).catch(() => {});
}

// ---------------------------------------------------------------- rendering
function tabLabel(t) {
  if (t.kind === "web") return t.title || hostOf(t.url);
  if (t.kind === "ai") return t.title || "Answer";
  if (t.kind === "settings") return "Settings";
  if (t.kind === "history") return "History";
  return "New tab";
}
function tabIcon(t) {
  if (t.kind === "ai") return ICON.spark;
  if (t.kind === "web") return favicon(t.url);
  if (t.kind === "settings") return ICON.gear;
  if (t.kind === "history") return ICON.clock;
  return ICON.home;
}
let dragTabId = null;
function renderTabs() {
  const box = $("#tabs");
  box.innerHTML = "";
  const narrow = state.tabs.length > 9;
  for (const t of state.tabs) {
    const entering = !!t._entering;
    delete t._entering;
    const el = document.createElement("div");
    el.className = ["tab", t.id === state.activeId && "active", t.kind === "ai" && "ai",
      (t.kind === "settings" || t.kind === "history") && "page", t.loading && "loading", narrow && "narrow", entering && "tab-enter"].filter(Boolean).join(" ");
    el.setAttribute("role", "tab");
    el.draggable = true;
    const label = tabLabel(t);
    el.innerHTML = `<span class="fav">${tabIcon(t)}</span><span class="title"></span><button class="close" title="Close tab (Ctrl+W)" aria-label="Close tab">${ICON.close}</button>`;
    el.querySelector(".title").textContent = label;
    el.title = label;
    el.addEventListener("mousedown", (e) => { if (e.button === 1) { e.preventDefault(); closeTab(t.id); } });
    el.addEventListener("click", (e) => { if (e.target.closest(".close")) closeTab(t.id); else activate(t.id); });
    el.addEventListener("dragstart", (e) => { dragTabId = t.id; el.classList.add("dragging"); e.dataTransfer.effectAllowed = "move"; });
    el.addEventListener("dragend", () => { dragTabId = null; renderTabs(); });
    el.addEventListener("dragover", (e) => { if (dragTabId && dragTabId !== t.id) { e.preventDefault(); el.classList.add("drop-before"); } });
    el.addEventListener("dragleave", () => el.classList.remove("drop-before"));
    el.addEventListener("drop", (e) => {
      e.preventDefault();
      const from = state.tabs.findIndex((x) => x.id === dragTabId);
      if (from < 0) return;
      const [moved] = state.tabs.splice(from, 1);
      state.tabs.splice(state.tabs.findIndex((x) => x.id === t.id), 0, moved);
      saveSession(); renderTabs();
    });
    box.appendChild(el);
  }
}

function renderOmni() {
  const t = activeTab();
  const omni = $("#omni");
  if (document.activeElement !== omni) {
    omni.value = !t ? "" : t.kind === "web" ? t.url || "" : t.kind === "ai" ? (t.thread[0] && t.thread[0].q) || "" : "";
  }
  setMode($("#omni-mode"), t && t.kind === "web" && document.activeElement !== omni ? "go" : classify(omni.value));
  const isWeb = !!(t && t.kind === "web");
  $("#star-btn").classList.toggle("hidden", !isWeb);
  $("#star-btn").classList.toggle("on", isWeb && state.bookmarks.some((b) => b.url === t.url));
  $("#shield-btn").classList.toggle("hidden", !isWeb);
  $("#vault-btn").classList.toggle("hidden", !isWeb);
  for (const id of ["#back-btn", "#fwd-btn", "#reload-btn"]) $(id).disabled = !isWeb;
  renderShieldBadge();
}

function renderShieldBadge() {
  const t = activeTab();
  const btn = $("#shield-btn");
  const sh = state.shields;
  const off = !sh || !sh.enabled || !!(t && t.kind === "web" && (sh.allowed_sites || []).includes(hostOf(t.url)));
  btn.classList.toggle("off", off);
  const n = (t && t.blocked) || 0;
  btn.classList.toggle("has", n > 0);
  $("#shield-count").textContent = n > 999 ? "999+" : String(n);
  btn.title = off ? "Shields are down for this site" : `Shields blocked ${n} ads and trackers on this page`;
}

function render() {
  const t = activeTab();
  for (const v of ["#home", "#answer", "#webview-bg", "#page"]) $(v).classList.add("hidden");
  if (!t || t.kind === "home") { $("#home").classList.remove("hidden"); renderHome(); }
  else if (t.kind === "ai") { $("#answer").classList.remove("hidden"); renderThread($("#thread"), t.thread, false); }
  else if (t.kind === "settings") { $("#page").classList.remove("hidden"); renderSettings(); }
  else if (t.kind === "history") { $("#page").classList.remove("hidden"); renderHistoryPage(); }
  else $("#webview-bg").classList.remove("hidden");
  renderTabs();
  renderOmni();
  renderSide();
  renderConnection();
  $("#assist-btn").classList.toggle("on", state.sidebarOpen && state.sidePane === "assistant");
  $("#sidebar").classList.toggle("hidden", !state.sidebarOpen);
  syncNative();
  saveSession();
}

// ---------------------------------------------------------------- home
function renderHome() {
  tickClock();
  
  // Use new pinned sites system if available, fall back to bookmarks
  const pinnedSites = state.prefs.pinnedSites && state.prefs.pinnedSites.length > 0 
    ? state.prefs.pinnedSites.filter(s => !s.folderId).slice(0, 8)
    : state.bookmarks.slice(0, 8);
  
  $("#pinned").innerHTML = state.prefs.showPinned ? pinnedSites.map((b, i) => `
    <div class="pin" data-open="${escapeHtml(b.url)}" title="${escapeHtml(b.name || b.url)}">
      <div class="pin-ic">${escapeHtml(((b.name || hostOf(b.url))[0] || "•").toUpperCase())}${isLocal(b.url) || !state.prefs.loadSiteIcons ? "" : `<img src="https://icons.duckduckgo.com/ip3/${encodeURIComponent(hostOf(b.url))}.ico" alt="" onerror="this.remove()">`}</div>
      <span>${escapeHtml(b.name || hostOf(b.url))}</span>
      <button class="unpin" data-unpin="${i}" title="Remove from pinned" aria-label="Remove">${ICON.close}</button>
    </div>`).join("") : "";
  
  const recent = state.asked.slice(0, 3);
  const defaults = ["Explain how a jet engine works", "What's new in tech this week?", "Plan a 3-day trip to Amsterdam"];
  const showSuggestions = state.prefs.showSuggestions !== false;
  const showRecentQuestions = state.prefs.showRecentQuestions !== false;
  
  let suggestHtml = "";
  if (showSuggestions) {
    suggestHtml = (recent.length ? recent : defaults).map((q) => `<button data-q="${escapeHtml(q)}">${escapeHtml(q)}</button>`).join("");
  }
  if (showRecentQuestions && recent.length) {
    suggestHtml += recent.slice(0, 3).map((q) => `<button data-q="${escapeHtml(q)}" class="recent-q">${escapeHtml(q)}</button>`).join("");
  }
  $("#suggest").innerHTML = suggestHtml;
  
  // Apply search box style
  const homeForm = $("#home-form");
  if (homeForm) {
    homeForm.classList.remove("style-centered", "style-minimal", "style-hero");
    homeForm.classList.add(`style-${state.prefs.searchBoxStyle || "centered"}`);
  }
}

// ---------------------------------------------------------------- tabs
function newTab(kind = "home", extra = {}) {
  const t = { id: uid(), kind, title: "", url: "", loading: false, conv: null, thread: [], blocked: 0, created: false, ...extra };
  t._entering = true;
  const at = state.tabs.findIndex((x) => x.id === state.activeId);
  state.tabs.splice(at >= 0 ? at + 1 : state.tabs.length, 0, t);
  state.activeId = t.id;
  render();
  if (kind === "home") setTimeout(() => $("#home-input").focus(), 30);
  return t;
}
function activate(id) { state.activeId = id; render(); }

async function closeTab(id) {
  const idx0 = state.tabs.findIndex((t) => t.id === id);
  if (idx0 < 0) return;
  const t = state.tabs[idx0];
  if (!window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
    const el = $("#tabs").children[idx0];
    if (el) { el.classList.add("tab-exit"); await new Promise((r) => setTimeout(r, 130)); }
  }
  if (t.kind === "web" && t.created) await invoke("tab_close", { id }).catch(() => {});
  if (t.conv) invoke("ai_reset", { conv: t.conv }).catch(() => {});
  if (t.kind === "web" && t.url) state.closed.push({ kind: "web", url: t.url });
  if (t.kind === "ai" && t.thread.length) state.closed.push({ kind: "ai", title: t.title, thread: t.thread.map((x) => ({ ...x, done: true })) });
  const idx = state.tabs.findIndex((x) => x.id === id);
  if (idx < 0) return;
  state.tabs.splice(idx, 1);
  if (state.activeId === id) {
    const next = state.tabs[idx] || state.tabs[idx - 1];
    state.activeId = next ? next.id : null;
  }
  if (!state.tabs.length) newTab();
  else render();
}
function reopenClosed() {
  const c = state.closed.pop();
  if (!c) return;
  if (c.kind === "web") openUrl(c.url, true);
  else newTab("ai", { title: c.title, thread: c.thread });
}
function openPage(kind) {
  const existing = state.tabs.find((t) => t.kind === kind);
  if (existing) return activate(existing.id);
  const t = activeTab();
  if (t && t.kind === "home") { t.kind = kind; render(); }
  else newTab(kind);
}

async function openUrl(url, inNewTab = false) {
  let t = activeTab();
  if (inNewTab || !t || (t.kind !== "web" && t.kind !== "home")) t = newTab("home");
  if (t.kind === "web" && t.created) {
    t.url = url; t.loading = true;
    render();
    await invoke("tab_navigate", { id: t.id, url }).catch((e) => console.error(e));
    return t;
  }
  Object.assign(t, { kind: "web", url, title: hostOf(url), loading: true, created: false, blocked: 0 });
  render();
  return t;
}

function go(raw) {
  const s = raw.trim();
  if (!s) return;
  if (s.startsWith("!")) return openUrl(searchUrl(s.slice(1).trim()));
  if (s.startsWith("?")) return ask(s.slice(1).trim());
  const url = toUrl(s);
  return url ? openUrl(url) : ask(s);
}

// ---------------------------------------------------------------- session
let saveTimer;
function snapshot() {
  const tabs = state.tabs
    .filter((t) => t.kind !== "home" || state.tabs.length === 1)
    .map((t) => t.kind === "web" ? { kind: "web", url: t.url, title: t.title }
      : t.kind === "ai" ? { kind: "ai", title: t.title, thread: t.thread.filter((x) => x.done).slice(-6).map(({ q, a, web, results, err }) => ({ q, a, web, err, results: (results || []).slice(0, 8), done: true })) }
      : { kind: t.kind });
  store.set("session", { tabs, active: state.tabs.findIndex((t) => t.id === state.activeId) });
}
function saveSession() { clearTimeout(saveTimer); saveTimer = setTimeout(snapshot, 400); }
function restoreSession() {
  const s = store.get("session", null);
  if (!state.prefs.restoreTabs || !s || !s.tabs || !s.tabs.length) return false;
  for (const x of s.tabs) {
    state.tabs.push({ id: uid(), kind: x.kind, title: x.title || "", url: x.url || "", loading: false, conv: null, thread: x.thread || [], blocked: 0, created: false });
  }
  const a = state.tabs[Math.min(Math.max(s.active, 0), state.tabs.length - 1)];
  state.activeId = (a || state.tabs[0]).id;
  return true;
}

// ---------------------------------------------------------------- history & bookmarks
function recordVisit(url, title) {
  if (!/^https?:/.test(url)) return;
  const last = state.history[0];
  if (last && last.url === url) { if (title) last.title = title; }
  else state.history.unshift({ url, title: title || hostOf(url), at: Date.now() });
  if (state.history.length > 3000) state.history.length = 3000;
  store.set("history", state.history);
}
function toggleBookmark() {
  const t = activeTab();
  if (!t || t.kind !== "web") return;
  const i = state.bookmarks.findIndex((b) => b.url === t.url);
  if (i >= 0) { state.bookmarks.splice(i, 1); notify("Removed from pinned sites"); }
  else {
    state.bookmarks.unshift({ url: t.url, title: t.title, name: (t.title || hostOf(t.url)).split(/[-–|:·]/)[0].trim().slice(0, 18) || hostOf(t.url) });
    notify("Pinned to your new tab page");
  }
  store.set("bookmarks", state.bookmarks);
  renderOmni();
}

// ---------------------------------------------------------------- answers + web results
listen("ai-event", (e) => { const h = convHandlers.get(e.payload.conv); if (h) h(e.payload.ev); });

function renderResults(item) {
  if (item.resultsLoading) return `<div class="results"><h3>From the web</h3><div class="skeleton"></div><div class="skeleton"></div><div class="skeleton"></div></div>`;
  if (!item.results || !item.results.length) return item.resultsError ? `<div class="results"><h3>From the web</h3><p class="fine">${escapeHtml(item.resultsError)}</p></div>` : "";
  const shown = item.showAll ? item.results : item.results.slice(0, 6);
  return `<div class="results"><h3>From the web</h3>${shown.map((r) => `
    <div class="result" data-href="${escapeHtml(r.url)}">
      <div class="r-fav">${favicon(r.url, 16)}</div>
      <div class="r-host">${escapeHtml(hostOf(r.url))}</div>
      <div class="r-title">${escapeHtml(r.title || r.url)}</div>
      <div class="r-snip">${escapeHtml(r.snippet || "")}</div>
    </div>`).join("")}
    ${!item.showAll && item.results.length > 6 ? `<button class="more" data-more="1">Show ${item.results.length - 6} more results</button>` : ""}
    <button class="more" data-search="${escapeHtml(item.q)}">Open in ${escapeHtml(engine().name)}</button>
  </div>`;
}

function renderThread(container, thread, compact) {
  if (!thread.length && compact) {
    container.innerHTML = `<p class="side-empty">Ask anything about the page you're on. With <b>Page</b> on, Odyssey sends the page's text along with your question.</p>`;
    return;
  }
  const prevScroll = container.scrollTop;
  const grew = thread.length > (+container.dataset.count || 0);
  container.dataset.count = thread.length;
  container.innerHTML = "";
  thread.forEach((item, idx) => {
    const turn = document.createElement("div");
    turn.className = "turn";
    let answer;
    if (item.err) answer = `<div class="err-box">${escapeHtml(item.err)}</div>`;
    else if (!item.a && !item.done) answer = `<div class="thinking">${escapeHtml(item.status || "Thinking")}</div>`;
    else answer = md(item.a);
    turn.innerHTML = compact
      ? `<h2 class="q"></h2><div class="a">${answer}</div>`
      : `<h2 class="q"></h2>
        <div class="answer-card">
          <div class="byline">${ICON.spark}<span>${item.web ? "Odysseus, with web search" : "Odysseus"}</span></div>
          <div class="a">${answer}</div>
          ${item.done && !item.err ? `<div class="meta"><button data-copy="${idx}">Copy answer</button><button data-retry="${idx}">Ask again</button></div>` : ""}
        </div>
        ${renderResults(item)}`;
    turn.querySelector(".q").textContent = item.q;
    if (!item.done && item.a) { const a = turn.querySelector(".a"); (a.lastElementChild || a).classList.add("cursor"); }
    container.appendChild(turn);
  });
  if (grew) {
    const last = container.lastElementChild;
    container.scrollTop = thread.length > 1 && last ? last.offsetTop - 16 : 0;
  } else container.scrollTop = prevScroll;
}

async function runAsk(conv, item, message, useWeb, rerender, context = null) {
  item.web = useWeb;
  convHandlers.set(conv, (ev) => {
    const type = (ev.type || "").toLowerCase();
    if (typeof ev.delta === "string" && !type) item.a += ev.delta;
    else if (/think|reason/.test(type)) item.status = "Thinking it through";
    else if (/search|tool|web/.test(type)) item.status = "Searching the web";
    else if (type === "error" || ev.error) item.err = ev.error || ev.message || "Odysseus reported an error.";
    rerender();
  });
  try { await invoke("ai_ask", { conv, message, useWeb, context }); }
  catch (e) { item.err = String(e); }
  convHandlers.delete(conv);
  item.done = true;
  if (!item.a && !item.err) item.err = "Odysseus finished without an answer. Try asking again.";
  rerender();
  saveSession();
}

async function loadResults(item, rerender) {
  item.resultsLoading = true;
  rerender();
  try { item.results = await invoke("web_search", { query: item.q }); }
  catch { item.results = []; item.resultsError = "Web results aren't available right now."; }
  item.resultsLoading = false;
  rerender();
}

function ask(q, opts = {}) {
  if (!q) return;
  let t = activeTab();
  if (!t || (t.kind !== "ai" && t.kind !== "home")) t = newTab("ai");
  if (t.kind === "home") t.kind = "ai";
  if (!t.conv) t.conv = uid();
  if (!t.title) t.title = q.length > 42 ? q.slice(0, 42) + "…" : q;
  const item = { q, a: "", status: "Thinking", done: false, err: null, results: null };
  t.thread.push(item);
  state.asked = [q, ...state.asked.filter((x) => x !== q)].slice(0, 20);
  store.set("asked", state.asked);
  render();
  const rerender = () => { if (state.activeId === t.id) renderThread($("#thread"), t.thread, false); };
  const useWeb = opts.web ?? $("#answer-web").checked;
  loadResults(item, rerender);
  runAsk(t.conv, item, q, useWeb, rerender);
}

async function askSidebar(q) {
  if (!q) return;
  const side = state.side;
  const item = { q, a: "", status: "Reading the page", done: false, err: null };
  side.thread.push(item);
  const rerender = () => renderThread($("#side-thread"), side.thread, true);
  rerender();
  let message = q, context = null;
  const t = activeTab();
  if ($("#side-page").checked && t && t.kind === "web") {
    try {
      const page = await invoke("tab_extract", { id: t.id });
      context = `Web page the user is currently viewing in their browser.\nTitle: ${page.t}\nURL: ${page.u}\n` + (page.d ? `Description: ${page.d}\n` : "") + `\n${page.x}`;
      message = `${q}\n\n(This is about the web page "${page.t}" that I have open; its text is in the provided context.)`;
    } catch { item.status = "Couldn't read the page, asking without it"; rerender(); }
  }
  item.status = "Thinking";
  rerender();
  runAsk(side.conv, item, message, false, rerender, context);
}

// ---------------------------------------------------------------- side panel (assistant / shields)
function renderSide() {
  $$(".seg button").forEach((b) => b.classList.toggle("on", b.dataset.pane === state.sidePane));
  $("#pane-assistant").classList.toggle("hidden", state.sidePane !== "assistant");
  $("#pane-shields").classList.toggle("hidden", state.sidePane !== "shields");
  $("#side-new").classList.toggle("hidden", state.sidePane !== "assistant");
  const t = activeTab();
  $("#side-context").innerHTML = t && t.kind === "web"
    ? `${favicon(t.url, 14, false)}<span>${escapeHtml(t.title || hostOf(t.url))}</span>`
    : `<span>No page open. Questions go straight to Odysseus.</span>`;
  if (state.sidePane === "shields" && state.sidebarOpen) renderShieldsPane();
}
function openSide(pane) {
  if (state.sidebarOpen && state.sidePane === pane) state.sidebarOpen = false;
  else { state.sidebarOpen = true; state.sidePane = pane; }
  if (pane === "shields") refreshShields();
  render();
  if (state.sidebarOpen && pane === "assistant") setTimeout(() => $("#side-input").focus(), 30);
}
async function refreshShields() {
  const t = activeTab();
  try { state.shields = await invoke("shields_info", { url: t && t.kind === "web" ? t.url : null, id: t ? t.id : null }); } catch {}
  renderShieldBadge();
  if (state.sidebarOpen && state.sidePane === "shields") renderShieldsPane();
  const a = activeTab();
  if (a && a.kind === "settings" && state.settingsSection === "shields") renderSettings();
}
async function refreshVault() {
  try { state.vault = await invoke("vault_list"); } catch { state.vault = []; }
  const a = activeTab();
  if (a && a.kind === "settings" && state.settingsSection === "passwords") renderSettings();
}
function renderShieldsPane() {
  const sh = state.shields;
  const t = activeTab();
  const pane = $("#pane-shields");
  if (!sh) { pane.innerHTML = `<div class="shields-pane"><p class="fine">Loading…</p></div>`; return; }
  const isWeb = !!(t && t.kind === "web");
  const host = isWeb ? hostOf(t.url) : "";
  const siteOn = isWeb && !(sh.allowed_sites || []).includes(host);
  pane.innerHTML = `<div class="shields-pane">
    <div class="big-count">${fmtNum(isWeb ? t.blocked || 0 : 0)}</div>
    <p class="big-sub">${!sh.enabled ? "Shields are turned off everywhere." : isWeb ? (siteOn ? "ads and trackers blocked on this page" : "Shields are down for this site") : "Open a website to see what Shields blocks."}</p>
    ${isWeb ? `<div class="site-row">${favicon(t.url, 18, false)}<span class="site-name">${escapeHtml(host)}</span>
      <label class="switch" title="Shields for this site"><input type="checkbox" id="site-shield" ${siteOn ? "checked" : ""} ${sh.enabled ? "" : "disabled"} /></label></div>` : ""}
    <p class="fine">${sh.ready ? `<b>${fmtNum(sh.total)}</b> blocked since Odyssey started. ${escapeHtml(sh.status)}, about ${fmtNum(sh.rules)} rules.` : escapeHtml(sh.status)}</p>
    <p class="fine">If a site looks broken, turn Shields off for it here. Odyssey remembers your choice for each site.</p>
    <button class="link-btn" id="shield-settings">Shields settings</button>
  </div>`;
  const sw = $("#site-shield");
  if (sw) sw.addEventListener("change", async () => {
    await invoke("shields_set_site", { host, on: sw.checked });
    await refreshShields();
    invoke("tab_action", { id: t.id, action: "reload" }).catch(() => {});
  });
  $("#shield-settings").addEventListener("click", () => { state.settingsSection = "shields"; openPage("settings"); });
}

// ---------------------------------------------------------------- connection
function renderConnection() {
  const connected = !!(state.config && state.config.connected);
  $("#connect-btn").classList.toggle("hidden", connected);
  const hs = $("#home-status");
  if (!connected) {
    hs.innerHTML = `Odyssey isn't connected to your Odysseus yet. <button id="home-connect">Connect it</button>`;
    hs.className = "status-line";
    $("#home-connect").onclick = () => { state.settingsSection = "ai"; openPage("settings"); };
  } else hs.textContent = "";
}
async function refreshConfig() {
  try { state.config = await invoke("config_get"); } catch { state.config = null; }
  renderConnection();
}
function findOdysseusTab() {
  let base;
  try { base = new URL(baseUrl()); } catch { return null; }
  const norm = (h) => (h === "localhost" ? "127.0.0.1" : h);
  return state.tabs.find((t) => {
    if (t.kind !== "web" || !t.created) return false;
    try { const u = new URL(t.url); return norm(u.hostname) === norm(base.hostname) && u.port === base.port; } catch { return false; }
  }) || null;
}
async function connect() {
  const t = findOdysseusTab();
  if (!t) { await openUrl(baseUrl(), true); notify("Sign in to Odysseus here if it asks, then click Connect Odysseus."); return; }
  try {
    const msg = await invoke("pair_odysseus", { id: t.id });
    await refreshConfig();
    notify(msg);
    render();
  } catch (e) { activate(t.id); notify(String(e)); }
}

// ---------------------------------------------------------------- settings page
const SECTIONS = [
  ["general", "General", ICON.home], ["appearance", "Appearance", ICON.palette], ["newtab", "New Tab", ICON.home],
  ["shields", "Shields", ICON.shield], ["ai", "Odysseus", ICON.spark], ["passwords", "Passwords", ICON.key],
  ["privacy", "Privacy", ICON.lock], ["shortcuts", "Shortcuts", ICON.keys], ["about", "About", ICON.info],
];
const row = (title, desc, control) => `<div class="row"><div class="txt"><b>${title}</b>${desc ? `<p>${desc}</p>` : ""}</div>${control}</div>`;
const toggle = (key, on) => `<label class="switch"><input type="checkbox" data-pref="${key}" ${on ? "checked" : ""}/></label>`;
const radios = (key, current, options) => `<div class="radio-set">${options.map(([v, l]) => `<button data-radio="${key}" data-val="${v}" class="${current === v ? "on" : ""}">${l}</button>`).join("")}</div>`;
const select = (key, current, options) => `<select data-select="${key}">${options.map(([v, l]) => `<option value="${v}" ${current === v ? "selected" : ""}>${l}</option>`).join("")}</select>`;
const input = (key, current, type = "text", placeholder = "") => `<input type="${type}" data-pref="${key}" value="${escapeHtml(current)}" placeholder="${escapeHtml(placeholder)}" spellcheck="false"/>`;
const range = (key, current, min, max, step) => `<input type="range" data-pref="${key}" value="${current}" min="${min}" max="${max}" step="${step}"/><span class="range-value">${current}</span>`;

function settingsPanel(sec) {
  const p = state.prefs, sh = state.shields || {}, c = state.config || {};
  
  // Theme editor mode
  if (sec === "appearance" && state.themeEditor.editingId) {
    return renderThemeEditor();
  }
  
  // Pinned sites editor mode
  if (sec === "newtab" && state.pinnedEditor.editingId) {
    return renderPinnedEditor();
  }
  
  switch (sec) {
    case "general": return `<h2>General</h2><p class="lead">How Odyssey starts, searches and opens links.</p>
      <div class="group">
        ${row("On startup", "Pick up where you left off, or start fresh.", radios("restoreTabs", p.restoreTabs, [[true, "Restore tabs"], [false, "New tab"]]))}
        ${row("Search engine", "Used for <kbd>!</kbd> searches and the “Open in” link under answers.",
          `<select data-select="searchEngine">${Object.entries(SEARCH_ENGINES).map(([k, v]) => `<option value="${k}" ${p.searchEngine === k ? "selected" : ""}>${v.name}</option>`).join("")}</select>`)}
        ${row("Open links from answers", "Where a web result or source opens when you click it.", radios("linkTarget", p.linkTarget, [["new", "New tab"], ["same", "Same tab"]]))}
      </div>`;
      
    case "appearance": return renderAppearanceSettings();
      
    case "newtab": return renderNewTabSettings();
      
    case "shields": return `<h2>Shields</h2><p class="lead">Ad and tracker blocking powered by Brave's adblock engine, using EasyList, EasyPrivacy, uBlock Origin's lists and Peter Lowe's list.</p>
      <div class="group">
        ${row("Block ads and trackers", sh.enabled === false ? "Off everywhere." : escapeHtml(sh.status || ""), `<label class="switch"><input type="checkbox" id="sh-enabled" ${sh.enabled !== false ? "checked" : ""}/></label>`)}
        ${row("Block cookie banners", "Hides “accept cookies” pop-ups using the Fanboy Cookie list.", `<label class="switch"><input type="checkbox" id="sh-cookies" ${sh.cookie_banners ? "checked" : ""}/></label>`)}
        ${row("Filter lists", sh.ready ? `About ${fmtNum(sh.rules)} rules. Lists refresh every few days on their own.` : "Not loaded yet.", `<button class="ghost-btn" id="sh-update">Update lists now</button>`)}
        ${row("Blocked since Odyssey started", "", `<b style="font-size:18px">${fmtNum(sh.total)}</b>`)}
      </div>
      <p class="group-title">Sites with Shields down</p>
      <div class="group">${(sh.allowed_sites || []).length ? `<div class="allow-list">${sh.allowed_sites.map((h) => `<div class="allow-item"><span>${escapeHtml(h)}</span><button class="ghost-btn" data-unallow="${escapeHtml(h)}">Turn Shields back on</button></div>`).join("")}</div>`
        : row("None", "Shields are up on every site.", "")}</div>`;
      
    case "ai": return `<h2>Odysseus</h2><p class="lead">Odyssey's answers and assistant come from your own Odysseus server.</p>
      <div class="group">
        ${row("Connection", c.connected ? "Odyssey has its own chat-only key, stored on this computer." : "Not connected yet.", `<span class="conn ${c.connected ? "on" : ""}">${c.connected ? "Connected" : "Not connected"}</span>`)}
        <div class="row"><span class="steps-num">1</span><div class="txt"><b>Sign in to Odysseus</b><p>Opens Odysseus in a tab. Sign in there if it asks.</p></div><button class="ghost-btn" id="cfg-open">Open Odysseus</button></div>
        <div class="row"><span class="steps-num">2</span><div class="txt"><b>Connect</b><p>With the Odysseus tab open, Odyssey creates its own key and keeps it private.</p></div><button class="primary-btn" id="cfg-connect">Connect</button></div>
      </div>
      <div class="group">
        ${row("Answers search the web", "Default for the Web switch on answer pages.", toggle("answerWeb", p.answerWeb))}
        ${row("Assistant reads the page", "Default for the Page switch in the assistant.", toggle("assistantPage", p.assistantPage))}
      </div>
      <p class="group-title">Advanced</p>
      <div class="group">
        ${row("Odysseus address", "", `<input type="text" id="cfg-base" value="${escapeHtml(c.base_url || "")}" spellcheck="false"/>`)}
        ${row("Model", "Must match a model Odysseus lists.", `<input type="text" id="cfg-model" value="${escapeHtml(c.model || "")}" spellcheck="false"/>`)}
        ${row("Endpoint ID", "Which model server Odysseus should use.", `<input type="text" id="cfg-endpoint" value="${escapeHtml(c.endpoint_id || "")}" spellcheck="false"/>`)}
        <div class="row"><button class="danger-btn" id="cfg-disconnect">Disconnect</button><span class="spacer"></span><button class="ghost-btn" id="cfg-test">Test connection</button><button class="primary-btn" id="cfg-save">Save</button></div>
      </div>`;
      
    case "passwords": {
      const entries = state.vault || [];
      const rows = entries.length ? entries.map((e) => `
        <div class="row pw-row">
          <div class="txt"><b>${escapeHtml(e.origin)}</b><p>${e.username ? escapeHtml(e.username) : "No username saved"}</p></div>
          <span class="pw-value" data-pw="${e.id}">••••••••</span>
          <button class="ghost-btn" data-reveal="${e.id}">Show</button>
          <button class="ghost-btn" data-copy="${e.id}">Copy</button>
          <button class="danger-btn" data-del="${e.id}">Delete</button>
        </div>`).join("") : row("No saved passwords yet", "Passwords you save are encrypted with Windows' own data protection, tied to your Windows login. Only you, signed in on this PC, can unlock them.", "");
      return `<h2>Passwords</h2><p class="lead">Encrypted with Windows DPAPI, tied to your Windows account — there's no separate master password to remember or lose.</p>
        <div class="group">${rows}</div>
        <div class="group">${state.vaultAdding ? `
          <div class="row"><input type="text" id="pw-origin" placeholder="Site, e.g. github.com" spellcheck="false"/></div>
          <div class="row"><input type="text" id="pw-user" placeholder="Username or email" spellcheck="false"/></div>
          <div class="row"><input type="password" id="pw-pass" placeholder="Password" spellcheck="false"/></div>
          <div class="row"><button class="ghost-btn" id="pw-cancel">Cancel</button><span class="spacer"></span><button class="primary-btn" id="pw-save">Save password</button></div>
        ` : `<div class="row"><div class="txt"><b>Add a password</b><p>Save a login so Odyssey can fill it in for you later.</p></div><button class="primary-btn" id="pw-add">Add password</button></div>`}</div>`;
    }
      
    case "privacy": return renderPrivacySettings();
      
    case "shortcuts": return `<h2>Shortcuts</h2><p class="lead">These work everywhere in Odyssey, including inside web pages.</p>
      <div class="group"><table class="keys">${[
        ["New tab", ["Ctrl+T"]], ["Close tab", ["Ctrl+W"]], ["Reopen closed tab", ["Ctrl+Shift+T"]], ["Next or previous tab", ["Ctrl+Tab", "Ctrl+Shift+Tab"]],
        ["Focus the address bar", ["Ctrl+L"]], ["Assistant", ["Ctrl+J"]], ["Pin page", ["Ctrl+D"]], ["History", ["Ctrl+H"]], ["Settings", ["Ctrl+,"]],
        ["Reload", ["F5", "Ctrl+R"]], ["Back or forward", ["Alt+Left", "Alt+Right"]], ["Zoom in, out, reset", ["Ctrl+=", "Ctrl+-", "Ctrl+0"]], ["Print", ["Ctrl+P"]], ["Developer tools", ["F12"]],
      ].map(([a, ks]) => `<tr><td>${a}</td><td>${ks.map((k) => `<kbd>${k}</kbd>`).join(" ")}</td></tr>`).join("")}</table></div>`;
      
    case "about": return `<h2>About</h2>
      <div class="about-mark"><svg viewBox="0 0 24 24" class="sail" style="width:56px;height:56px"><path class="s1" d="M11.4 3 5.2 15h6.2z"/><path class="s2" d="M12.2 6.2V15h5.4z"/><path class="wave" d="M3.5 18.2c2.4-1.6 4.6-1.6 7 0s4.6 1.6 7 0c.9-.6 1.8-1 2.7-1.1"/></svg>
        <div><b style="font-size:20px">Odyssey 0.2</b><p class="lead" style="margin:4px 0 0">An AI-first browser for your own Odysseus.</p></div></div>
      <div class="group">
        ${row("Engine", "Microsoft Edge WebView2, through Tauri.", "")}
        ${row("Shields", "Brave's adblock-rust with EasyList, EasyPrivacy, uBlock Origin filters and Peter Lowe's list.", "")}
        ${row("AI", "Your Odysseus server, running on your own hardware.", "")}
      </div>`;
  }
  return "";
}

function renderAppearanceSettings() {
  const p = state.prefs;
  const theme = getCurrentTheme();
  const isCustom = p.theme === "custom";
  
  return `<h2>Appearance</h2><p class="lead">Make Odyssey yours.</p>
  
  <!-- Theme Selector -->
  <p class="group-title">Theme</p>
  <div class="group">
    <div class="theme-grid">
      ${Object.entries(THEME_PRESETS).map(([id, t]) => `
        <button class="theme-card ${p.theme === id ? "on" : ""}" data-theme="${id}" title="${t.description}">
          <div class="theme-preview" style="
            --deep: ${t.deep}; --sea: ${t.sea}; --hull: ${t.hull}; --hull-2: ${t["hull-2"]};
            --rope: ${t.rope}; --foam: ${t.foam}; --accent: ${t.accent}; --glass: ${t.glass}; --coral: ${t.coral};
          "></div>
          <span class="theme-name">${t.name}</span>
          <span class="theme-desc">${t.description}</span>
        </button>
      `).join("")}
      ${Object.entries(p.customThemes).map(([id, t]) => `
        <button class="theme-card ${p.theme === "custom" && p.customThemeId === id ? "on" : ""}" data-theme="custom" data-custom-id="${id}" title="${t.description || "Custom theme"}">
          <div class="theme-preview custom" style="
            --deep: ${t.deep}; --sea: ${t.sea}; --hull: ${t.hull}; --hull-2: ${t["hull-2"]};
            --rope: ${t.rope}; --foam: ${t.foam}; --accent: ${t.accent}; --glass: ${t.glass}; --coral: ${t.coral};
          "></div>
          <span class="theme-name">${t.name}</span>
          <span class="theme-desc">${t.description || "Custom theme"}</span>
          <div class="theme-actions">
            <button class="icon-btn small" data-edit-theme="${id}" title="Edit">${ICON.edit}</button>
            <button class="icon-btn small" data-delete-theme="${id}" title="Delete">${ICON.trash}</button>
            <button class="icon-btn small" data-export-theme="${id}" title="Export">${ICON.download}</button>
          </div>
        </button>
      `).join("")}
      <button class="theme-card add-theme" data-new-theme="true" title="Create new theme">
        ${ICON.plus}
        <span>New theme</span>
      </button>
    </div>
    ${isCustom ? `
      <div class="theme-editor-actions">
        <button class="primary-btn" id="theme-save-btn">Save changes</button>
        <button class="ghost-btn" id="theme-cancel-btn">Cancel</button>
        <button class="ghost-btn" id="theme-import-btn">Import JSON</button>
        <input type="file" id="theme-import-file" accept=".json" style="display:none"/>
      </div>
    ` : ""}
  </div>
  
  <!-- Accent Colour (for non-custom themes) -->
  ${!isCustom ? `
  <p class="group-title">Accent colour</p>
  <div class="group">${row(escapeHtml(ACCENTS.find(([v]) => v === p.accent)?.[1] || "Custom"), "Used for the AI, buttons and highlights.",
    `<div class="swatches">${ACCENTS.map(([v, n]) => `<button class="swatch ${p.accent === v ? "on" : ""}" data-accent="${v}" title="${n}" aria-label="${n}" style="background:${v}"></button>`).join("")}</div>`)}</div>
  ` : ""}
  
  <!-- UI Customization -->
  <p class="group-title">Answer text</p>
  <div class="group">
    ${row("Font family", "Typeface for AI answers.", select("answerFont", p.answerFont, [["serif", "Serif (Cambria/Georgia)"], ["sans", "Sans-serif (System UI)"], ["mono", "Monospace (Cascadia Code)"]]))}
    ${row("Font size", "Size of answer text in pixels.", range("answerFontSize", p.answerFontSize, 13, 24, 1))}
    ${row("Answer width", "Maximum width of the answer column.", select("answerWidth", p.answerWidth, [[640, "Narrow (640px)"], [760, "Default (760px)"], [920, "Wide (920px)"], [1100, "Extra wide (1100px)"]]))}
  </div>
  
  <p class="group-title">UI density & spacing</p>
  <div class="group">
    ${row("Density", "Compact reduces padding and heights throughout.", radios("uiDensity", p.uiDensity, [["comfortable", "Comfortable"], ["compact", "Compact"]]))}
    ${row("Corner radius", "Rounded corners on cards, buttons, inputs.", radios("cornerRadius", p.cornerRadius, [["sharp", "Sharp (0px)"], ["default", "Default (8px)"], ["round", "Round (16px)"]]))}
  </div>
  
  <p class="group-title">Tabs</p>
  <div class="group">
    ${row("Tab style", "Visual style of tabs in the tab bar.", radios("tabStyle", p.tabStyle, [["classic", "Classic"], ["compact", "Compact (icon-first)"], ["pills", "Pills"]]))}
    ${row("Tab bar position", "Top bar or vertical sidebar tabs.", radios("tabBarPosition", p.tabBarPosition, [["top", "Top (horizontal)"], ["sidebar", "Sidebar (vertical, collapsible)"]]))}
  </div>
  
  <p class="group-title">Toolbar</p>
  <div class="group">
    ${row("Button visibility", "Choose which buttons appear in the toolbar.", `<button class="ghost-btn" id="toolbar-editor-btn">Customize toolbar…</button>`)}
    ${row("Button order", "Drag to reorder (applies after save).", `<button class="ghost-btn" id="toolbar-order-btn">Reorder…</button>`)}
  </div>
  
  <p class="group-title">Sidebar</p>
  <div class="group">
    ${row("Sidebar side", "Which side the assistant/shields panel appears on.", radios("sidebarSide", p.sidebarSide, [["left", "Left"], ["right", "Right"]]))}
    ${row("Sidebar width", "Width of the sidebar panel in pixels.", range("sidebarWidth", p.sidebarWidth, 280, 600, 20))}
  </div>
  
  <p class="group-title">Scaling & window</p>
  <div class="group">
    ${row("UI scale", "Scale the entire interface.", range("uiScale", p.uiScale, 0.8, 1.5, 0.05))}
    ${row("Window opacity", "Requires WebView2 backdrop-filter support.", range("windowOpacity", p.windowOpacity, 0.3, 1, 0.05))}
    ${row("Window blur", "Background blur behind chrome and sidebar.", range("windowBlur", p.windowBlur, 0, 50, 1))}
  </div>
  
  <p class="group-title">Motion</p>
  <div class="group">
    ${row("Reduce motion", "Disables most animations (respects system preference).", toggle("reduceMotion", p.reduceMotion))}
    ${row("Animation speed", "Multiplier for all transition/animation durations.", range("animationSpeed", p.animationSpeed, 0.5, 2, 0.1))}
  </div>`;
}

function renderNewTabSettings() {
  const p = state.prefs;
  
  return `<h2>New Tab Page</h2><p class="lead">Customize your start page.</p>
  
  <p class="group-title">Background</p>
  <div class="group">
    <div class="bg-cards">
      ${[["compass", "Compass Chart", compassSvg(false)], ["stars", "Night Sky", starsSvg()], ["waves", "Ocean Waves", wavesSvg()], ["chart-lines", "Chart Lines", chartLinesSvg()], ["nautical-map", "Nautical Map", nauticalMapSvg()]]
        .map(([v, n, svg]) => `<button class="bg-card ${p.background === v ? "on" : ""}" data-bg="${v}"><div class="bg-prev">${svg}</div><span>${n}</span></button>`).join("")}
      ${p.customWallpaper ? `
        <button class="bg-card ${p.background === "custom" ? "on" : ""}" data-bg="custom">
          <div class="bg-prev" style="background-image:url(${p.customWallpaper});background-size:cover;background-position:center;"></div>
          <span>Custom wallpaper</span>
        </button>
      ` : ""}
      <button class="bg-card add-wallpaper" data-add-wallpaper="true" title="Add custom wallpaper">
        ${ICON.plus}
        <span>Add image</span>
      </button>
    </div>
    ${p.customWallpaper || p.background === "custom" ? `
      <div class="wallpaper-controls">
        ${row("Dim overlay", "Darkens the wallpaper for readability.", range("wallpaperDim", p.wallpaperDim, 0, 1, 0.05))}
        ${row("Blur", "Blurs the wallpaper behind the dim.", range("wallpaperBlur", p.wallpaperBlur, 0, 40, 1))}
        <div class="row"><button class="danger-btn" id="remove-wallpaper">Remove custom wallpaper</button></div>
      </div>
    ` : ""}
  </div>
  
  <p class="group-title">Clock & Greeting</p>
  <div class="group">
    ${row("Show clock", "", toggle("showClock", p.showClock))}
    ${row("Clock format", "12-hour or 24-hour time.", radios("clockFormat", p.clockFormat, [["12h", "12-hour (3:30 PM)"], ["24h", "24-hour (15:30)"]]))}
    ${row("Show greeting", "Displays a time-based greeting with your name.", toggle("showGreeting", p.showGreeting))}
    ${row("Your name", "Name used in the greeting.", input("greetingName", p.greetingName, "text", "Captain"))}
  </div>
  
  <p class="group-title">Content blocks</p>
  <div class="group">
    ${row("Pinned sites", "Sites you've starred in the address bar.", toggle("showPinned", p.showPinned))}
    ${row("Suggestions", "Quick prompts under the search box.", toggle("showSuggestions", p.showSuggestions))}
    ${row("Recent questions", "Your recent AI questions.", toggle("showRecentQuestions", p.showRecentQuestions))}
  </div>
  
  <p class="group-title">Search box style</p>
  <div class="group">
    ${row("Style", "Layout of the search box on the new tab page.", radios("searchBoxStyle", p.searchBoxStyle, [["centered", "Centered (default)"], ["minimal", "Minimal (smaller)"], ["hero", "Hero (large, prominent)"]]))}
  </div>
  
  <p class="group-title">Pinned sites editor</p>
  <div class="group">
    ${row("Manage pinned sites", "Rename, reorder, organize into folders.", `<button class="primary-btn" id="pinned-editor-btn">Open editor</button>`)}
  </div>`;
}

function renderPrivacySettings() {
  const p = state.prefs;
  
  return `<h2>Privacy</h2><p class="lead">Everything Odyssey stores stays on this computer.</p>
  
  <p class="group-title">Data clearing</p>
  <div class="group">
    ${row("Browsing history", `${fmtNum(state.history.length)} pages.`, `<button class="danger-btn" id="pv-history">Clear history</button>`)}
    ${row("Cookies and site data", "Signs you out of websites and clears their caches.", `<button class="danger-btn" id="pv-sitedata">Clear site data</button>`)}
    ${row("Recent questions", "The suggestions under the new tab search box.", `<button class="danger-btn" id="pv-asked">Clear</button>`)}
    ${row("Pinned sites", `${state.bookmarks.length} pinned.`, `<button class="danger-btn" id="pv-pins">Remove all</button>`)}
  </div>
  
  <p class="group-title">Network privacy</p>
  <div class="group">
    ${row("Load site icons", "Fetches favicons from icons.duckduckgo.com (reveals visited sites to DuckDuckGo). Disable for local-only fallback.", toggle("loadSiteIcons", p.loadSiteIcons))}
  </div>
  
  <p class="group-title">Settings backup</p>
  <div class="group">
    ${row("Export all settings", "Downloads a JSON file with all preferences.", `<button class="ghost-btn" id="export-settings">Export settings…</button>`)}
    ${row("Import settings", "Restores preferences from a JSON file.", `<button class="ghost-btn" id="import-settings">Import settings…</button><input type="file" id="import-settings-file" accept=".json" style="display:none"/>`)}
    ${row("Reset to defaults", "Resets all preferences to factory defaults.", `<button class="danger-btn" id="reset-settings">Reset all to defaults</button>`)}
  </div>`;
}

function renderThemeEditor() {
  const theme = state.themeEditor.previewTheme;
  const isPreset = theme.isPreset;
  
  return `<h2>${isPreset ? "Customize" : "Edit"} Theme: ${escapeHtml(theme.name)}</h2>
  <p class="lead">${isPreset ? "Create a custom theme based on this preset." : "Modify your custom theme."}</p>
  
  <div class="theme-editor">
    <div class="theme-editor-preview" id="theme-preview">
      <div class="preview-card">
        <div class="preview-header">
          <div class="preview-dots"><span></span><span></span><span></span></div>
        </div>
        <div class="preview-content">
          <div class="preview-tab-bar">
            <div class="preview-tab active">Tab 1</div>
            <div class="preview-tab">Tab 2</div>
            <div class="preview-tab">Tab 3</div>
          </div>
          <div class="preview-toolbar">
            <div class="preview-omni">Ask anything or enter a website</div>
          </div>
          <div class="preview-answer">
            <h3>Sample Answer</h3>
            <p>This is a preview of how your theme looks. The answer text uses your selected answer font and size.</p>
            <pre><code>const code = "example";</code></pre>
          </div>
        </div>
      </div>
    </div>
    
    <div class="theme-editor-controls">
      <div class="theme-editor-section">
        <h4>Basic colors</h4>
        ${THEME_TOKENS.filter(t => !["accent-ink", "accent-dim"].includes(t)).map(token => `
          <div class="theme-color-row">
            <label>${token}</label>
            <input type="color" data-theme-token="${token}" value="${theme[token]}"/>
            <input type="text" data-theme-token-text="${token}" value="${theme[token]}" placeholder="#rrggbb"/>
          </div>
        `).join("")}
      </div>
      
      <div class="theme-editor-section">
        <h4>Metadata</h4>
        <div class="theme-color-row">
          <label>Theme name</label>
          <input type="text" id="theme-name-input" value="${escapeHtml(theme.name)}" placeholder="My Theme"/>
        </div>
        <div class="theme-color-row">
          <label>Description</label>
          <input type="text" id="theme-desc-input" value="${escapeHtml(theme.description || "")}" placeholder="A beautiful theme"/>
        </div>
        <div class="theme-color-row">
          <label>Color scheme</label>
          <select id="theme-colorscheme-select">
            <option value="dark" ${theme.colorScheme === "dark" ? "selected" : ""}>Dark</option>
            <option value="light" ${theme.colorScheme === "light" ? "selected" : ""}>Light</option>
          </select>
        </div>
      </div>
      
      <div class="theme-editor-actions">
        <button class="primary-btn" id="theme-save-btn">Save theme</button>
        <button class="ghost-btn" id="theme-cancel-btn">Cancel</button>
        ${!isPreset ? `<button class="danger-btn" id="theme-delete-btn">Delete theme</button>` : ""}
        <button class="ghost-btn" id="theme-export-btn">Export JSON</button>
        <input type="file" id="theme-import-file" accept=".json" style="display:none"/>
      </div>
    </div>
  </div>`;
}

function renderPinnedEditor() {
  const p = state.prefs;
  const sites = p.pinnedSites || state.bookmarks.map((b, i) => ({ id: b.url, name: b.name || hostOf(b.url), url: b.url, folderId: null }));
  const folders = p.pinnedFolders || [];
  
  return `<h2>Pinned Sites Editor</h2>
  <p class="lead">Drag to reorder. Click to rename. Organize into folders.</p>
  
  <div class="pinned-editor">
    <div class="pinned-editor-toolbar">
      <button class="primary-btn" id="pin-add-btn">${ICON.plus} Add site</button>
      <button class="ghost-btn" id="pin-add-folder-btn">${ICON.folder} Add folder</button>
      <button class="ghost-btn" id="pin-save-btn">Save</button>
      <button class="ghost-btn" id="pin-cancel-btn">Cancel</button>
    </div>
    
    <div class="pinned-list" id="pinned-list">
      ${folders.map(folder => `
        <div class="pinned-folder" data-folder-id="${folder.id}">
          <div class="pinned-folder-header">
            ${ICON.grip}
            <input type="text" class="folder-name" value="${escapeHtml(folder.name)}" data-folder-id="${folder.id}"/>
            <button class="icon-btn small" data-delete-folder="${folder.id}" title="Delete folder">${ICON.trash}</button>
          </div>
          <div class="pinned-folder-content">
            ${sites.filter(s => s.folderId === folder.id).map(site => `
              <div class="pinned-item" data-site-id="${site.id}" draggable="true">
                ${ICON.grip}
                <div class="pin-favicon">${favicon(site.url, 24, false)}</div>
                <input type="text" class="pin-name" value="${escapeHtml(site.name)}" data-site-id="${site.id}"/>
                <span class="pin-url">${escapeHtml(site.url)}</span>
                <div class="pin-actions">
                  <button class="icon-btn small" data-move-site="${site.id}" data-move-to="unfiled" title="Move out of folder">${ICON.chevronUp}</button>
                  <button class="icon-btn small" data-delete-site="${site.id}" title="Remove">${ICON.trash}</button>
                </div>
              </div>
            `).join("")}
          </div>
        </div>
      `).join("")}
      
      <div class="pinned-folder" data-folder-id="unfiled">
        <div class="pinned-folder-header">
          ${ICON.grip}
          <strong>Unfiled</strong>
        </div>
        <div class="pinned-folder-content">
          ${sites.filter(s => !s.folderId).map(site => `
            <div class="pinned-item" data-site-id="${site.id}" draggable="true">
              ${ICON.grip}
              <div class="pin-favicon">${favicon(site.url, 24, false)}</div>
              <input type="text" class="pin-name" value="${escapeHtml(site.name)}" data-site-id="${site.id}"/>
              <span class="pin-url">${escapeHtml(site.url)}</span>
              <div class="pin-actions">
                <button class="icon-btn small" data-delete-site="${site.id}" title="Remove">${ICON.trash}</button>
              </div>
            </div>
          `).join("")}
        </div>
      </div>
    </div>
  </div>`;
}

function renderSettings() {
  const page = $("#page");
  const scroll = $(".panel", page)?.scrollTop || 0;
  page.innerHTML = `<div class="settings">
    <nav><h1>Settings</h1>${SECTIONS.map(([k, n, ic]) => `<button data-sec="${k}" class="${state.settingsSection === k ? "on" : ""}">${ic}${n}</button>`).join("")}</nav>
    <div class="panel">${settingsPanel(state.settingsSection)}</div></div>`;
  $(".panel", page).scrollTop = scroll;
  wireSettings(page);
}

function wireSettings(page) {
  $$("[data-sec]", page).forEach((b) => b.onclick = () => { state.settingsSection = b.dataset.sec; if (b.dataset.sec === "shields") refreshShields(); if (b.dataset.sec === "passwords") refreshVault(); renderSettings(); });
  
  // Checkboxes
  $$("[data-pref]", page).forEach((i) => {
    if (i.type === "checkbox") {
      i.onchange = () => { state.prefs[i.dataset.pref] = i.checked; savePrefs(); applyPrefsToUi(); };
    } else if (i.type === "range") {
      i.oninput = () => { 
        state.prefs[i.dataset.pref] = parseFloat(i.value); 
        i.nextElementSibling.textContent = i.value;
        savePrefs(); 
        applyPrefsToUi(); 
      };
      i.onchange = () => { state.prefs[i.dataset.pref] = parseFloat(i.value); savePrefs(); applyPrefsToUi(); };
    } else {
      i.onchange = () => { state.prefs[i.dataset.pref] = i.value; savePrefs(); applyPrefsToUi(); };
    }
  });
  
  // Radio buttons
  $$("[data-radio]", page).forEach((b) => b.onclick = () => {
    let v = b.dataset.val; if (v === "true") v = true; if (v === "false") v = false;
    state.prefs[b.dataset.radio] = v; savePrefs(); applyPrefsToUi(); renderSettings();
  });
  
  // Select dropdowns
  $$("[data-select]", page).forEach((s) => s.onchange = () => { state.prefs[s.dataset.select] = s.value; savePrefs(); applyPrefsToUi(); });
  
  // Accent swatches
  $$("[data-accent]", page).forEach((b) => b.onclick = () => { state.prefs.accent = b.dataset.accent; savePrefs(); applyAppearance(); renderSettings(); });
  
  // Background cards
  $$("[data-bg]", page).forEach((b) => b.onclick = () => { state.prefs.background = b.dataset.bg; savePrefs(); applyAppearance(); renderSettings(); });
  
  // Theme cards
  $$("[data-theme]", page).forEach((b) => b.onclick = () => {
    const themeId = b.dataset.theme;
    const customId = b.dataset.customId;
    if (themeId === "custom" && customId) {
      state.prefs.theme = "custom";
      state.prefs.customThemeId = customId;
      savePrefs();
      applyTheme("custom", state.prefs.customThemes[customId]);
    } else if (themeId && THEME_PRESETS[themeId]) {
      state.prefs.theme = themeId;
      state.prefs.customThemeId = null;
      savePrefs();
      applyTheme(themeId);
    }
    renderSettings();
  });
  
  // Theme editor actions
  $$("[data-new-theme]", page).forEach((b) => b.onclick = () => openThemeEditor("new"));
  $$("[data-edit-theme]", page).forEach((b) => b.onclick = (e) => { e.stopPropagation(); openThemeEditor(b.dataset.editTheme); });
  $$("[data-delete-theme]", page).forEach((b) => b.onclick = (e) => { e.stopPropagation(); deleteCustomTheme(b.dataset.deleteTheme); });
  $$("[data-export-theme]", page).forEach((b) => b.onclick = (e) => { e.stopPropagation(); exportTheme(b.dataset.exportTheme); });
  
  // Theme editor internal
  const themeNameInput = $("#theme-name-input", page);
  const themeDescInput = $("#theme-desc-input", page);
  const themeColorSchemeSelect = $("#theme-colorscheme-select", page);
  if (themeNameInput) themeNameInput.oninput = () => { if (state.themeEditor.previewTheme) state.themeEditor.previewTheme.name = themeNameInput.value; };
  if (themeDescInput) themeDescInput.oninput = () => { if (state.themeEditor.previewTheme) state.themeEditor.previewTheme.description = themeDescInput.value; };
  if (themeColorSchemeSelect) themeColorSchemeSelect.onchange = () => { if (state.themeEditor.previewTheme) state.themeEditor.previewTheme.colorScheme = themeColorSchemeSelect.value; };
  
  $$("[data-theme-token]", page).forEach((i) => {
    i.oninput = () => { updateThemeEditorPreview(i.dataset.themeToken, i.value); };
    i.onchange = () => { updateThemeEditorPreview(i.dataset.themeToken, i.value); };
  });
  $$("[data-theme-token-text]", page).forEach((i) => {
    i.onchange = () => { 
      if (/^#[0-9a-fA-F]{6}$/.test(i.value)) { 
        updateThemeEditorPreview(i.dataset.themeTokenText, i.value); 
        const colorInput = $(`[data-theme-token="${i.dataset.themeTokenText}"]`, page);
        if (colorInput) colorInput.value = i.value;
      }
    };
  });
  
  const on = (id, fn) => { const el = $("#" + id, page); if (el) el.onclick = fn; };
  const ch = (id, fn) => { const el = $("#" + id, page); if (el) el.onchange = fn; };
  
  // Theme editor buttons
  on("theme-save-btn", saveThemeEditorTheme);
  on("theme-cancel-btn", closeThemeEditor);
  on("theme-delete-btn", () => { if (state.themeEditor.editingId && state.themeEditor.editingId !== "new") deleteCustomTheme(state.themeEditor.editingId); });
  on("theme-export-btn", () => { if (state.themeEditor.previewTheme) exportTheme(state.themeEditor.previewTheme.id); });
  on("theme-import-btn", () => { $("#theme-import-file", page)?.click(); });
  $("#theme-import-file", page)?.addEventListener("change", (e) => { if (e.target.files[0]) importThemeFromFile(e.target.files[0]); });
  
  // Toolbar editor
  on("toolbar-editor-btn", () => openToolbarEditor());
  on("toolbar-order-btn", () => openToolbarOrderEditor());
  
  // New tab settings
  $$("[data-add-wallpaper]", page).forEach((b) => b.onclick = () => { $("#wallpaper-file", page)?.click(); });
  $("#wallpaper-file", page)?.addEventListener("change", (e) => handleWallpaperFile(e.target.files[0]));
  on("remove-wallpaper", () => { state.prefs.customWallpaper = null; state.prefs.background = "compass"; savePrefs(); applyAppearance(); renderSettings(); });
  
  // Pinned editor
  on("pinned-editor-btn", () => { state.settingsSection = "newtab"; state.pinnedEditor.editingId = "main"; renderSettings(); });
  on("pin-save-btn", savePinnedEditor);
  on("pin-cancel-btn", () => { state.pinnedEditor.editingId = null; renderSettings(); });
  on("pin-add-btn", addPinnedSite);
  on("pin-add-folder-btn", addPinnedFolder);
  
  // Pinned site drag and drop
  setupPinnedDragDrop(page);
  
  $$("[data-delete-site]", page).forEach((b) => b.onclick = () => deletePinnedSite(b.dataset.deleteSite));
  $$("[data-delete-folder]", page).forEach((b) => b.onclick = () => deletePinnedFolder(b.dataset.deleteFolder));
  $$("[data-move-site]", page).forEach((b) => b.onclick = () => movePinnedSite(b.dataset.moveSite, b.dataset.moveTo));
  $$(".folder-name", page).forEach((i) => i.onchange = () => updatePinnedFolderName(i.dataset.folderId, i.value));
  $$(".pin-name", page).forEach((i) => i.onchange = () => updatePinnedSiteName(i.dataset.siteId, i.value));
  
  // Shields
  $$("[data-unallow]", page).forEach((b) => b.onclick = async () => { await invoke("shields_set_site", { host: b.dataset.unallow, on: true }); refreshShields(); });
  ch("sh-enabled", async (e) => { await invoke("shields_set_enabled", { on: e.target.checked }); refreshShields(); });
  ch("sh-cookies", async (e) => { await invoke("shields_set_cookie_banners", { on: e.target.checked }); notify("Rebuilding filter lists"); refreshShields(); });
  on("sh-update", async () => { await invoke("shields_update"); notify("Updating filter lists"); setTimeout(refreshShields, 800); });
  
  // AI config
  on("cfg-open", () => openUrl(baseUrl(), true));
  on("cfg-connect", connect);
  on("cfg-test", async () => { try { notify(await invoke("ai_test")); } catch (e) { notify(String(e)); } });
  on("cfg-save", async () => {
    try {
      await invoke("config_set", { config: {
        base_url: $("#cfg-base", page).value.trim() || "http://127.0.0.1:7000",
        model: $("#cfg-model", page).value.trim() || "qwen3.5:9b",
        endpoint_id: $("#cfg-endpoint", page).value.trim(),
        home_search: engine().url,
      } });
      await refreshConfig(); notify("Saved."); renderSettings();
    } catch (e) { notify(String(e)); }
  });
  on("cfg-disconnect", async () => { await invoke("disconnect").catch(() => {}); await refreshConfig(); notify("Disconnected. Odyssey's key is deleted."); renderSettings(); });
  
  // Privacy
  on("pv-history", () => { state.history = []; store.set("history", []); notify("History cleared."); renderSettings(); });
  on("pv-sitedata", async () => { try { await invoke("clear_site_data"); notify("Cookies and site data cleared."); } catch (e) { notify(String(e)); } });
  on("pv-asked", () => { state.asked = []; store.set("asked", []); notify("Recent questions cleared."); });
  on("pv-pins", () => { state.bookmarks = []; store.set("bookmarks", []); notify("Pinned sites removed."); renderSettings(); });
  on("export-settings", exportAllSettings);
  on("import-settings", () => { $("#import-settings-file", page)?.click(); });
  $("#import-settings-file", page)?.addEventListener("change", (e) => { if (e.target.files[0]) importAllSettings(e.target.files[0]); });
  on("reset-settings", () => { if (confirm("Reset ALL settings to defaults? This cannot be undone.")) resetAllSettings(); });
  
  // Passwords
  on("pw-add", () => { state.vaultAdding = true; renderSettings(); setTimeout(() => $("#pw-origin", page)?.focus(), 0); });
  on("pw-cancel", () => { state.vaultAdding = false; renderSettings(); });
  on("pw-save", async () => {
    const origin = $("#pw-origin", page).value.trim();
    const username = $("#pw-user", page).value.trim();
    const password = $("#pw-pass", page).value;
    if (!origin || !password) { notify("Enter at least a site and a password."); return; }
    try {
      await invoke("vault_add", { origin, username, password });
      state.vaultAdding = false;
      notify("Password saved.");
      await refreshVault();
    } catch (e) { notify(String(e)); }
  });
  $$("[data-reveal]", page).forEach((b) => b.onclick = async () => {
    const id = b.dataset.reveal;
    const span = $(`[data-pw="${id}"]`, page);
    if (b.dataset.shown === "1") { span.textContent = "••••••••"; b.textContent = "Show"; b.dataset.shown = ""; return; }
    try {
      const pw = await invoke("vault_reveal", { id });
      span.textContent = pw;
      b.textContent = "Hide";
      b.dataset.shown = "1";
      setTimeout(() => { if (b.isConnected && b.dataset.shown === "1") { span.textContent = "••••••••"; b.textContent = "Show"; b.dataset.shown = ""; } }, 20000);
    } catch (e) { notify(String(e)); }
  });
  $$("[data-copy]", page).forEach((b) => b.onclick = async () => {
    try { const pw = await invoke("vault_reveal", { id: b.dataset.copy }); await navigator.clipboard.writeText(pw); notify("Password copied."); }
    catch (e) { notify(String(e)); }
  });
  $$("[data-del]", page).forEach((b) => b.onclick = async () => {
    if (b.dataset.armed !== "1") { b.dataset.armed = "1"; b.textContent = "Confirm delete"; setTimeout(() => { if (b.isConnected) { b.dataset.armed = ""; b.textContent = "Delete"; } }, 4000); return; }
    try { await invoke("vault_delete", { id: b.dataset.del }); notify("Password deleted."); await refreshVault(); } catch (e) { notify(String(e)); }
  });
}

function applyPrefsToUi() {
  $("#answer-web").checked = !!state.prefs.answerWeb;
  $("#side-page").checked = !!state.prefs.assistantPage;
  applyAppearanceVariables();
  tickClock();
  updateToolbarVisibility();
  renderHome();
}

// Toolbar editor
function openToolbarEditor() {
  const buttons = [
    { id: "back", label: "Back", icon: ICON.chevronLeft || '<svg viewBox="0 0 16 16"><path d="M10 3 5 8l5 5"/></svg>' },
    { id: "forward", label: "Forward", icon: ICON.chevronRight || '<svg viewBox="0 0 16 16"><path d="m6 3 5 5-5 5"/></svg>' },
    { id: "reload", label: "Reload", icon: ICON.refresh || '<svg viewBox="0 0 16 16"><path d="M13 8a5 5 0 1 1-1.6-3.7M13 2.5v3h-3"/></svg>' },
    { id: "omni", label: "Address bar", icon: ICON.search },
    { id: "star", label: "Bookmark", icon: ICON.star || '<svg viewBox="0 0 16 16"><path d="m8 2 1.8 3.8 4.2.5-3.1 2.9.8 4.1L8 11.3l-3.7 2 .8-4.1L2 6.3l4.2-.5z"/></svg>' },
    { id: "shield", label: "Shields", icon: ICON.shield },
    { id: "vault", label: "Passwords", icon: ICON.key },
    { id: "connect", label: "Connect", icon: ICON.link || '<svg viewBox="0 0 16 16"><path d="M8 3l3 3-3 3M11 6l3-3-3-3"/></svg>' },
    { id: "assist", label: "Assistant", icon: ICON.spark },
    { id: "history", label: "History", icon: ICON.clock },
    { id: "settings", label: "Settings", icon: ICON.gear },
  ];
  
  const html = buttons.map(btn => `
    <label class="toolbar-toggle">
      <input type="checkbox" data-toolbar-btn="${btn.id}" ${state.prefs.toolbarButtons.includes(btn.id) ? "checked" : ""}/>
      <span>${btn.icon} ${btn.label}</span>
    </label>
  `).join("");
  
  const panel = $(".panel", $("#page"));
  const originalContent = panel.innerHTML;
  panel.innerHTML = `
    <div class="toolbar-editor">
      <h3>Customize Toolbar</h3>
      <p class="lead">Toggle buttons on/off. Drag to reorder in the "Reorder" dialog.</p>
      <div class="toolbar-toggle-grid">${html}</div>
      <div class="toolbar-editor-actions">
        <button class="ghost-btn" id="toolbar-done-btn">Done</button>
      </div>
    </div>
  `;
  
  $$("[data-toolbar-btn]", panel).forEach((i) => i.onchange = () => {
    const btnId = i.dataset.toolbarBtn;
    if (i.checked) {
      if (!state.prefs.toolbarButtons.includes(btnId)) state.prefs.toolbarButtons.push(btnId);
    } else {
      state.prefs.toolbarButtons = state.prefs.toolbarButtons.filter(b => b !== btnId);
    }
    savePrefs();
    updateToolbarVisibility();
  });
  
  on("toolbar-done-btn", () => { panel.innerHTML = originalContent; wireSettings($("#page")); });
}

function openToolbarOrderEditor() {
  const buttons = state.prefs.toolbarOrder.map((id, i) => {
    const labels = { back: "Back", forward: "Forward", reload: "Reload", omni: "Address bar", star: "Bookmark", shield: "Shields", vault: "Passwords", connect: "Connect", assist: "Assistant", history: "History", settings: "Settings" };
    return `<div class="toolbar-order-item" data-btn="${id}" draggable="true">${ICON.grip} ${labels[id] || id}</div>`;
  }).join("");
  
  const panel = $(".panel", $("#page"));
  const originalContent = panel.innerHTML;
  panel.innerHTML = `
    <div class="toolbar-order-editor">
      <h3>Reorder Toolbar Buttons</h3>
      <p class="lead">Drag items to reorder. Hidden buttons won't appear.</p>
      <div class="toolbar-order-list" id="toolbar-order-list">${buttons}</div>
      <div class="toolbar-editor-actions">
        <button class="ghost-btn" id="toolbar-order-done-btn">Done</button>
      </div>
    </div>
  `;
  
  setupToolbarDragDrop($("#toolbar-order-list", panel));
  
  on("toolbar-order-done-btn", () => { panel.innerHTML = originalContent; wireSettings($("#page")); });
}

function setupToolbarDragDrop(list) {
  let dragged = null;
  $$(".toolbar-order-item", list).forEach(item => {
    item.addEventListener("dragstart", (e) => { dragged = item; item.classList.add("dragging"); e.dataTransfer.effectAllowed = "move"; });
    item.addEventListener("dragend", () => { dragged = null; $$(".toolbar-order-item", list).forEach(i => i.classList.remove("dragging")); });
    item.addEventListener("dragover", (e) => { e.preventDefault(); if (dragged && dragged !== item) { const rect = item.getBoundingClientRect(); if (e.clientY < rect.top + rect.height / 2) item.parentNode.insertBefore(dragged, item); else item.parentNode.insertBefore(dragged, item.nextSibling); } });
  });
  
  list.addEventListener("dragend", () => {
    const newOrder = $$(".toolbar-order-item", list).map(i => i.dataset.btn);
    state.prefs.toolbarOrder = newOrder;
    savePrefs();
    updateToolbarVisibility();
  });
}

// Wallpaper handling
function handleWallpaperFile(file) {
  if (!file || !file.type.startsWith("image/")) { notify("Please select an image file"); return; }
  const reader = new FileReader();
  reader.onload = (e) => {
    state.prefs.customWallpaper = e.target.result;
    state.prefs.background = "custom";
    savePrefs();
    applyAppearance();
    renderSettings();
    notify("Custom wallpaper set");
  };
  reader.readAsDataURL(file);
}

// Pinned sites editor
function savePinnedEditor() {
  const items = $$(".pinned-item", $("#pinned-list"));
  const folders = $$(".pinned-folder[data-folder-id]:not([data-folder-id='unfiled'])", $("#pinned-list"));
  
  const pinnedSites = [];
  const pinnedFolders = [];
  
  folders.forEach((folder, fi) => {
    const folderId = folder.dataset.folderId;
    const nameInput = folder.querySelector(".folder-name");
    pinnedFolders.push({ id: folderId, name: nameInput?.value || "Folder", order: fi });
    
    $$(".pinned-item", folder).forEach((item, si) => {
      const siteId = item.dataset.siteId;
      const nameInput = item.querySelector(".pin-name");
      const site = state.prefs.pinnedSites?.find(s => s.id === siteId) || state.bookmarks.find(b => b.url === siteId);
      if (site) {
        pinnedSites.push({ id: siteId, name: nameInput?.value || site.name || hostOf(siteId), url: site.url || siteId, folderId, order: si });
      }
    });
  });
  
  // Unfiled
  $$(".pinned-item", $("#pinned-list .pinned-folder[data-folder-id='unfiled']")).forEach((item, si) => {
    const siteId = item.dataset.siteId;
    const nameInput = item.querySelector(".pin-name");
    const site = state.prefs.pinnedSites?.find(s => s.id === siteId) || state.bookmarks.find(b => b.url === siteId);
    if (site) {
      pinnedSites.push({ id: siteId, name: nameInput?.value || site.name || hostOf(siteId), url: site.url || siteId, folderId: null, order: si });
    }
  });
  
  state.prefs.pinnedSites = pinnedSites;
  state.prefs.pinnedFolders = pinnedFolders;
  state.prefs.showPinned = true;
  savePrefs();
  state.pinnedEditor.editingId = null;
  renderSettings();
  notify("Pinned sites saved");
}

function addPinnedSite() {
  const url = prompt("Enter URL:");
  if (!url) return;
  const name = prompt("Enter name (optional):") || hostOf(url);
  const pinnedSites = state.prefs.pinnedSites || [];
  pinnedSites.push({ id: uid(), name, url, folderId: null, order: pinnedSites.length });
  state.prefs.pinnedSites = pinnedSites;
  renderSettings();
}

function addPinnedFolder() {
  const name = prompt("Folder name:");
  if (!name) return;
  const folders = state.prefs.pinnedFolders || [];
  const newId = uid();
  folders.push({ id: newId, name, order: folders.length });
  state.prefs.pinnedFolders = folders;
  renderSettings();
}

function deletePinnedSite(siteId) {
  state.prefs.pinnedSites = (state.prefs.pinnedSites || []).filter(s => s.id !== siteId);
  renderSettings();
}

function deletePinnedFolder(folderId) {
  state.prefs.pinnedFolders = (state.prefs.pinnedFolders || []).filter(f => f.id !== folderId);
  state.prefs.pinnedSites = (state.prefs.pinnedSites || []).map(s => s.folderId === folderId ? { ...s, folderId: null } : s);
  renderSettings();
}

function movePinnedSite(siteId, targetFolderId) {
  state.prefs.pinnedSites = (state.prefs.pinnedSites || []).map(s => s.id === siteId ? { ...s, folderId: targetFolderId === "unfiled" ? null : targetFolderId } : s);
  renderSettings();
}

function updatePinnedFolderName(folderId, name) {
  state.prefs.pinnedFolders = (state.prefs.pinnedFolders || []).map(f => f.id === folderId ? { ...f, name } : f);
}

function updatePinnedSiteName(siteId, name) {
  state.prefs.pinnedSites = (state.prefs.pinnedSites || []).map(s => s.id === siteId ? { ...s, name } : s);
}

function setupPinnedDragDrop(page) {
  const list = $("#pinned-list", page);
  if (!list) return;
  
  let dragged = null;
  $$(".pinned-item", list).forEach(item => {
    item.addEventListener("dragstart", (e) => { dragged = item; item.classList.add("dragging"); e.dataTransfer.effectAllowed = "move"; });
    item.addEventListener("dragend", () => { dragged = null; $$(".pinned-item", list).forEach(i => i.classList.remove("dragging")); });
    item.addEventListener("dragover", (e) => { 
      e.preventDefault(); 
      if (dragged && dragged !== item) { 
        const rect = item.getBoundingClientRect(); 
        if (e.clientY < rect.top + rect.height / 2) item.parentNode.insertBefore(dragged, item); 
        else item.parentNode.insertBefore(dragged, item.nextSibling); 
      } 
    });
  });
  
  // Also allow dragging between folders
  $$(".pinned-folder-content", list).forEach(folderContent => {
    folderContent.addEventListener("dragover", (e) => { e.preventDefault(); });
    folderContent.addEventListener("drop", (e) => {
      e.preventDefault();
      if (dragged) {
        const folderId = folderContent.closest(".pinned-folder")?.dataset.folderId || "unfiled";
        movePinnedSite(dragged.dataset.siteId, folderId === "unfiled" ? null : folderId);
      }
    });
  });
}

// Settings import/export
function exportAllSettings() {
  const json = JSON.stringify(state.prefs, null, 2);
  navigator.clipboard.writeText(json).then(() => {
    notify("Settings exported to clipboard");
  }).catch(() => {
    const blob = new Blob([json], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `odyssey-settings-${Date.now()}.json`;
    a.click();
    URL.revokeObjectURL(url);
    notify("Settings downloaded as file");
  });
}

function importAllSettings(file) {
  const reader = new FileReader();
  reader.onload = (e) => {
    try {
      const imported = JSON.parse(e.target.result);
      // Merge with defaults to ensure all keys exist
      state.prefs = { ...DEFAULT_PREFS, ...imported };
      savePrefs();
      initTheme();
      applyAppearanceVariables();
      renderSettings();
      notify("Settings imported successfully");
    } catch (err) {
      notify("Failed to import settings: " + err.message);
    }
  };
  reader.readAsText(file);
}

function resetAllSettings() {
  state.prefs = { ...DEFAULT_PREFS };
  savePrefs();
  initTheme();
  applyAppearanceVariables();
  renderSettings();
  notify("All settings reset to defaults");
}

// ---------------------------------------------------------------- history page
let historyFilter = "";
function renderHistoryPage() {
  const page = $("#page");
  const q = historyFilter.toLowerCase();
  const items = state.history.filter((h) => !q || (h.title || "").toLowerCase().includes(q) || h.url.toLowerCase().includes(q)).slice(0, 400);
  const days = new Map();
  const today = new Date().toDateString(), yday = new Date(Date.now() - 864e5).toDateString();
  for (const h of items) {
    const d = new Date(h.at);
    const key = d.toDateString() === today ? "Today" : d.toDateString() === yday ? "Yesterday" : d.toLocaleDateString([], { weekday: "long", day: "numeric", month: "long" });
    if (!days.has(key)) days.set(key, []);
    days.get(key).push(h);
  }
  const hadFocus = document.activeElement && document.activeElement.id === "h-search";
  page.innerHTML = `<div class="history">
    <div class="history-head"><h2>History</h2><input id="h-search" placeholder="Search history" value="${escapeHtml(historyFilter)}" spellcheck="false"/>
      <button class="danger-btn" id="h-clear">Clear history</button></div>
    ${items.length ? [...days].map(([day, rows]) => `<div class="day">${day}</div>${rows.map((h) => `
      <div class="h-row" data-open="${escapeHtml(h.url)}"><time>${new Date(h.at).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}</time>
        <span class="h-fav">${favicon(h.url, 16)}</span><span class="h-title">${escapeHtml(h.title || h.url)}</span><span class="h-host">${escapeHtml(hostOf(h.url))}</span></div>`).join("")}`).join("")
      : `<p class="empty">${q ? "Nothing matches that search." : "Pages you visit will show up here."}</p>`}
  </div>`;
  const s = $("#h-search", page);
  s.oninput = () => { historyFilter = s.value; renderHistoryPage(); };
  if (hadFocus) { s.focus(); s.setSelectionRange(s.value.length, s.value.length); }
  $("#h-clear", page).onclick = () => { state.history = []; store.set("history", []); renderHistoryPage(); };
}

// ---------------------------------------------------------------- events from Rust
listen("tab-load", (e) => {
  const { id, url, loading } = e.payload;
  const t = state.tabs.find((x) => x.id === id);
  if (!t) return;
  t.url = url; t.loading = loading;
  if (!loading) { if (!t.title) t.title = hostOf(url); recordVisit(url, t.title); saveSession(); }
  renderTabs();
  if (id === state.activeId) { renderOmni(); renderSide(); }
});
listen("tab-title", (e) => {
  const t = state.tabs.find((x) => x.id === e.payload.id);
  if (!t) return;
  t.title = e.payload.title;
  if (state.history[0] && state.history[0].url === t.url) { state.history[0].title = t.title; store.set("history", state.history); }
  renderTabs();
  if (t.id === state.activeId) renderSide();
});
listen("open-tab", (e) => openUrl(e.payload.url, true));
listen("shield-count", (e) => {
  const t = state.tabs.find((x) => x.id === e.payload.id);
  if (!t) return;
  t.blocked = e.payload.count;
  if (t.id === state.activeId) {
    renderShieldBadge();
    if (state.sidebarOpen && state.sidePane === "shields") { const el = $(".big-count"); if (el) el.textContent = fmtNum(t.blocked); }
  }
});
listen("shield-status", () => refreshShields());
listen("win-state", (e) => setMaximized(e.payload.maximized));
listen("accel", (e) => handleShortcut(e.payload));

// ---------------------------------------------------------------- window chrome
function setMaximized(m) {
  state.maximized = m;
  document.body.classList.toggle("maximized", m);
  $("#win-max").title = m ? "Restore down" : "Maximize";
  $("#win-max").innerHTML = m
    ? '<svg viewBox="0 0 12 12"><rect x="2" y="3.5" width="6.5" height="6.5" rx="1"/><path d="M4 3.5V3a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v4a1 1 0 0 1-1 1h-.5"/></svg>'
    : '<svg viewBox="0 0 12 12"><rect x="2" y="2" width="8" height="8" rx="1"/></svg>';
  requestAnimationFrame(reportLayout);
}
$("#win-min").onclick = () => invoke("win_minimize");
$("#win-max").onclick = async () => setMaximized(await invoke("win_toggle_maximize"));
$("#win-close").onclick = () => { clearTimeout(saveTimer); snapshot(); invoke("win_close"); };
$$("[data-drag]").forEach((el) => {
  el.addEventListener("mousedown", (e) => { if (e.button === 0 && e.detail === 1) invoke("win_drag"); });
  el.addEventListener("dblclick", async () => setMaximized(await invoke("win_toggle_maximize")));
});
$("#tabs").addEventListener("mousedown", (e) => { if (e.button === 0 && e.target === $("#tabs") && e.detail === 1) invoke("win_drag"); });
$$("#edges [data-edge]").forEach((el) => el.addEventListener("mousedown", (e) => { if (e.button === 0) invoke("win_resize", { edge: el.dataset.edge }); }));
window.addEventListener("beforeunload", snapshot);

// ---------------------------------------------------------------- shortcuts
const zoom = new Map();
function zoomTab(delta) {
  const t = activeTab();
  if (!t || t.kind !== "web") return;
  const z = delta === 0 ? 1 : Math.min(3, Math.max(0.3, Math.round(((zoom.get(t.id) || 1) + delta) * 10) / 10));
  zoom.set(t.id, z);
  invoke("tab_zoom", { id: t.id, factor: z });
  notify(`Zoom ${Math.round(z * 100)}%`);
}
function handleShortcut({ key, ctrl, shift, alt }) {
  const t = activeTab();
  const k = key.length === 1 ? key.toLowerCase() : key;
  const web = !!(t && t.kind === "web");
  if (ctrl && shift && k === "t") { reopenClosed(); return true; }
  if (ctrl && k === "t") { newTab(); return true; }
  if (ctrl && k === "w") { if (state.activeId) closeTab(state.activeId); return true; }
  if (ctrl && k === "l") { $("#omni").focus(); return true; }
  if (ctrl && k === "j") { openSide("assistant"); return true; }
  if (ctrl && k === "d") { toggleBookmark(); return true; }
  if (ctrl && k === "h") { openPage("history"); return true; }
  if (ctrl && k === ",") { openPage("settings"); return true; }
  if (ctrl && k === "Tab") {
    const i = state.tabs.findIndex((x) => x.id === state.activeId);
    const n = state.tabs[(i + (shift ? -1 : 1) + state.tabs.length) % state.tabs.length];
    if (n) activate(n.id);
    return true;
  }
  if ((ctrl && k === "r") || k === "F5") { if (web) invoke("tab_action", { id: t.id, action: "reload" }); return true; }
  if (alt && k === "ArrowLeft") { if (web) invoke("tab_action", { id: t.id, action: "back" }); return true; }
  if (alt && k === "ArrowRight") { if (web) invoke("tab_action", { id: t.id, action: "forward" }); return true; }
  if (ctrl && (k === "=" || k === "+")) { zoomTab(0.1); return true; }
  if (ctrl && k === "-") { zoomTab(-0.1); return true; }
  if (ctrl && k === "0") { zoomTab(0); return true; }
  if (ctrl && k === "p") { if (web) invoke("tab_print", { id: t.id }); return true; }
  if (k === "F12") { if (web) invoke("tab_devtools", { id: t.id }); return true; }
  return false;
}
document.addEventListener("keydown", (e) => {
  if (handleShortcut({ key: e.key, ctrl: e.ctrlKey, shift: e.shiftKey, alt: e.altKey })) e.preventDefault();
});

// ---------------------------------------------------------------- wiring
$("#omni").addEventListener("input", renderOmni);
$("#omni").addEventListener("focus", () => { $("#omni").select(); renderOmni(); });
$("#omni").addEventListener("blur", () => setTimeout(renderOmni, 0));
$("#omni-form").addEventListener("submit", (e) => { e.preventDefault(); const v = $("#omni").value; $("#omni").blur(); go(v); });
$("#home-input").addEventListener("input", () => setMode($("#home-mode"), classify($("#home-input").value)));
$("#home-form").addEventListener("submit", (e) => { e.preventDefault(); const v = $("#home-input").value; $("#home-input").value = ""; setMode($("#home-mode"), "ask"); go(v); });
$("#followup-form").addEventListener("submit", (e) => { e.preventDefault(); const v = $("#followup").value.trim(); $("#followup").value = ""; if (v) ask(v); });
$("#side-form").addEventListener("submit", (e) => { e.preventDefault(); const v = $("#side-input").value.trim(); $("#side-input").value = ""; askSidebar(v); });
$$(".chips button").forEach((b) => b.addEventListener("click", () => askSidebar(b.dataset.prompt)));
$$(".seg button").forEach((b) => b.addEventListener("click", () => { state.sidePane = b.dataset.pane; if (b.dataset.pane === "shields") refreshShields(); render(); }));
$("#side-new").addEventListener("click", () => { invoke("ai_reset", { conv: state.side.conv }).catch(() => {}); state.side = { conv: uid(), thread: [] }; renderThread($("#side-thread"), [], true); });
$("#assist-btn").addEventListener("click", () => openSide("assistant"));
$("#side-close").addEventListener("click", () => { state.sidebarOpen = false; render(); });
$("#shield-btn").addEventListener("click", () => openSide("shields"));
$("#star-btn").addEventListener("click", toggleBookmark);
$("#newtab-btn").addEventListener("click", () => newTab());
$("#settings-btn").addEventListener("click", () => openPage("settings"));
$("#vault-btn").addEventListener("click", async () => {
  const t = activeTab();
  if (!t || t.kind !== "web") return;
  const origin = hostOf(t.url);
  let entries = [];
  try { entries = await invoke("vault_list_for_origin", { origin }); } catch {}
  if (!entries.length) { notify(`No saved password for ${origin}. Add one in Settings \u2192 Passwords.`); return; }
  const entry = entries[0];
  try {
    const result = await invoke("vault_autofill", { id: t.id, entryId: entry.id });
    notify(result && result.filled ? "Filled your saved password." : "Couldn't find a password field on this page.");
  } catch (e) { notify(String(e)); }
});
$("#history-btn").addEventListener("click", () => openPage("history"));
$("#connect-btn").addEventListener("click", connect);
const tabAction = (action) => { const t = activeTab(); if (t && t.kind === "web") invoke("tab_action", { id: t.id, action }).catch(() => {}); };
$("#back-btn").addEventListener("click", () => tabAction("back"));
$("#fwd-btn").addEventListener("click", () => tabAction("forward"));
$("#reload-btn").addEventListener("click", () => tabAction("reload"));

document.addEventListener("click", (e) => {
  const unpin = e.target.closest("[data-unpin]");
  if (unpin) { e.stopPropagation(); state.bookmarks.splice(+unpin.dataset.unpin, 1); store.set("bookmarks", state.bookmarks); renderHome(); return; }
  const open = e.target.closest("[data-open]");
  if (open) { openUrl(open.dataset.open, !!(activeTab() && activeTab().kind === "history")); return; }
  const a = e.target.closest("[data-href]");
  if (a) { e.preventDefault(); openUrl(a.dataset.href, state.prefs.linkTarget !== "same"); return; }
  const s = e.target.closest("[data-search]");
  if (s) { openUrl(searchUrl(s.dataset.search), true); return; }
  const q = e.target.closest("[data-q]");
  if (q) { go(q.dataset.q); return; }
  const more = e.target.closest("[data-more]");
  if (more) {
    const t = activeTab(); const idx = $$(".turn", $("#thread")).indexOf(more.closest(".turn"));
    if (t && t.thread[idx]) { t.thread[idx].showAll = true; renderThread($("#thread"), t.thread, false); }
    return;
  }
  const cp = e.target.closest("[data-copy]");
  if (cp) { const t = activeTab(); const it = t && t.thread[+cp.dataset.copy]; if (it) { navigator.clipboard.writeText(it.a); cp.textContent = "Copied"; } return; }
  const rt = e.target.closest("[data-retry]");
  if (rt) { const t = activeTab(); const it = t && t.thread[+rt.dataset.retry]; if (it) ask(it.q); }
});

// ---------------------------------------------------------------- start
(async () => {
  initTheme();
  applyPrefsToUi();
  try { setMaximized(await invoke("win_is_maximized")); } catch {}
  await refreshConfig();
  refreshShields();
  if (!restoreSession()) newTab();
  render();
  renderThread($("#side-thread"), [], true);
  reportLayout();
})();
