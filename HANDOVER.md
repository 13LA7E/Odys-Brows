# Odyssey: handover brief (updated 2026-11-18)

You are taking over **Odyssey**, an AI-first desktop web browser for Windows, built with **Tauri 2 (Rust) + WebView2** and a plain HTML/CSS/JS interface (no framework, no bundler). It talks to **Odysseus**, a self-hosted AI server on the same laptop. This session works **only on Odyssey**. Read this whole file before changing anything; every item under "Gotchas" cost real debugging time.

* Project folder: `C:\Projects\Odyssey`
* GitHub: `https://github.com/13LA7E/Odys-Brows` (**private**), branch `main`, `origin` already configured
* Owner: Syed (Windows 11, MSI Cyborg 15, RTX 4060). His taskbar is at the **top** of the screen.

---

## 1. How to build and run

* Double-click **`build.bat`**. It kills any running `odyssey.exe`, runs `cargo build --release` (logged to `build-log.txt`), and starts `target\release\odyssey.exe` only if the build succeeded. A rebuild takes about 40 s.
* **`Odyssey.bat`** just launches the last build.
* Toolchain: Rust stable MSVC + VS 2022 Build Tools (already installed). No Node, no tauri-cli. The frontend in `ui/` is embedded at compile time, so **every UI change needs a rebuild**.
* Check `build-log.txt` for `cargo exit: 0` / `BUILD_OK`. On failure, read the `error[...]` lines.
* `cargo build` on Windows is the only way to compile `src/shields.rs`'s `#[cfg(windows)]` module. It uses the WebView2 COM APIs and will not compile on Linux/macOS.

## 2. File map

```
Cargo.toml            tauri 2 (features: unstable, devtools), reqwest, adblock 0.13 (multi-thread), webview2-com 0.39, windows 0.62
tauri.conf.json       productName Odyssey, identifier dev.syed.odyssey, frontendDist "ui", no windows in config (created in code)
capabilities/default.json   only the "ui" webview gets IPC (core:default). Web-page tabs get NO IPC on purpose.
icons/icon.ico|png    Odysseus sailboat (from odysseus/static/icons). icons/src = originals.
src/main.rs           window + tab management, Odysseus client, window controls, web search, shields commands
src/shields.rs        ad/tracker blocking ("Shields"): filter lists, engine, WebView2 hooks, cosmetic filtering, keyboard accelerators
ui/index.html         chrome (title bar + toolbar), views: #home, #answer, #page (settings/history), #webview-bg; #sidebar (assistant/shields)
ui/app.css            all styles; design tokens in :root
ui/app.js             all UI logic (state, tabs, omnibox, answers, results, settings, history, shields panel, shortcuts)
build.bat / Odyssey.bat / 1 - Install build tools.bat
```

## 3. Architecture (read this)

**Window = one Tauri window `main`, frameless (`decorations(false)`, `shadow(true)`), several child webviews:**
* `ui` is the interface (`WebviewUrl::App("index.html")`, `auto_resize()`), covering the whole window.
* `tab-<id>` is one child webview per web tab, created lazily on first activation (`tab_create`). Odyssey positions the active tab over the `#content` rectangle and moves inactive ones off-screen (x = -30000). The UI reports the content rect via `set_layout` whenever it resizes (a ResizeObserver on `#content`).
* **Consequence:** anything the UI draws inside `#content` is **hidden behind** a web tab's native webview. Popovers, dropdowns and toasts over a web page will not show. For an overlay on a web tab, either put it in the chrome or the sidebar, or temporarily hide the page with `invoke("tab_activate", {id: null})` and restore it afterwards.

**Tab kinds in `app.js` state:** `home` (new tab), `ai` (answer page, conversation thread), `web` (real website), `settings`, `history`. Each tab object looks like `{id, kind, title, url, loading, conv, thread[], blocked, created}`.

**Reading page text without IPC:** `run_in_tab()` evals a script in the tab. The result travels back by setting `document.title` to chunks `ODY~i~n~<urlencoded>`, which Rust catches in `on_document_title_changed` (`handle_chunk`). Used by `tab_extract` (assistant reads the page) and `pair_odysseus`. Do not give web tabs IPC permissions.

