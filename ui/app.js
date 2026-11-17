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
};

// ---------------------------------------------------------------- persistence
const store = {
  get(k, d) { try { const v = localStorage.getItem("odyssey." + k); return v ? JSON.parse(v) : d; } catch { return d; } },
  set(k, v) { try { localStorage.setItem("odyssey." + k, JSON.stringify(v)); } catch {} },
};
const DEFAULT_PREFS = {
  accent: "#cfa75e", background: "compass", showClock: true, showPinned: true, restoreTabs: true,
  answerWeb: true, assistantPage: true, linkTarget: "new", searchEngine: "duckduckgo",
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
  shields: null,
  side: { conv: uid(), thread: [] },
  vault: [],
  vaultAdding: false,
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
  if (isLocal(url)) return letter;
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
  el.textContent = msg;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => el.remove(), 3000);
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

// ---------------------------------------------------------------- appearance
function applyAppearance() {
  document.documentElement.style.setProperty("--accent", state.prefs.accent);
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
  // The Plough, drawn in the accent colour: sailors' oldest way to find north.
  const p = [[1150, 150], [1230, 180], [1300, 230], [1380, 260], [1395, 340], [1500, 350], [1510, 270]];
  const lines = p.slice(1).map((q, i) => `<line x1="${p[i][0]}" y1="${p[i][1]}" x2="${q[0]}" y2="${q[1]}"/>`).join("") + `<line x1="${p[3][0]}" y1="${p[3][1]}" x2="${p[6][0]}" y2="${p[6][1]}"/>`;
  const big = p.map(([x, y]) => `<circle class="bright" cx="${x}" cy="${y}" r="2.6"/>`).join("");
  return `<svg class="stars" viewBox="0 0 1600 900" preserveAspectRatio="xMidYMid slice" style="width:100%;height:100%;left:0;top:0;transform:none">${dots.join("")}${lines}${big}</svg>`;
}
function renderHomeBackground() {
  $("#home-bg").innerHTML = state.prefs.background === "compass" ? compassSvg(true) : state.prefs.background === "stars" ? starsSvg() : "";
}
function tickClock() {
  const el = $("#clock");
  if (!state.prefs.showClock) { el.innerHTML = ""; return; }
  const d = new Date();
  el.innerHTML = `<b>${d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}</b>${d.toLocaleDateString([], { weekday: "long", day: "numeric", month: "long" })}`;
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
    const el = document.createElement("div");
    el.className = ["tab", t.id === state.activeId && "active", t.kind === "ai" && "ai",
      (t.kind === "settings" || t.kind === "history") && "page", t.loading && "loading", narrow && "narrow"].filter(Boolean).join(" ");
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
  const pins = state.prefs.showPinned ? state.bookmarks.slice(0, 8) : [];
  $("#pinned").innerHTML = pins.map((b, i) => `
    <div class="pin" data-open="${escapeHtml(b.url)}" title="${escapeHtml(b.title || b.url)}">
      <div class="pin-ic">${escapeHtml(((b.name || hostOf(b.url))[0] || "•").toUpperCase())}${isLocal(b.url) ? "" : `<img src="https://icons.duckduckgo.com/ip3/${encodeURIComponent(hostOf(b.url))}.ico" alt="" onerror="this.remove()">`}</div>
      <span>${escapeHtml(b.name || hostOf(b.url))}</span>
      <button class="unpin" data-unpin="${i}" title="Remove from pinned" aria-label="Remove">${ICON.close}</button>
    </div>`).join("");
  const recent = state.asked.slice(0, 3);
  const defaults = ["Explain how a jet engine works", "What's new in tech this week?", "Plan a 3-day trip to Amsterdam"];
  $("#suggest").innerHTML = (recent.length ? recent : defaults).map((q) => `<button data-q="${escapeHtml(q)}">${escapeHtml(q)}</button>`).join("");
}

// ---------------------------------------------------------------- tabs
function newTab(kind = "home", extra = {}) {
  const t = { id: uid(), kind, title: "", url: "", loading: false, conv: null, thread: [], blocked: 0, created: false, ...extra };
  const at = state.tabs.findIndex((x) => x.id === state.activeId);
  state.tabs.splice(at >= 0 ? at + 1 : state.tabs.length, 0, t);
  state.activeId = t.id;
  render();
  if (kind === "home") setTimeout(() => $("#home-input").focus(), 30);
  return t;
}
function activate(id) { state.activeId = id; render(); }

async function closeTab(id) {
  const idx = state.tabs.findIndex((t) => t.id === id);
  if (idx < 0) return;
  const t = state.tabs[idx];
  if (t.kind === "web" && t.created) await invoke("tab_close", { id }).catch(() => {});
  if (t.conv) invoke("ai_reset", { conv: t.conv }).catch(() => {});
  if (t.kind === "web" && t.url) state.closed.push({ kind: "web", url: t.url });
  if (t.kind === "ai" && t.thread.length) state.closed.push({ kind: "ai", title: t.title, thread: t.thread.map((x) => ({ ...x, done: true })) });
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
  ["general", "General", ICON.home], ["appearance", "Appearance", ICON.palette], ["shields", "Shields", ICON.shield],
  ["ai", "Odysseus", ICON.spark], ["passwords", "Passwords", ICON.key], ["privacy", "Privacy", ICON.lock], ["shortcuts", "Shortcuts", ICON.keys], ["about", "About", ICON.info],
];
const row = (title, desc, control) => `<div class="row"><div class="txt"><b>${title}</b>${desc ? `<p>${desc}</p>` : ""}</div>${control}</div>`;
const toggle = (key, on) => `<label class="switch"><input type="checkbox" data-pref="${key}" ${on ? "checked" : ""}/></label>`;
const radios = (key, current, options) => `<div class="radio-set">${options.map(([v, l]) => `<button data-radio="${key}" data-val="${v}" class="${current === v ? "on" : ""}">${l}</button>`).join("")}</div>`;