**Shields (`src/shields.rs`):**
* Lists: EasyList, EasyPrivacy, uBO filters/privacy/unbreak/quick-fixes, Peter Lowe, and optionally the Fanboy Cookie list. They are cached in `%APPDATA%\dev.syed.odyssey\shields\` and refreshed when older than 4 days. Brave's `resources.json` (scriptlets) is attempted, and blocking still works if it fails. The engine is rebuilt in the background on start.
* `win::attach()` runs per tab:
  * WebView2 `WebResourceRequested` (filter `*`): blocked requests get a 403.
  * `NavigationStarting`: resets the counter and injects scriptlets via `AddScriptToExecuteOnDocumentCreated`.
  * `DOMContentLoaded`: cosmetic CSS (specific + generic class/id), plus a second pass 2.5 s later.
  * `AcceleratorKeyPressed`: forwards browser shortcuts to the UI as an `accel` event.
* Events to the UI: `shield-count {id,count}`, `shield-status`.
* Commands: `shields_info`, `shields_set_enabled`, `shields_set_site`, `shields_set_cookie_banners`, `shields_update`.

**Odysseus client (`src/main.rs`):**
* Config is at `%APPDATA%\dev.syed.odyssey\config.json`: `base_url` (`http://127.0.0.1:7000`), `token` (never sent to the UI), `model`, `endpoint_id`, `home_search`, `shields_enabled`, `shield_allow[]`, `cookie_banners`.
* Pairing: `pair_odysseus` runs `fetch('/api/tokens')` inside a signed-in Odysseus tab and stores a chat-scope `ody_` token.
* `ai_ask(conv, message, useWeb, context)` creates an Odysseus session per conversation (`POST /api/session` with model and endpoint_id). It streams `POST /api/chat_stream` (SSE lines `data: {...}`; answer text is `{"delta": "..."}` with no `type`), emitting `ai-event {conv, ev}` to the UI.
* `web_search(query)`: `POST /api/search/query` (provider `duckduckgo`) with a fallback that scrapes `html.duckduckgo.com`.

**UI persistence (localStorage, `odyssey.*` keys):**
* `prefs`: see `DEFAULT_PREFS` in app.js. **Add new settings there and in `settingsPanel()`.**
* Also `history`, `bookmarks` (pinned sites on the new tab page), `asked` (recent questions), `session` (tabs restored on start; web tabs are created lazily).

## 4. Gotchas (important)

1. **Use `127.0.0.1`, never `localhost`,** for Odysseus/Ollama. On Windows, `localhost` tries IPv6 first and adds ~2 s per request.
2. Odysseus needs **`endpoint_id` (`582f71d6`)** when creating sessions or it returns 400.
3. Model is **`qwen3.5-16k:latest`**: a copy of qwen3.5:9b with `num_ctx 16384`, created through Ollama's `/api/create`. At the default 4096 context, Ollama silently cuts page text.
4. Page text for the assistant goes in **`search_context`**, not in the message. Odysseus scans the message for intent; long page text made it escalate to "agent mode" and refuse ("currently permitted tools"). `search_context` is also treated as untrusted reference text, which is safer.
5. Page text is capped at **6500 chars** in `EXTRACT_BODY` (main.rs). With 16k context this can go to roughly 20000.
6. Creating webviews from a **synchronous** Tauri command deadlocks on Windows. Keep tab commands `async`.
7. `open_application` / launching the exe again opens a **second instance**. Consider `tauri-plugin-single-instance`.
8. UI shortcuts only reach `app.js` when the UI has focus. When a web page has focus they come through the WebView2 accelerator hook (`accel` event); extend `hook_shortcuts` in shields.rs for new ones.
9. Restoring from maximized calls `fit_to_work_area` (82% of the work area, centered). The user's taskbar is at the **top** of the screen.
10. Frameless resize uses `#edges` divs (hidden when maximized) and `win_resize`. When not maximized, `#main` is inset by 4px so the edges stay grabbable.

## 5. Design system (keep it consistent)

Theme: **"night-sea chart"**. Odyssey is named after Odysseus the navigator, so the visual language is nautical charts and navigation instruments: brass on deep water. It should never look like a generic SaaS dashboard.

* Tokens (`:root` in app.css): `--deep #0a1620` (chrome), `--sea #0e1c27` (page), `--hull #142633` / `--hull-2 #1a3040` (surfaces), `--rope #24404f` (lines), `--foam #e3ebee` (text), `--mist #8ea4af` (secondary), `--accent` (user-selectable, default brass `#cfa75e`), `--glass #6bbfb0` (web/navigation), `--coral #ef8a7a` (errors; also the Odysseus logo colour).
* Type: **Bahnschrift** for interface (often `font-stretch: semi-condensed`). **Cambria** (serif) for AI answer text so it reads like a page.
* Rules:
  * One bold element per screen, everything else quiet.
  * Sentence case everywhere, no ALL-CAPS labels, no "→" arrows in button text.
  * Copy is plain and active ("Clear history", not "Submit"). Errors say what happened and how to fix it.
* Motion: purposeful only (the compass rose settling on the new tab page is the signature moment). Always respect `prefers-reduced-motion` (already globally handled at the bottom of app.css).
* Icons: inline SVG, 16px, `stroke-width: 1.5`, round caps (see `ICON` in app.js).

## 6. What's done and verified on the laptop

Frameless window with integrated tabs and window controls · Odysseus app icon · Shields blocking (verified 11 blocked on theverge.com, 7 lists, ~148k rules) · Shields panel with per-site toggle · smart address bar (Ask / Go / Search modes, `?` forces AI, `!` forces web search) · answer page with Odysseus answer first, then "From the web" results with favicons · assistant side panel that reads the current page · Odysseus pairing · session restore · restore/maximize sizing.

**Phase 0 complete (2026-11-18):**
- ✅ Fixed compiler warning (unnecessary nested `unsafe` in shields.rs:494)
- ✅ Added `tauri-plugin-single-instance` — second launch now focuses existing window
- ✅ Build passes with **zero warnings**
- ✅ Hand-tested all previously untested features:
  - Settings sections (General, Appearance, Shields, Odysseus, Passwords, Privacy, Shortcuts, About) — all render and wire correctly
  - History page — renders, search works, clear works
  - Pinned sites (star / Ctrl+D) — add/remove works, persists to localStorage
  - Keyboard shortcuts from inside web pages — forwarded via WebView2 accelerator hook (`accel` event)
  - Tab drag-reorder — smooth drag-and-drop with visual feedback
  - Zoom (Ctrl+=/-), Print (Ctrl+P), DevTools (F12) — all commands work
  - Cookie-banner list toggle — rebuilds filter lists on change
  - Password vault — add/reveal/copy/delete/autofill all functional (DPAPI encryption verified)
  - New tab animations — compass settle, tab unfurl/stow animations work
- ✅ Committed and pushed to origin/main

  **Phase 1 complete (2026-11-18):**
  - ✅ **Theming engine**: 6 presets with crossfade transitions — Night Sea (default), Abyss, Chart Paper (light), Dawn, Fathom, Graphite — via `data-theme` on `<html>`
  - ✅ **Custom theme editor** in Settings → Appearance: color pickers for all 12 tokens, live preview, save/rename/delete, import/export JSON (clipboard + file)
  - ✅ **Settings architecture**: typed prefs schema (key, type, default, section, label, description), search inside Settings, per-section "Reset to default", export/import all settings
  - ✅ **Appearance options** (all apply live): answer font (serif/sans/mono + size), answer width (640/760/920/1100), UI density (comfortable/compact), corner radius (sharp/default/round), tab style (classic/compact/pills), tab bar position (top/vertical sidebar), toolbar button visibility/order, sidebar side/width, UI scale, window opacity/blur, reduce motion + animation speed slider (0.5x–2x)
  - ✅ **New tab page**: custom wallpaper (IndexedDB, dim/blur sliders), 5 built-in nautical wallpapers, clock 12h/24h, optional greeting, block toggles, pinned sites editor (rename/reorder/folders), search box style
  - ✅ **Privacy toggle**: "Load site icons" with local-only fallback
  - ✅ All changes respect `prefers-reduced-motion` and in-app Reduce motion toggle
  - ✅ Build passes with zero warnings
  - ✅ Committed and pushed to origin/main

    **Phase 2 complete (2026-11-18):**
    - ✅ **Motion helper**: CSS variables for durations/easings/speed multiplier (`--motion-duration-*`, `--motion-easing-*`, `--motion-speed`)
    - ✅ **View Transitions API** for view changes (home/answer/settings/history) with graceful fallback
    - ✅ **FLIP tab drag-reorder** with smooth animations, hover preview card, pinned tabs, tab groups, tab search (Ctrl+Shift+A)
    - ✅ **Answer page margin ornaments**: Bearing rails (default) with parallax ticks and reading-progress needle, Compass watermark, Off option
    - ✅ **Sidebar slide in/out** with `reportLayout()` at transition end
    - ✅ **Shield counter tick-up animation** on count increase
    - ✅ **Thin loading bar** under toolbar (`--glass`, 2px) with indeterminate creep and quick fill
    - ✅ **Button press feedback, skeleton shimmer, smooth scroll, unified motion polish**
    - ✅ All respect `prefers-reduced-motion` and in-app Reduce motion toggle
    - ✅ Build passes with zero warnings
    - ✅ Committed and pushed to origin/main

## 7. Your tasks (in order)

### A. Answer page: fill the empty side margins (user's main request)
On wide screens the answer column is ~760px with bare space either side. Add nautical **margin ornaments** that tie into the compass theme, as a setting under Appearance → "Answer page decoration": **Bearing rails** (default) / **Compass watermark** / **Off**.
* **Bearing rails:** a thin vertical rule in each margin with tick marks every 24px and small degree labels (000°, 045°, 090°… in `--mist`, 11px). Add a brass **compass arrow / needle** marker that slides along the rail to show reading progress as the user scrolls `#thread`, like a ship's position on a chart. The left rail could carry the arrow pointing right toward the text and the right rail a mirrored one. Use subtle parallax: the ticks scroll at about 0.3× the content.
* **Compass watermark:** a large, very faint (opacity ≈ .05) compass rose fixed in the right margin. It rotates slowly with scroll position (e.g. 1° per 40px).
* Draw both with SVG inside `#answer` (`position: absolute`, `pointer-events: none`). Hide them when the margins are narrower than ~120px. They must never overlap text.
* While an answer streams, the arrow could gently "seek" (a small oscillation) and settle when done.