function settingsPanel(sec) {
  const p = state.prefs, sh = state.shields || {}, c = state.config || {};
  switch (sec) {
    case "general": return `<h2>General</h2><p class="lead">How Odyssey starts, searches and opens links.</p>
      <div class="group">
        ${row("On startup", "Pick up where you left off, or start fresh.", radios("restoreTabs", p.restoreTabs, [[true, "Restore tabs"], [false, "New tab"]]))}
        ${row("Search engine", "Used for <kbd>!</kbd> searches and the “Open in” link under answers.",
          `<select data-select="searchEngine">${Object.entries(SEARCH_ENGINES).map(([k, v]) => `<option value="${k}" ${p.searchEngine === k ? "selected" : ""}>${v.name}</option>`).join("")}</select>`)}
        ${row("Open links from answers", "Where a web result or source opens when you click it.", radios("linkTarget", p.linkTarget, [["new", "New tab"], ["same", "Same tab"]]))}
      </div>`;
    case "appearance": return `<h2>Appearance</h2><p class="lead">Make Odyssey yours.</p>
      <p class="group-title">Accent colour</p>
      <div class="group">${row(escapeHtml(ACCENTS.find(([v]) => v === p.accent)?.[1] || "Custom"), "Used for the AI, buttons and highlights.",
        `<div class="swatches">${ACCENTS.map(([v, n]) => `<button class="swatch ${p.accent === v ? "on" : ""}" data-accent="${v}" title="${n}" aria-label="${n}" style="background:${v}"></button>`).join("")}</div>`)}</div>
      <p class="group-title">New tab page</p>
      <div class="group">
        <div class="bg-cards">${[["compass", "Compass chart", compassSvg(false)], ["stars", "Night sky", starsSvg()], ["plain", "Plain", ""]]
          .map(([v, n, svg]) => `<button class="bg-card ${p.background === v ? "on" : ""}" data-bg="${v}"><div class="bg-prev">${svg}</div><span>${n}</span></button>`).join("")}</div>
        ${row("Show the clock", "", toggle("showClock", p.showClock))}
        ${row("Show pinned sites", "Pin any page with the star in the address bar.", toggle("showPinned", p.showPinned))}
      </div>`;
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
    case "privacy": return `<h2>Privacy</h2><p class="lead">Everything Odyssey stores stays on this computer.</p>
      <div class="group">
        ${row("Browsing history", `${fmtNum(state.history.length)} pages.`, `<button class="danger-btn" id="pv-history">Clear history</button>`)}
        ${row("Cookies and site data", "Signs you out of websites and clears their caches.", `<button class="danger-btn" id="pv-sitedata">Clear site data</button>`)}
        ${row("Recent questions", "The suggestions under the new tab search box.", `<button class="danger-btn" id="pv-asked">Clear</button>`)}
        ${row("Pinned sites", `${state.bookmarks.length} pinned.`, `<button class="danger-btn" id="pv-pins">Remove all</button>`)}
      </div>`;
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
  $$("[data-pref]", page).forEach((i) => i.onchange = () => { state.prefs[i.dataset.pref] = i.checked; savePrefs(); applyPrefsToUi(); });
  $$("[data-radio]", page).forEach((b) => b.onclick = () => {
    let v = b.dataset.val; if (v === "true") v = true; if (v === "false") v = false;
    state.prefs[b.dataset.radio] = v; savePrefs(); renderSettings();
  });
  $$("[data-select]", page).forEach((s) => s.onchange = () => { state.prefs[s.dataset.select] = s.value; savePrefs(); });
  $$("[data-accent]", page).forEach((b) => b.onclick = () => { state.prefs.accent = b.dataset.accent; savePrefs(); applyAppearance(); renderSettings(); });
  $$("[data-bg]", page).forEach((b) => b.onclick = () => { state.prefs.background = b.dataset.bg; savePrefs(); applyAppearance(); renderSettings(); });
  $$("[data-unallow]", page).forEach((b) => b.onclick = async () => { await invoke("shields_set_site", { host: b.dataset.unallow, on: true }); refreshShields(); });
  const on = (id, fn) => { const el = $("#" + id, page); if (el) el.onclick = fn; };
  const ch = (id, fn) => { const el = $("#" + id, page); if (el) el.onchange = fn; };
  ch("sh-enabled", async (e) => { await invoke("shields_set_enabled", { on: e.target.checked }); refreshShields(); });
  ch("sh-cookies", async (e) => { await invoke("shields_set_cookie_banners", { on: e.target.checked }); notify("Rebuilding filter lists"); refreshShields(); });
  on("sh-update", async () => { await invoke("shields_update"); notify("Updating filter lists"); setTimeout(refreshShields, 800); });
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
  on("pv-history", () => { state.history = []; store.set("history", []); notify("History cleared."); renderSettings(); });
  on("pv-sitedata", async () => { try { await invoke("clear_site_data"); notify("Cookies and site data cleared."); } catch (e) { notify(String(e)); } });
  on("pv-asked", () => { state.asked = []; store.set("asked", []); notify("Recent questions cleared."); });
  on("pv-pins", () => { state.bookmarks = []; store.set("bookmarks", []); notify("Pinned sites removed."); renderSettings(); });
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
  tickClock();
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
  applyAppearance();
  applyPrefsToUi();
  try { setMaximized(await invoke("win_is_maximized")); } catch {}
  await refreshConfig();
  refreshShields();
  if (!restoreSession()) newTab();
  render();
  renderThread($("#side-thread"), [], true);
  reportLayout();
})();