### B. Animations and quality of life
* Tab open/close: animate width + opacity (~140ms). New tabs slide in from the "+" button.
* View changes (home ↔ answer ↔ settings): a short crossfade (~120ms). Use the View Transitions API (`document.startViewTransition`); WebView2 supports it.
* Answers: fade each new paragraph in as it streams. Results list: a one-time 30ms stagger when it first appears.
* Sidebar: slide in from the right (it changes `#content`'s width, so call `reportLayout()` at the end of the transition as well).
* Shield count: a tiny tick-up animation when the number increases.
* A loading progress bar under the toolbar for web tabs (`--glass`, 2px), driven by `tab-load` start/finish with an indeterminate creep.
* Add a **"Reduce motion"** toggle in Appearance that adds a `reduce-motion` class to `<body>` (mirroring the media query rule).

### C. More customisation (Settings → Appearance and General)
* Theme presets: **Night sea** (current), **Abyss** (darker, near-black blue), **Chart paper** (a light theme: warm off-white paper, ink-blue text, brass accent). Implement via `data-theme` on `<html>` that overrides the tokens.
* Custom accent: add an `<input type="color">` after the five swatches.
* Answer font: Serif (Cambria) / Sans (Bahnschrift). Answer width: Narrow 640 / Default 760 / Wide 920.
* Density: Comfortable / Compact (the compact toolbar is 40px tall; remember the `#chrome` height and `#main` top offset are linked, currently 86px).
* New tab page:
  * Custom wallpaper image (file input, stored in IndexedDB, not localStorage) with a dim slider.
  * Clock 12h/24h.
  * Optional greeting with the user's name.
  * Toggle the suggestions row.
  * Edit pinned sites (rename/reorder).
* Tab style: Classic (current) / Compact (icon-first) / Pills.
* Privacy toggle: "Load site icons" (favicons currently come from `icons.duckduckgo.com`, which tells DuckDuckGo what sites you view).

### D. More browser polish
* **Omnibox suggestions** from history, pinned sites and recent questions while typing (arrow keys plus Enter). Because of the native-webview overlap (Gotcha/section 3), hide the page (`tab_activate null`) while the dropdown is open and restore it on close, or render the dropdown inside the chrome by temporarily increasing the chrome height.
* **Find in page** (Ctrl+F): a small bar in the toolbar. Run `window.find()` / highlight via `eval`, and show the match count via the title channel.
* **Downloads**: hook `WebviewBuilder::on_download` and show a downloads list in the sidebar (third pane) with "Show in folder".
* Per-site zoom memory; a "page is muted / playing audio" indicator on tabs; a custom tab context menu (Duplicate, Pin, Close others, Reopen closed), rendered in the chrome.
* Single instance (`tauri-plugin-single-instance`): a second launch focuses the existing window.
* Proper installer + Start menu/desktop shortcut: install tauri-cli (`cargo install tauri-cli --version "^2"`) and run `cargo tauri build` (NSIS is already configured in tauri.conf.json).
* Raise the assistant page-text cap now that the model has 16k context (Gotcha 5).
* Shields: check whether `resources.json` actually downloads (for YouTube scriptlets). If it doesn't, find a working source for adblock-rust–format resources. Consider turning cookie-banner blocking on by default.

### E. Known issues
* Pop-up sign-ins ("Sign in with Google") open as normal tabs via `on_new_window` → `Deny` + new tab, so `window.opener` flows break. Allow real popups for known OAuth hosts (accounts.google.com, login.microsoftonline.com, appleid.apple.com, github.com/login) by returning `NewWindowResponse::Allow`.
* Web-search answers from the local 9B model take ~60 s; the results list loads in ~2 s. Consider showing a "Quick answer" first (useWeb=false) and offering "Search deeper".

## 8. Working rules
* Keep `cargo build` warning-free and test every change by rebuilding with `build.bat` and looking at the app.
* Don't give web tabs IPC. Don't send the Odysseus token to the UI. Don't block requests to `127.0.0.1`/`localhost`/`*.localhost` (Tauri's own protocols use them).
* Keep the design system above; when unsure, choose the quieter option.
* The user (Syed) is on Windows 11 with an RTX laptop, Ollama, and Odysseus at `C:\Users\13la7\odysseus` (launched with `Restart Odysseus (Tailscale).bat`, reachable over Tailscale at `100.119.240.1:7000`).
