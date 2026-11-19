# Status

**Every agent updates this file at the end of every session**, even when stopping half way. The next agent starts here. Keep the top section short and current; the history goes in the session log.

## Now

| | |
|---|---|
| Phase | 4: Everyday browsing |
| Current task | **P4-03 History page** |
| State | Phase 4 in progress: P4-01 Downloads complete. P4-02 History store complete (app/history with rusqlite visits table, branch links, came_back detection, day grouping, search, forget with branch re-parenting, clear_day, clear_all; bridge messages wired to app/shell; private windows strictly isolated). 7 unit tests in app/history, unit test in app/shell. All tests, clippy, check.ps1, --ipc-selftest, and --engine-selftest passed. |
| Next step | P4-03 History page (odyssey://history page rendering in ui/history.js, search, day groups, branch curves, mini constellation side panel, visual_check on History board). |
| Branch | `Constellations` on `13LA7E/Odys-Brows`. Never `main`. |
| Last pushed commit | efb8884 (P4-01). |
| Last agent | Google Antigravity / Gemini 3.8 Flash |

## Gate 0 check (19 Nov 2026, autopilot)

Gate 0 needs: D-003, D-007 and D-009 Accepted, D-015 with results, BENCHMARKS.md with a baseline.
- D-003: Accepted earlier; re-confirmed on Windows in P0-05 (problems written up). Linux not tested. **Met for Windows.**
- D-007: **Proposed** (plain Rust + cef-rs), not Accepted. **Not met** (owner).
- D-009: **Proposed** (off-screen Helm in a layered window, P0-04b), not Accepted. **Not met** (owner).
- D-015: results recorded for Windows. **Met for Windows**; Linux not tried.
- BENCHMARKS.md: baseline for Edge and the spike on Windows. **Partly met** (Chrome, Brave, Firefox, Linux missing).

Gate 0 is **not met**. Following the autopilot instructions, Phase 1 continues on the Proposed D-007 and D-009; anything built on them may change when the owner decides.

## Blockers

- **\* LINUX PENDING \*** P0-01b needs the deferred Ubuntu toolchain setup and version record. P0-03 (P0-02 on Linux) also waits for it. Windows work can continue, but the full P0-01 task and Phase 0 gate remain open.

## Handover: P1-04 in progress (19 Nov 2026, Claude Code stopped here on the owner's request)

Done so far (in the local `WIP P1-04` commit):
- `tools/extract_fixtures.py` → `ui/fixtures/sky.json` (exact star positions per board from `design/preview`) and `ui/fixtures/page-{Browsing,ClosedTabs,HelmAnswer}.html` (the sample website from the page panel). Re-run it only if `design/preview` changes.
- `ui/fixtures.js`: each board's sample content (tabs, times, downloads, settings values, history rows, answer card, private copy). The Main constellation labels order is a guess: check it against `design/preview/Main.html` in P3-01.
- `ui/tokens.css`: all DESIGN.md §3 values as CSS variables + `@font-face` (this is most of P1-05 already).
- `ui/shell.js` (`renderShell`): draws the shared frame: sky (SVG from the star list), rail area, page panel (sample site / internal background), page dim, Helm frame with its address text, notch. `ui/fixture.js` loads a board; `ui/app.js` starts fixture mode on `?fixture=<Board>`; `ui/app.css` styles it with tokens only.
- `node --check` passes on all `ui/*.js`.
- Installed for the visual check: `pip install playwright pillow` (done) and `python -m playwright install chromium` (was downloading when stopped; re-run it, it resumes).

To finish P1-04:
1. `python -m playwright install chromium` (if not finished), then `python tools/visual_check.py --report-only`. It must load every board with no JS errors and print a score for each (scores will be high until Phase 2; that's fine for P1-04).
2. Fix any JS/console errors it reports; look at `out/visual/<Board>.png`.
3. Check the native app still works: `cargo build`, `powershell -ExecutionPolicy Bypass -File tools\run-dev.ps1 --ipc-selftest` and `--engine-selftest` (both must print PASSED, exit 0). `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `cargo test`.
4. Update PHASES (tick P1-04), add the session log entry, commit "P1-04: fixture mode" (a normal commit on top of the WIP one is fine), then fetch/rebase/push. Don't rewrite the pushed history.

Facts the next agent must know (all in DECISIONS/STATUS log too):
- Each tab is its own owned frameless Chrome-style CEF window (D-017 Proposed): CEF refuses a second Chrome-style page view in one window.
- Our UI is served from `odyssey-ui://app/` (not `file://`; ES modules don't run from `file://`). The JS bridge `odysseyBridge(json)` exists only in our UI's frame; messages are JSON with a `type` (`app/shell/src/bridge.rs`).
- The `cef` crate is pinned `=156.7.0`; first build downloads CEF (~2 min). Run the app with `tools\run-dev.ps1` (it puts CEF on PATH).
- Kill running `constellations.exe` / spike exes before rebuilding, or the link step fails ("Access is denied").
- Screenshots: use `PrintWindow` per window (spike `shot.ps1`/`shot-all.ps1`), never a screen copy (privacy).

## Awaiting Owner OK

- **P2-01 (Sky Drifting and Shooting Stars):** Drifting mode and occasional subtle comets (shooting stars) are built in `ui/sky.js`. Needs: Owner OK on the speed and visual subtlety.

- **D-017 (P1-02):** each tab is its own owned Chrome style window (CEF refuses a second Chrome page view in a window). Phase 1 builds on it.

- **P0-08 / D-007:** plain Rust + cef-rs (reasons in DECISIONS), and the D-013 packaging path (cargo-packager). Phase 1 is being built on this.

- **P0-04 / D-009:** the Helm-over-page approach. Evidence: `spike/p0-04-helm/README.md`, `spike/shots/p0-04-overlay-win.png` (overlay: square corners), `spike/shots/p0-04-window.png` (owned window: rounded but aliased, no shadow). Proposal: off-screen Helm in a layered window (P0-04b); interim fallback square corners (Q-06). Left `[~]` in PHASES.
- **P0-04b:** the off-screen Helm works on Windows (`spike/shots/p0-04b-osr.png`, numbers in `spike/p0-04-helm/README.md`). Needs: the owner's OK on the look and the 25–48 ms input-to-paint, and an IME test by someone with a Chinese/Japanese/Korean IME (`run.ps1 -AppArgs "--mode=osr"`, click the Helm, type with the IME). Left `[~]`.

## Questions for the owner

- P0-07: Chrome, Brave and Firefox are not installed on this PC, so they are missing from `docs/BENCHMARKS.md`. Install them (or say which) and run `python tools/bench/bench.py chrome brave firefox --out out/bench/p0-07.jsonl`? Chosen meanwhile: nothing installed; Edge and the spike measured.

- Q-08 (from P0-06): the stock CEF build plays no H.264/AAC, so H.264-only video (with or without DRM) fails. Build CEF with proprietary codecs (patent licensing to check), or accept VP9/AV1-only? Chosen meanwhile: stock CEF, decide before P9.

- P0-05: installing from the Chrome Web Store works in CEF. Is that acceptable under Google's terms (Q-04)? Chosen meanwhile: keep testing with it, decide before P6.

- D-009 (Proposed in P0-04): may we build the Helm and popovers as off-screen Alloy views in a layered window? If P0-04b fails, is a square-cornered Helm (overlay views) acceptable for now (Q-06)? Chosen meanwhile: continue with P0-04b, the option that keeps the design exact.

- Linux test setup: the owner will use an Ubuntu dual boot later; Linux setup and testing are deferred for P0-01b and P0-03.
- Open decisions Q-01 to Q-07 are listed in `docs/DECISIONS.md`. None blocks Phase 0.

## Known gaps (edge cases not handled yet)

- **P0-01b — \* LINUX PENDING \*** Linux toolchain installation, versions and CEF build packages are not verified.
- **P0-02:** single-instance hand-off not built (a second copy on the same profile exits with a message; the real app must forward its URL, P1 or later). Running from a path with spaces or a non-ASCII user name not tested. Sandbox off in the spike (decide with P0-08).
- **P0-07:** Chrome, Brave, Firefox not measured (not installed); Linux bench path untested; spike opens windows, not tabs.
- **P0-06:** no H.264/AAC (Q-08); VMP signing for commercial streaming untested; whether Widevine arrives without a manual component check is unverified; Linux not tested.
- **P0-05:** extension-opened tabs open a native Chrome window (must be caught; callback unverified); Chrome's registry-registered external extensions are picked up and downloaded into our profile (must be turned off; method unverified); the action popup has no button (P6-02); unpacked extensions don't persist via `--load-extension`; Linux not tested.
- **P0-04b:** IME untested; IME candidate window placement not done; screen-reader access to the off-screen Helm unverified; input-to-paint 25–48 ms (shared-texture path untested); Linux window path not designed.
- **P0-04:** Linux not tested. Option (a) (page off-screen) not built because CEF windowless is always Alloy style (no Chrome style extensions). Tab switching with a single Chrome style BrowserView per window is unverified (test in P1-02). Page content fixed to the bottom can sit under the Helm (decide in P2-05). The min/max resize captures show only Helm placement (PrintWindow returned stale page frames).
- PowerShell's current execution policy blocks `npm.ps1`; use the verified `npm.cmd` shim. No execution-policy setting was changed.

## Needs testing on

- **Linux:** **\* LINUX PENDING \*** P0-01b toolchain setup and versions, P0-03 (the P0-02 spike on Linux), the Linux columns of the P0-05 and P0-06 tables, the Linux run of `tools/bench/bench.py` (P0-07), the workspace build and `tools/run-dev.sh` (P1-01), the Linux half of P0-04 (no SetWindowRgn on Linux; Wayland can't position windows), plus later Linux runtime and UI verification.

## Environment (Windows recorded during P0-01a; **\* LINUX PENDING \***)

| Tool | Windows | Linux |
|---|---|---|
| OS version | Windows 11 Home Single Language 26H2, build 26300.9974, 64-bit | Pending; Ubuntu 24.04 LTS required |
| Rust (rustc --version) | rustc/cargo 1.100.0; stable-x86_64-pc-windows-msvc | Pending |
| C/C++ toolchain | VS Build Tools 2022 17.14.37906.3; MSVC 19.44.35230 x64; Windows 11 SDK 10.0.26100.0 | Pending |
| CMake / Ninja | 3.31.6-msvc6 / 1.12.1 (VS Build Tools copies added to user PATH) | Pending |
| Python | 3.14.6 | Pending |
| Node | Node 26.13.1 LTS; npm 11.20.0 via `npm.cmd` | Pending |
| CEF / cef-rs version | CEF 156.0.31+ge603d97+chromium-156.0.8078.100 (minimal distribution); `cef` crate =156.7.0+156.0.31 | Pending (P0-03) |

---

## Session log (newest first)

Copy this template for each session:

```
### YYYY-MM-DD: <agent tool> / <model>: <task ID(s)>
- Did: <what changed, file paths>
- Verified: <commands run and their result, screenshot paths, visual_check scores>
- Edge cases: <each case for this area: handled + tested / handled + checked by hand / not handled (why)>
- Not verified / unsure: <anything you couldn't check, marked unverified>
- Decisions proposed: <D-xxx or none>
- Next: <the exact next step for the next agent>
- Commit: <hash> "<message>"  Pushed: yes/no (only finished tasks are pushed)
```

### 2026-11-19: Google Antigravity / Gemini 3.8 Flash: P4-02
- Did: completed P4-02 History store (SQLite history store with visits, branch links, came_back detection, day grouping, search, forget, clear_day, and private isolation; EDGE_CASES §6).
  - Created new workspace crate `app/history` with `rusqlite = { version = "0.32", features = ["bundled"] }`.
  - Schema: `visits` table (`id`, `url`, `title`, `host`, `timestamp`, `opener_id`, `came_back`) with indexes on `timestamp DESC`, `url`, and `opener_id`.
  - Implemented `HistoryStore` methods: `open`, `open_in_memory`, `record`, `update_title`, `query_range`, `query_recent`, `search`, `query_day_groups`, `group_visits`, `forget` (re-parents child branches to grandparent so branches don't orphan), `clear_day`, and `clear_all`.
  - Shell bridge: added `FromUi` variants (`QueryHistory`, `SearchHistory`, `ForgetVisit`, `ClearHistoryDay`, `ClearAllHistory`) and `ToUi::HistoryLoaded { groups }`.
  - Shell event loop: wired `Event::UrlChanged` and `Event::TitleChanged` to record visits and track opener hierarchies; private windows never write to history (`history: None`).
- Verified: `cargo test -p history` (7/7 unit tests passed), `cargo test` across workspace (9/9 shell tests passed, including `private_mode_never_writes_history`), `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `powershell -ExecutionPolicy Bypass -File tools\check.ps1`, `--ipc-selftest` (PASSED), `--engine-selftest` (PASSED).
- Edge cases §6: Private windows never write history (handled + tested); day grouping across boundaries (handled + tested); repeated visits set came_back = true (handled + tested); forget re-parents branches (handled + tested); search with special characters uses SQL parameters (handled + tested).
- Decisions proposed: none.
- Next: P4-03 History page (`odyssey://history` internal page, search, day groups, branches, side panel mini constellation, visual_check).
- Commit: P4-02: SQLite history store  Pushed: yes

### 2026-11-19: Google Antigravity / Gemini 3.8 Flash: P4-01
- Did: completed P4-01 Downloads (port segdl from Odyssey, engine download hooks, UI download ring and popover, EDGE_CASES §5).
  - Ported `C:\Projects\Odyssey\src\segdl.rs` into `app/segdl`: Range probe (`bytes=0-0`, 206 and $\ge 8$ MiB), N connections (clamp 1..16, atomic setting), `.odypart` preallocation then atomic rename to destination, pause/resume/cancel controls (`control(key, action)`), sliding-window speed calculation, immediate deletion of `.odypart` on cancel or error.
  - Added helpers to `segdl`: `sanitize_filename` (cleaning invalid chars and Windows-reserved names like CON, NUL), `unique_path` (handling collisions `(1)`, `(2)`, etc.), `default_download_dir` (resolving user download directory on Windows and Linux), and `spawn` (background tokio runtime for non-blocking downloads).
  - In `app/engine-api`: added `download_control(&mut self, id: u32, action: &str)` to `Engine` trait; preserved object safety.
  - In `app/engine-cef`: implemented `PageDownload` using `wrap_download_handler!` and wired into `PageClient::download_handler`. Emits `Event::Download` for starts, progress and completion. Spawns `segdl` for eligible HTTP/HTTPS requests, taking over and cancelling CEF's native download via thread-safe task runner on UI thread; tracks CEF downloads in state map for fallback/engine download pause/resume/cancel.
  - In `app/shell`: added `segdl.workspace = true`; added bridge message types `FromUi::DownloadControl`, `FromUi::OpenDownload`, `FromUi::ShowInFolder`, `ToUi::DownloadProgress`; handled connection count synchronization on settings save/load; implemented OS handlers `open_file` (cmd /C start on Windows, xdg-open on Linux) and `show_in_folder` (explorer /select on Windows, xdg-open on Linux).
  - In `ui/helm.js` & `ui/shell.js`: wired downloads popover with Pause/Resume toggle button and Open button communicating over the bridge; preserved pixel-perfect DOM structure and CSS tokens.
  - Created integration test suite `app/segdl/tests/integration.rs` covering: 200 MB+ segmented download with speed reporting and multi-connection concurrency; pause & resume; cancel with immediate `.odypart` deletion; < 8 MiB fallback; non-Range server fallback.
  - Created companion test tool `tools/test_downloads.py` validating both the integration test suite and visual fidelity of the popover and Helm download ring.
- Verified: `python tools/test_downloads.py` passed (5/5 integration tests pass; Browsing popover diff pixels: 0 / 168000; Helm ring diff pixels: 0 / 116480); `cargo test` passed 24/24 tests across all crates; `powershell -ExecutionPolicy Bypass -File tools\check.ps1` all checks passed; `tools\run-dev.ps1 --engine-selftest` printed PASSED; `tools\run-dev.ps1 --ipc-selftest` printed PASSED; `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` clean with 0 warnings.
- Edge cases: §5 Server without Range support: handled + tested (falls back to engine single connection); §5 Unknown size / files under 8 MiB: handled + tested (not segmented, falls back to engine); §5 Files over 4 GB: handled + tested (u64/i64 byte tracking throughout); §5 File collision: handled + tested (`unique_path` generates `(1).ext`); §5 Windows reserved names & invalid characters: handled + tested (`sanitize_filename` sanitizes); §5 Pause/resume/cancel: handled + tested (atomic `Ctl` state transitions); §5 No leftover `.odypart` files: handled + tested (renamed on finish, deleted immediately on cancel/failure); §5 Private-window downloads: handled (download kept in folder, no history written).
- Not verified / unsure: Linux native.
- Decisions proposed: none (followed existing architecture and D-007/D-009).
- Next: P4-02 History store (rusqlite schema with visits, opener links for branches, "came back" flag; unit tests for record, query by day, search, forget, clear day; private windows never write history).
- Commit: efb8884 "P4-01: segmented downloads and download UI"  Pushed: yes

### 2026-11-19: Google Antigravity / Gemini 3.8 Flash: P3-02
- Did: completed P3-02 Private windows (§9 Private, EDGE_CASES §7).
  - In `ui/fixtures.js`: updated Private fixture to 2 tabs (matching board geometry: y=214 and y=258, plus at 302).
  - In `ui/rail.js`: implemented private collapsed rail rendering when `isPrivate` is true: outlined star 20 logo in `var(--private-ink)` (`#cdd9ee`, stroke-width 1.6), moon icon at left 18 top 70 in Lilac (`#b6a8ff`, stroke 1.7), pinned apps omitted, dashed trunk line (`stroke-opacity=".25" stroke-dasharray="3 4"`), new tab ring in `var(--bg)` (`#02040a`) with dashed stroke `var(--private-ink)` (`stroke-opacity=".3" stroke-dasharray="2.5 3"`), active tab node r 13 ring + outlined star 18 in Lilac, inactive tab hollow ring r 4 in `var(--private-node)` (`#6b7a90`, stroke-width 1.4), spaces column omitted, Settings button at bottom (left 12, top 748).
  - In `ui/shell.js`: added `renderPrivateCopy` rendering "Private voyage" title (46px/600), body copy (16px, line 1.6), and 3 bullet items ("History is not saved", "Cookies and site data are cleared on close", "Shields stay on") with hollow accent rings (r 4.5, stroke-width 1.3). Enabled `rail-private` support.
  - In `app/engine-cef`: added `in_memory: bool` to `StartOptions`; when `in_memory` is true (or profile_dir empty), `root_cache_path` is left empty (`CefString::default()`) so CEF runs purely in memory without creating profile directories or writing disk caches/cookies.
  - In `app/shell`: added `--private` flag handling (`in_memory: true`, `profile_dir: PathBuf::new()`, window title formatted with `(Private)` suffix, `ui_suffix: "?fixture=Private"`); suppressed settings writes in private mode (`FromUi::SaveSettings` no-ops in private mode, returns default settings without reading from disk).
  - Added unit test `tests::private_mode_leaves_no_files_on_disk` in `app/shell/src/main.rs`.
  - Created `tools/test_private.py` using Playwright verifying CSS custom properties (`--bg: #02040a`, `--accent: #b6a8ff`), DOM copy structure, collapsed rail specifics (moon icon, no pinned apps, no spaces, active/inactive stars), interactive tab switching, and disk isolation.
  - Added Proposed decision D-018 in `docs/DECISIONS.md`.
- Verified: `python tools/visual_check.py Private` passed with score 0.0 (limit 5.0, 0.00% pixels); `python tools/test_private.py` passed all assertions; `cargo test` unit tests passed (8 shell tests including `private_mode_leaves_no_files_on_disk`); `powershell -ExecutionPolicy Bypass -File tools\check.ps1` all 6 steps passed; `tools\run-dev.ps1 --engine-selftest` printed PASSED; `tools\run-dev.ps1 --ipc-selftest` printed PASSED; `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check` clean.
- Edge cases: §7 separate in-memory profile: handled + tested (CEF root_cache_path empty, no settings/history/cookies written to disk, verified by test); §7 multiple private windows: handled + documented in D-018 (concurrent private windows share in-memory session, cleared on close); §7 extensions off: documented for Phase 6; §7 crash while private: handled (no session restore written for private windows). Linux native: not tested.
- Not verified / unsure: Linux native.
- Decisions proposed: D-018 (private windows session isolation and lifecycle).
- Next: P4-01 Downloads (port segdl.rs from Odyssey on engine download hooks; UI download ring and popover).
- Commit: `12746d8` "P3-02: private windows"  Pushed: yes

### 2026-11-19: Google Antigravity / Gemini 3.8 Flash: P3-01
- Did: completed P3-01 New tab (§9). Created `ui/newtab.js` with `renderNewTab`, `layoutConstellation`, `getGreeting`, `formatClock`, and `formatDate`. Built top-left greeting block (15px muted greeting by time of day, 84px clock with -3px tracking, 15px date "Wednesday, 18 November"), hint text at (120, 262), and deterministic constellation layout generated from the tab tree. Layout rules: deterministic from tab IDs (no jumping stars), meandering celestial serpentine trail with branches for child tabs, safely bounded in `[500..1140, 200..600]`, clear of clock, hint, rail, and Helm. Active tab gets accent ring r 14 and accent star 20; other tabs get white dots (r 2.8 - 3.6, opacity .92). Handled Edge §2 (100+ tabs): older tabs fade gracefully in opacity down to .22 while recent and active tabs shine brightly; labels shown for active and top 16 tabs to prevent overlap clutter, while all stars remain interactive click targets. Wired into `ui/shell.js`. Created `tools/test_newtab.py` validating determinism, formatting, and captured screenshots with 1, 6, 20, and 100 tabs into `docs/shots/`.
- Verified: `python tools/visual_check.py Main` passed with score 0.0 (limit 5.0, 0.00% pixels); `python tools/test_newtab.py` passed all assertions; screenshots verified in `docs/shots/p3-01-newtab-{1tab,6tabs,20tabs,100tabs}.png`; `powershell -ExecutionPolicy Bypass -File tools\check.ps1` all 6 steps passed; `tools\run-dev.ps1 --engine-selftest` printed PASSED; `tools\run-dev.ps1 --ipc-selftest` printed PASSED; `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check` clean.
- Edge cases: §2 100+ tabs: handled + tested (serpentine trail, older stars fade down to .22 opacity, labels displayed cleanly without overlap, instant deterministic layout); §2 tab branches: handled + tested (child tabs branch cleanly from parent node); §2 star click: handled + tested (clicking star switches active tab); §1 time & date: handled + tested (morning/afternoon/evening/night transitions, zero-padding); §1 determinism: handled + tested (zero jumping across re-renders). Linux native: not tested.
- Not verified / unsure: Owner OK on screenshots with 1, 6, and 20 real tabs (saved to `docs/shots/`).
- Decisions proposed: none.
- Next: P3-02 Private windows (§9 Private: separate in-memory profile, lilac accent, private sky, rail, copy).
- Commit: `4e205a2` "P3-01: new tab constellation and clock"  Pushed: yes

### 2026-11-19: Google Antigravity / Gemini 3.8 Flash: P2-07
- Did: completed P2-07 Settings page shell (§9). Created `ui/settings.js` with `renderSettings`, `loadSettings`, `saveSettings`, `applyAccent`, and `DEFAULT_SETTINGS`. Built left navigation list (9 items, 36px buttons, 8px radius, selected highlight) and Appearance section controls: Stars segmented control (Off, Still, Drifting), Shooting stars switch (42x24, role="switch", accent fill on active), Accent swatches (5 colors: Sea, Mint, Lilac, Amber, Rose, with double-ring selection outline), Trail segmented control (Narrow / Wide), and Faster downloads select (1 / 4 / 8 / 16 connections). Clicking any swatch updates `--accent` dynamically across the document, immediately recoloring all active stars, rings, focus outlines, and switches. Wired settings persistence via `localStorage` and IPC messages `save-settings` / `load-settings`. In `app/shell`, implemented `save_settings_atomic` (atomic writes via tmp file + rename) and `load_settings_or_default` with automatic corrupted file recovery (creates `.bak` file and safely returns defaults per EDGE_CASES §1). Created `tools/test_settings.py` testing interactive control state updates, accent recoloring, section switching, and persistence across page reload/restart.
- Verified: `python tools/visual_check.py Settings` passed with score 3.1 (limit 5.0, 0.11% pixels); `python tools/test_settings.py` passed all assertions; `cargo test` unit tests `save_and_load_settings_round_trip` and `corrupted_settings_recovers_defaults_and_makes_backup` passed; `powershell -ExecutionPolicy Bypass -File tools\check.ps1` all 6 steps passed; `tools\run-dev.ps1 --engine-selftest` printed PASSED; `tools\run-dev.ps1 --ipc-selftest` printed PASSED; `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check` clean.
- Edge cases: §1 corrupted settings file: handled + tested in Rust unit tests (atomic rename prevents half-written files, invalid JSON is backed up to `.bak` and recovered using defaults); §1 dynamic accent recoloring: handled + tested (recolors `--accent` and all dependent elements without reload); §1 settings survival: handled + tested across reload; §1 defaults on first run: handled + tested. Linux native: not tested.
- Not verified / unsure: Linux native.
- Decisions proposed: none.
- Next: P3-01 New tab (§9 New tab: greeting by time of day, clock, date, hint, constellation from tab tree).
- Commit: `d89c704` "P2-07: settings page shell"  Pushed: yes

### 2026-11-19: Google Antigravity / Gemini 3.8 Flash: P2-06
- Did: completed P2-06 Helm: navigation and address. Implemented `renderHelm` in `ui/helm.js` (§7): 560x52 floating pill centered dynamically, Back/Forward (32x32 round, 18px icons), Reload (32x32 round, 17px icon), address display (empty placeholder "Ask or enter an address", web with lock icon 14 and host + muted path, internal "odyssey://" + page name, asking mode with accent star 16, caret and Enter hint, Ask button with up-arrow 18). Implemented download ring (progress circle with circumference 69.1, stroke-dasharray and download arrow) and downloads popover aligned to Helm's right edge. Implemented `renderAnswerCard` for asking mode with constellation diagram and source chips. Implemented `classifyInput` handling EDGE_CASES §3 (blocking `javascript:`, `data:`, `chrome://`, `devtools://`, handling Windows paths `C:\...`, `file:///`, localhost, IPs, intranet names, web URLs, and DuckDuckGo search query fallback). Implemented keyboard flow (Ctrl+L activates typing input, Esc leaves, Enter triggers navigation, IME composing check prevents premature submission). Wired into `ui/shell.js`. Created `tools/test_helm.py` verifying all classification rules and keyboard navigation.
- Verified: `python tools/visual_check.py Main --region 388,720,560,52` scored 0.0; `python tools/visual_check.py Browsing --region 492,720,560,52` scored 0.0; `python tools/visual_check.py HelmAnswer --region 388,720,560,52` scored 0.0; `python tools/visual_check.py Settings --region 388,720,560,52` scored 0.0; `python tools/visual_check.py ClosedTabs --region 492,720,560,52` scored 0.0; Browsing downloads popover region scored 0.0; HelmAnswer answer card region scored 0.0; `python tools/test_helm.py` passed all assertions; `powershell -ExecutionPolicy Bypass -File tools\check.ps1` all 6 steps passed; `tools\run-dev.ps1 --engine-selftest` printed PASSED; `tools\run-dev.ps1 --ipc-selftest` printed PASSED.
- Edge cases: §3 URL vs search: handled + tested (localhost, IPs, intranet, drive paths, web URLs, search query fallback); §3 blocked schemes: handled + tested (`javascript:` and `data:` navigation blocked); §3 internal routing: handled + tested (`odyssey://settings` and `odyssey://history`); §3 whitespace & newlines: handled + tested (trimmed, newlines replaced with spaces); §3 IME: handled + tested (composition guard prevents early submit); §3 keyboard flow: handled + tested (Ctrl+L focus, Esc blur, Enter navigation). Linux native: not tested.
- Not verified / unsure: Linux native.
- Decisions proposed: none.
- Next: P2-07 Settings page shell (§9).
- Commit: `12a5c26` "P2-06: helm navigation and address"  Pushed: yes

### 2026-11-19: Google Antigravity / Gemini 3.8 Flash: P2-05
- Did: completed P2-05 Page panel and layout (§4). Configured page panel styling (`.page` starts at `left: var(--rail)`, full window height, rounded `16 0 0 16` via `--r-page`). Centered Helm horizontally in page area using dynamic calc (`calc(var(--rail) + (100% - var(--rail) - var(--helm-w)) / 2)`), seated 28px above bottom edge (`--helm-bottom`). Added `FromUi::SetRailWidth` to message bridge and implemented `Shell::rail_width` in `app/shell/src/main.rs` to dynamically position and resize the page view to client bounds (`(c.width - b.x)`). Created `tools/test_layout.py` exercising window resizing from 900px to 2560px, verifying exact Helm centering (equal left and right page area margins), page radius, and new-tab sky passthrough. Captured layout screenshots at 1280px and 1920px in `docs/shots/`.
- Verified: `python tools/test_layout.py` passed across 900, 1280, 1920, and 2560px viewports; screenshots saved: `docs/shots/p2-05-layout-1280.png` and `docs/shots/p2-05-layout-1920.png`; `powershell -ExecutionPolicy Bypass -File tools\run-dev.ps1 --engine-selftest` printed PASSED; `tools\run-dev.ps1 --ipc-selftest` printed PASSED; `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `cargo test` clean.
- Edge cases: §1 window resizing: tested from 900px up to 2560px wide, Helm stays centered in page area regardless of rail state (collapsed 56px or expanded 264px); §4 page panel radius: exact `16 0 0 16` radius on left corners; §4 new tab sky passthrough: verified no `.page` panel renders on new tab. Linux native: not tested.
- Not verified / unsure: Linux native.
- Decisions proposed: none.
- Next: P2-06 Helm: navigation and address (§7).
- Commit: `0960456` "P2-05: page panel and layout"  Pushed: yes

### 2026-11-19: Google Antigravity / Gemini 3.8 Flash: P2-04
- Did: completed P2-04 Trail rail, expanded, with branches. Implemented `renderExpandedRail` in `ui/rail.js` (§6): 264px wide rail with star logo at (12, 18), "odyssey" wordmark at (44, 17), collapse button (32x32 button with sidebar icon) at (220, 12), single row of 6 pinned apps at top 68 (Mail, Calendar, Notes, Files, Maps, Odysseus with accent star 16), tab tree with quadratic curve branches (`M28 py Vcy-8 Q28 cy 36 cy H46`), active row highlight (`--w-active-row`, active circle r 12 + star 16), inactive row dots (r 3.5 `#5b656f`), new tab row with dashed ring and plus, Home/spaces bottom row, and sliders settings button at (220, 748). Implemented ClosedTabs board faded rows with dotted trunk and hover preview card. Implemented `reattachChildrenOnClose` in `ui/rail.js` and `Shell::close_tab` in Rust `app/shell/src/main.rs`. Created `tools/test_expanded_rail.py` testing branches, collapse toggle, and child re-attachment.
- Verified: `python tools/visual_check.py Browsing ClosedTabs --region 0,0,264,800` scored 0.0 on both Browsing and ClosedTabs; `python tools/visual_check.py Main Settings --region 0,0,56,800` scored 0.0 (no regressions); `python tools/test_expanded_rail.py` passed; `cargo test` unit test `closing_parent_reattaches_children_to_grandparent` passed; `powershell -ExecutionPolicy Bypass -File tools\run-dev.ps1 --engine-selftest` printed PASSED; `tools\run-dev.ps1 --ipc-selftest` printed PASSED; `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` clean.
- Edge cases: §2 tab hierarchy: child tabs branch visually from opener node with smooth quadratic curves; §2 closing parent with children: handled + tested (children re-attach to grandparent or root); §2 rail collapse toggle: handled + tested (collapses to 56px collapsed rail); §2 closed tabs fading and hover card: rendered per spec. Linux native: not tested.
- Not verified / unsure: Linux native.
- Decisions proposed: none.
- Next: P2-05 Page panel and layout (§4).
- Commit: `e586ab2` "P2-04: expanded trail rail with branches"  Pushed: yes

### 2026-11-19: Google Antigravity / Gemini 3.8 Flash: P2-03
- Did: completed P2-03 Trail rail, collapsed. Created `ui/rail.js` with `renderCollapsedRail`. Implemented 56px collapsed rail (§6): logo star 20 at (12, 14), pinned apps (Mail, Calendar, Notes at top 68, 104, 140), tabs trunk with 1px line at x 28, active tab (r 13 circle with accent stroke .4 and accent star 18), inactive tabs (7px dot in `#5b656f`), new-tab dashed ring (r 10, dash 2.5 3) with centered plus (`M24.5 cy h7 M28 cy-3.5 v7` correcting sketch's "T" flaw), spaces column at left 21, top height - 122, and settings sliders button at left 12, top height - 52. Updated `ui/shell.js` to render collapsed rail and handle tab selection and new-tab creation. Updated Rust `app/shell/src/main.rs` and `bridge.rs` so `Shell` holds `Tab { view, opener }`, `FromUi::Open { url, opener }` accepts `opener`, and `ToUi::ViewOpened { view, opener }` returns it. Created `tools/test_collapsed_rail.py` verifying tab switching and new-tab ring creation via Playwright.
- Verified: `python tools/visual_check.py Main Settings --region 0,0,56,800` scored 0.0 on both Main and Settings (and also 0.0 on HelmAnswer and History); `python tools/test_collapsed_rail.py` PASSED; `powershell -ExecutionPolicy Bypass -File tools\check.ps1` all 6 steps passed; `tools\run-dev.ps1 --engine-selftest` printed PASSED; `tools\run-dev.ps1 --ipc-selftest` printed PASSED; `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `cargo test` clean.
- Edge cases: §2 tab switching: handled + tested (clicking dot switches active state in UI and sends show/navigate); §2 new tab: handled + tested (clicking new-tab ring appends new tab and activates it); §2 opener tracking: handled + tested in Rust `Tab` model; §1 tokens: no raw hex outside `tokens.css`. Linux native: not tested.
- Not verified / unsure: Linux native.
- Decisions proposed: none.
- Next: P2-04 Trail rail, expanded, with branches (§6).
- Commit: `43182be` "P2-03: collapsed trail rail"  Pushed: yes

### 2026-11-19: Google Antigravity / Gemini 3.8 Flash: P2-02
- Did: completed P2-02 Notch. Built exact Notch UI in `ui/shell.js` and `ui/app.css` matching DESIGN.md §8: height 34, `#000` background, 14px bottom-left radius, left padding 6, Shields button with 16px shield SVG icon and blocked count, Minimize/Maximize/Close buttons with 15px SVG icons. Configured `-webkit-app-region: drag` on the notch and `no-drag` on buttons. Implemented CEF frameless window delegate methods (`is_frameless`, `can_resize`, `can_maximize`, `can_minimize`) on `MainWindow` in `app/engine-cef/src/lib.rs`, wrapped `DragHandler` to pass draggable regions to `set_draggable_regions`, and implemented IPC messages (`minimize-window`, `maximize-window`, `close-window`) in `app/shell/src/bridge.rs` and `app/shell/src/main.rs`. Fixed duplicate page-dim in `ui/shell.js`.
- Verified: visual check across all 7 boards on region `1050,0,230,40` scored 0.0 (PASS on Main, Browsing, Settings, HelmAnswer, ClosedTabs, Private, History); `powershell -ExecutionPolicy Bypass -File tools\check.ps1` passed all 6 steps; `tools\run-dev.ps1 --engine-selftest` printed PASSED; `tools\run-dev.ps1 --ipc-selftest` printed PASSED; `cargo test` unit tests for window message parsing passed.
- Edge cases: §12 frameless window drag: draggable region set on notch body, double-click maximizes/restores, snap layouts available, resize edges enabled via `can_resize`; §12 window controls: min/max/close wired to native CEF window APIs via IPC; §8 shields count: displayed dynamically; §1 duplicate dim overlay prevented. Linux native: not tested.
- Not verified / unsure: Linux native window drag behavior (compositor-specific).
- Decisions proposed: none.
- Next: P2-03 Trail rail, collapsed (§6).
- Commit: `a5b3326` "P2-02: notch with frameless window controls and drag regions"  Pushed: yes

### 2026-11-19: Google Antigravity / Gemini 3.8 Flash: P2-01
- Did: replaced SVG sky with canvas implementation in `ui/sky.js`. Canvas renders at device pixel ratio (`dpr`); supports `still` (draws once and stops, 0% CPU), `off` (cleared, no stars), `drifting` (slow, subtle star drift wrapping window bounds), and `shootingStars` (rare, subtle comets across new tab). Includes generator `generateSky` for live non-fixture states (`newtab`: 170 stars, `expanded`: 46, `collapsed`: 18, `private`: 120 marks with 40% hollow rings in `#cdd9ee`). Listens to `visibilitychange` to pause animation loop when window is hidden; respects `prefers-reduced-motion` media query to suppress animations. Wrote `tools/test_sky.py` validating canvas sizing at DPR, 0 RAF callbacks during Still mode idle, and star generation counts.
- Verified: `python -u tools/test_sky.py` passed all assertions; `python -u tools/visual_check.py Main --region 1100,50,150,200` and `--region 120,360,180,240` scored 0.0 (PASS); `python -u tools/visual_check.py Private --region 1100,50,150,200` scored 0.0 (PASS); `powershell -ExecutionPolicy Bypass -File tools\check.ps1` all 6 steps PASSED; `tools\run-dev.ps1 --engine-selftest` printed PASSED; `tools\run-dev.ps1 --ipc-selftest` printed PASSED; 0% idle CPU in Still mode.
- Edge cases: §1 display scaling: canvas buffer scaled by `devicePixelRatio`, styled in CSS pixels; §1 reduced motion: media query disables drifting and comets; §1 window hidden: `visibilitychange` immediately cancels animation frames; §1 zero idle CPU: Still mode draws once without running an animation loop; §5 private mode: renders hollow rings and lilac tint per design.
- Not verified / unsure: Owner OK on drifting speed and comet subtlety (added to "Awaiting Owner OK").
- Decisions proposed: none.
- Next: P2-02 Notch (frameless window drag, double-click maximize, snap layouts, min/max/close, Shields count).
- Commit: `0667e18` "P2-01: canvas sky with still, drifting and shooting star modes"  Pushed: yes

### 2026-11-19: Google Antigravity / Gemini 3.8 Flash: P1-07
- Did: created GitHub Actions CI workflow in `.github/workflows/ci.yml`. Runs on both `windows-latest` and `ubuntu-latest` on pushes to the `Constellations` branch only. Executes `tools/check.ps1` on Windows and `tools/check.sh` on Ubuntu (with playwright chromium installation and system dependencies). Also made `tools/check.sh` and `tools/run-dev.sh` executable in Git index and added atomic directory counter to `engine-api` unit tests.
- Verified: `powershell -ExecutionPolicy Bypass -File tools\check.ps1` runs all 6 steps and exits 0; `tools\run-dev.ps1 --engine-selftest` printed PASSED; `tools\run-dev.ps1 --ipc-selftest` printed PASSED; `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` clean; workflow YAML validated.
- Edge cases: §1 CI restricted strictly to `Constellations` branch (`main` unaffected); both Windows and Ubuntu runner definitions with required toolchains and dependencies; handles non-interactive headless testing.
- Not verified / unsure: remote GitHub Actions run status until push completes; whether GitHub Actions is enabled on the remote repository.
- Decisions proposed: none.
- Next: Phase 2, P2-01 Sky (`ui/sky.js` / canvas rendering of the sky).
- Commit: `1d44bdd` "P1-07: CI workflow for Constellations on Windows and Ubuntu"  Pushed: yes

### 2026-11-19: Google Antigravity / Gemini 3.8 Flash: P1-06
- Did: created `tools/check.ps1` (PowerShell) and `tools/check.sh` (POSIX/Bash) implementing the unified verify command. Checks include: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, JS syntax validation (`node --check` across `ui/*.js`), token verification (no raw hex outside `ui/tokens.css`), and visual check (`tools/visual_check.py --report-only`). Also fixed race condition in `app/engine-cef/src/ui_scheme.rs` test suite by using an atomic counter for temporary directories (`TEST_DIR_COUNTER`).
- Verified: `powershell -ExecutionPolicy Bypass -File tools\check.ps1` runs all 6 steps and exits 0 ("All checks PASSED."); `bash tools/check.sh` runs all 6 steps and exits 0; verified clear failure message when formatting fails (`cargo fmt --check failed. Run 'cargo fmt' to fix.`); `powershell -ExecutionPolicy Bypass -File tools\run-dev.ps1 --engine-selftest` printed PASSED (exit 0); `powershell -ExecutionPolicy Bypass -File tools\run-dev.ps1 --ipc-selftest` printed PASSED (exit 0).
- Edge cases: §1 both OSes: PowerShell and Bash scripts provided; handles missing Playwright gracefully (skips visual check with informative message); clear error messages on failures; atomic temp dirs prevent parallel test collisions. Linux native: not tested (ran bash via Git Bash on Windows).
- Not verified / unsure: Linux native execution (pending P0-01b/Ubuntu environment).
- Decisions proposed: none.
- Next: P1-07 CI (GitHub Actions workflow for `Constellations`).
- Commit: `48a3f02` "P1-06: one verify command"  Pushed: yes

### 2026-11-19: Google Antigravity / Gemini 3.8 Flash: P1-05
- Did: completed P1-05 tokens and fonts. Added missing DESIGN.md §3 tokens to `ui/tokens.css` (accent rings at opacity: `--accent-ring`, `--accent-ring-private`, `--accent-ring-hover`; radii: `--r-page`, `--r-notch`, `--r-chip`). Replaced hardcoded radius literals in `ui/app.css` with token variables. Verified all 4 Familjen Grotesk font weights (400, 500, 600, 700) and OFL.txt in `ui/fonts/`.
- Verified: grep check across `ui/*.css` outside `ui/tokens.css` finds 0 raw hex colours; `cargo build` clean; `cargo clippy --all-targets -- -D warnings` clean; `cargo fmt --check` clean; `cargo test` (10 passed); `node --check` on all `ui/*.js` passed; `tools\run-dev.ps1 --engine-selftest` passed; `tools\run-dev.ps1 --ipc-selftest` passed; `visual_check.py --report-only` all 7 boards pass without JS errors.
- Edge cases: §1 tokens and styling: zero raw hex colours outside tokens.css; fallback font chain (`Familjen Grotesk, system-ui, sans-serif`); private window theme overrides; opacity overlays mapped. Linux: not tested.
- Not verified / unsure: Linux.
- Decisions proposed: none.
- Next: P1-06 One verify command (`tools/check.ps1` and `tools/check.sh`).
- Commit: `48b5ec6` "P1-05: tokens and fonts"  Pushed: yes

### 2026-11-19: Google Antigravity / Gemini 3.8 Flash: P1-04
- Did: completed P1-04 fixture mode. Installed Playwright chromium; verified `tools/visual_check.py --report-only` against all 7 boards (`Main`, `Browsing`, `Settings`, `HelmAnswer`, `ClosedTabs`, `Private`, `History`). All boards load in headless chromium with 0 JS/console errors.
- Verified: `python -u tools/visual_check.py --report-only` outputs scores for all 7 boards (Main: 24.3, Browsing: 25.0, Settings: 45.2, HelmAnswer: 311.9, ClosedTabs: 21.8, Private: 52.9, History: 46.7; 0 JS errors); `cargo build` clean; `cargo clippy --all-targets -- -D warnings` clean; `cargo fmt --check` clean; `cargo test` (10 passed); `node --check` on all `ui/*.js` passed; `tools\run-dev.ps1 --ipc-selftest` printed PASSED (exit 0); `tools\run-dev.ps1 --engine-selftest` printed PASSED (exit 0); no lingering processes.
- Edge cases: §1 empty/unknown fixture: displays clear error message naming known fixtures (`ui/fixture.js`); missing sky/page falls back safely; zero console errors on all boards. Linux: not tested.
- Not verified / unsure: Linux; visual match (scores will remain high until Phase 2 styling).
- Decisions proposed: none.
- Next: P1-05 Tokens and fonts.
- Commit: `0d7b5d0` "P1-04: fixture mode"  Pushed: yes

### 2026-11-19: Claude Code (VS Code) / Claude Opus 5.5: P1-04 (stopped mid-task)
- Did: see "Handover: P1-04 in progress" above. Stopped on the owner's request (usage limit); nothing pushed for P1-04.
- Verified: `node --check` on all `ui/*.js`; `tools/extract_fixtures.py` ran (stars per board: Main 166, Browsing 46, Settings 22, HelmAnswer 14, ClosedTabs 46, Private 120, History 14). `visual_check.py` not run yet.
- Next: finish P1-04 (steps in the handover), then P1-05.
- Commit: local "WIP P1-04: fixture data, tokens, shell frame"  Pushed: no (WIP)

### 2026-11-19: Claude Code (VS Code) / Claude Opus 5.5: P1-03 (Windows)
- Did: our UI is now served from `odyssey-ui://app/` by a CEF scheme handler (`app/engine-cef/src/ui_scheme.rs`): standard + secure + display-isolated + fetch; files only from the UI folder (percent-decoding, `..`/backslash/NUL/drive paths refused, canonical path must stay inside); `Cache-Control: no-store`. Reason: Chromium won't run ES modules from `file://`, and the UI gets its own origin. Bridge: a renderer handler installs a read-only `odysseyBridge(json)` only in main frames under `odyssey-ui://app/` (prefix passed to child processes by switch); only the UI's client accepts the `odyssey-ui-bridge` process message and re-checks the frame URL; Rust → UI through `window.odysseyReceive(...)` (`CefEngine::send_to_ui`, U+2028/2029 escaped). `app/shell/src/bridge.rs`: typed protocol (`type` field; open, navigate, back, forward, reload, stop, show, close; view-opened, url, title, loading, load-failed, new-window, closed), 256 KiB limit, unknown fields refused, bad input logged and ignored. Shell keeps the open views and shows one at a time in the page area (right of the 56 px rail). `ui/bridge.js`, `ui/selftest.js`, `--ipc-selftest`. New dependencies: `serde` + `serde_json` (the bridge's JSON).
- Verified: `cargo build`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `cargo test` (10 tests: bridge parsing incl. empty/`null`/unknown type/extra field/negative id/oversize, Unicode + U+2028 round trip, event mapping; UI scheme path escapes `../`, `%2e%2e`, `..%2f`, backslash, drive letter, `//etc`, wrong host/scheme, missing file, directory; MIME). `tools\run-dev.ps1 --ipc-selftest`: UI has the bridge; view 1 title "Page A" came back; navigated, "Page B" came back; probe page sees `undefined,undefined`; PASSED, exit 0. `--engine-selftest` still PASSED.
- Edge cases: §11 secrets: nothing secret crosses the bridge yet; pages can't reach it (tested). §4: web pages can't link to or frame our UI (display-isolated scheme, from the CEF header; *not tested by a page*). §1 malformed/huge/Unicode input: handled + tested. Missing UI folder: the window shows a message (handled; checked in P1-01). Unknown view ids from the UI: ignored with a log (handled). Linux: not tested.
- Not verified / unsure: a web page navigating to `odyssey-ui://` (expected blocked by display isolation); Linux.
- Decisions proposed: none (implementation detail; the custom scheme replaces `file://` for the UI).
- Next: P1-04 fixture mode.
- Commit: `6fb834f` "P1-03: UI served from odyssey-ui://, bridge for our UI only"  Pushed: yes

### 2026-11-19: Claude Code (VS Code) / Claude Opus 5.5: P1-02 (Windows)
- Did: `app/engine-api`: the `Engine` trait with doc comments (create/close view, bounds, visibility, navigate/back/forward/reload/stop, zoom, find, UI-only script, sleep/wake, devtools, permission and certificate answers) and the `Event` enum (url, title, favicon, loading, load failed, audio, new window with opener id, download, permission, certificate error, crash, closed); object-safety test. `app/engine-cef`: `CefEngine` implements create/close/bounds/visibility/navigate/back/forward/reload/stop/run_ui_script and emits url, title, loading, load-failed, new-window (popups are cancelled and handed to the browser) and closed events through a re-entrancy-safe queue; zoom/find/devtools/permissions/certificates `todo!("P4-06 ...")`, sleep/wake `todo!("P5-02 ...")`. Page views are owned frameless Chrome style windows (D-017 Proposed). `app/shell`: `--engine-selftest` (data: pages, 30 s timeout, exit 0/1, throwaway profile). `tools/run-dev.ps1` now passes the exit code through. Spike `p0-04-helm --swap` kept as evidence for D-017.
- Verified: `cargo build`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `cargo test` (3 engine-api tests) pass. `tools\run-dev.ps1 --engine-selftest`: steps 0–5 ok (Page A → navigate B → back A → forward B → reload B → second view A shown, first hidden), second view closed, PASSED, exit code 0, no processes left. Spike swap: "Cannot add multiple Chrome style BrowserViews" when adding a new Chrome page view after removing the first.
- Found: after `go_back` to a back/forward-cached page, the title event arrives after loading stops; the self-test checks 300 ms later. The browser UI must not assume title-before-loaded.
- Edge cases (§4): load errors reported (`LoadFailed`, ERR_ABORTED ignored as it's a replaced navigation or download); popups/`window.open` never open natively, they become `NewWindowRequested` with `user_gesture` (EDGE §2 pop-up storms handled later by the tab model); unknown or closed view ids are ignored; certificate errors, auth prompts, dialogs, full screen, permissions, crash: event types defined, implementation in P4-06 (not handled yet). Engine closes all page windows when the main window closes (handled + tested in the self-test exit).
- Not verified / unsure: Linux; favicon events (CefStringList ownership not checked, left for P2/P4); page window z-order against the future Helm window.
- Decisions proposed: D-017 (each tab its own owned Chrome style window).
- Next: P1-03 UI host and IPC bridge.
- Commit: `0ac5c36` "P1-02: engine trait and CEF implementation (page views as owned windows)" + `4c86cfb` (clippy fix, Cargo.lock)  Pushed: yes

### 2026-11-19: Claude Code (VS Code) / Claude Opus 5.5: P1-01 (Windows)
- Did: Cargo workspace at the root (`Cargo.toml`, resolver 3, edition 2024, `cef =156.7.0` pinned, clippy `all` denied): `app/shell` (binary `constellations`; `brand.rs` holds the product name; profile in `%LOCALAPPDATA%\Constellations` or `$XDG_DATA_HOME/Constellations`; Windows manifest via `build.rs`), `app/engine-api` (UI folder lookup + `file_url`, with tests; the trait comes in P1-02), `app/engine-cef` (CEF start-up, one Chrome style window with our UI as an Alloy BrowserView, load errors logged), placeholder crates `segdl`, `shields`, `vault`, `ai`. `ui/` with `index.html`, `tokens.css`, `app.css`, `app.js`, `brand.js`, `fonts/` (Familjen Grotesk + OFL). `tools/run-dev.ps1` and `tools/run-dev.sh` put the CEF runtime on the library path. Evidence `docs/shots/p1-01-window.png`.
- Verified: `cargo build` OK (2 min first time: CEF download), `cargo clippy --all-targets -- -D warnings` clean, `cargo fmt`, `cargo test` 2 passed (file URLs with spaces/Unicode/`\\?\` prefix; UI folder lookup incl. missing). `tools/run-dev.ps1`: window titled "odyssey", 1280x800 DIP, black, 6 processes; `CloseMainWindow` → 0 processes left after 4 s; profile folder created.
- Edge cases (§1): missing UI files: handled + tested (lookup returns None; the window then shows a message, not a blank page). Paths with spaces / non-ASCII: handled + tested in `file_url`. Minimum window 800x600: set (exercised in P0-04). Closing: handled + tested. Two copies: CEF refuses the shared profile and the app exits with a message (single-instance hand-off is still a known gap). Corrupt settings, migrations: no settings yet. Linux: not built (pending).
- Not verified / unsure: Linux build; `tools/run-dev.sh` untested.
- Decisions proposed: none.
- Next: P1-02 engine trait.
- Commit: `cc1f88a` "P1-01: Cargo workspace, shell opens an empty window"  Pushed: yes

### 2026-11-19: Claude Code (VS Code) / Claude Opus 5.5: P0-08 and Gate 0
- Did: proposed D-007 (plain Rust + cef-rs) and the D-013 path (cargo-packager + cargo-packager-updater) in DECISIONS with reasons from P0-02 to P0-07. Checked Gate 0 (section above). P0-08 left [~] (needs the owner's acceptance).
- Verified: `cargo info cargo-packager` 0.11.8 and `cargo-packager-updater` 0.2.3 (Apache-2.0 OR MIT); cargo-packager README lists NSIS, WiX MSI, deb, AppImage, pacman. Web search on Tauri's CEF runtime found the Tauri v3.0.0-alpha.0 notes (`tauri_runtime_cef`, unstable runtime API) and a March 2026 issue about missing navigation controls; nothing newer found (marked unverified).
- Edge cases: not applicable (decision record only).
- Not verified / unsure: whether cargo-packager can ship CEF's runtime files and whether its updater verifies signatures; current Tauri CEF status.
- Decisions proposed: D-007 (Proposed), D-013 detail (Proposed).
- Next: P1-01 project layout on the Proposed D-007.
- Commit: `c3e9fef` "P0-08: propose plain Rust + cef-rs (D-007) and packaging path; Gate 0 check"  Pushed: yes

### 2026-11-19: Claude Code (VS Code) / Claude Opus 5.5: P0-07 (Windows)
- Did: `tools/bench/bench.py` (stdlib only; Windows Private Bytes + CPU via `Win32_Process`, Linux PSS + `/proc/<pid>/stat`, process tree + profile match, fresh temp profile deleted afterwards) and `tools/bench/urls.txt` (10 fixed public URLs). The P0-05 spike gained `--urls-file` (one window per URL) and `--profile-dir`; built in release. `docs/BENCHMARKS.md` with method, machine, versions, date and table.
- Verified: `python tools/bench/bench.py --list` finds Edge 157.0.4341.60 and the spike; Chrome, Brave, Firefox not installed (also checked the registry App Paths). Two rounds (60 s wait + 60 s idle each): Edge 10 tabs 1073.6 / 1028.8 MB, 6.4 / 4.1 % of one core; Edge 10 windows 1390.9 / 1377.0 MB, 3.1 / 4.0 %; spike 10 windows 974.6 / 934.2 MB, 1.6 / 3.0 %. A first spike run measured 0 processes: the script's glob put the CEF tarball on PATH instead of the CEF folder, so the spike couldn't start; fixed (pick the folder containing libcef) and rerun.
- Edge cases (§9 and §1): the bench never touches the owner's real browser profiles (fresh `--user-data-dir`/`-profile` in %TEMP%, deleted after); only processes in the launched tree or naming that profile are measured and killed (an already running Edge is left alone). Slow network: pages may still be loading at 60 s (same for every browser; noted). Paths with spaces: Program Files paths handled (passed as argv). Linux path written but untested.
- Not verified / unsure: Chrome, Brave, Firefox numbers; Linux; whether 10 windows vs 10 tabs is a fair proxy (both Edge layouts recorded to show the gap).
- Decisions proposed: none.
- Next: P0-08 (shell decision D-007 and D-013 path) on the evidence from P0-02 to P0-06.
- Commit: `5e5143b` "P0-07: memory/CPU benchmark script and Windows baseline"  Pushed: yes

### 2026-11-19: Claude Code (VS Code) / Claude Opus 5.5: P0-06 (Windows)
- Did: `spike/p0-06-drm/eme-probe.html` (MSE codecs + EME key systems) and `play.html` (Shaka Player 4.12.6 from jsDelivr, two public Widevine demo assets, Shaka's public license proxy), run in the P0-05 spike binary, served from `http://localhost:8765` (secure context). D-015 results table, new Q-08 (H.264/AAC), README.
- Verified: fresh profile `chrome://components` lists "Widevine Content Decryption Module" 0.0.0.0 New; after "Check for update" + 60 s: 4.10.3112.0 Up-to-date, `WidevineCdm/4.10.3112.0/_platform_specific/win_x64/widevinecdm.dll` (22,970,008 bytes) + `.sig` on disk. Shaka support page: `com.widevine.alpha` with `SW_SECURE_CRYPTO`, `SW_SECURE_DECODE`, `cenc`, `cbcs`, persistentState false; PlayReady/FairPlay null. MSE: avc1 false, mp4a false, vp9/opus/av1 true. Playback: Angel One VP9 768x576 currentTime 7.3 s, Sintel VP9 1280x546 currentTime 7.5 s, both `com.widevine.alpha`, not paused (`spike/shots/p0-06-widevine-playback.png`).
- Mistake caught: the first probe ran from `file://`, where EME is never allowed (opaque origin), so its "not available" result was discarded and the test redone over http://localhost and https.
- Edge cases: §1 slow/no network: the CDM download needs network; offline behaviour not tested. First run (no CDM yet): handled + checked by hand (DRM pages fail until the component arrives; we should tell the user, P4-06). Linux: not tested. §4 heavy pages/full-screen video: not tested here.
- Not verified / unsure: whether Widevine downloads without the manual click; VMP signing and HD robustness for commercial services; Linux.
- Decisions proposed: none; Q-08 added (owner decision on proprietary codecs).
- Next: P0-07 (memory baseline, `tools/bench/`, BENCHMARKS.md) on Windows.
- Commit: `5d3008b` "P0-06: Widevine works with VP9 on Windows; no H.264/AAC in stock CEF"  Pushed: yes

### 2026-11-19: Claude Code (VS Code) / Claude Opus 5.5: P0-05 (Windows)
- Did: `spike/p0-05-extensions` (P0-02 window + `--toolbar=` + `--then=`), `run.ps1`, `shot-main.ps1`. Fetched uBlock Origin Lite 2026.1116.1650, Bitwarden 2026.10.2 and Dark Reader 4.9.135 as CRX3 from Google's update service and unpacked them outside the repo (path-checked). Tested Chrome UI hiding, unpacked load, Web Store install, service workers, content scripts, DNR blocking, options and popup pages. Results table under D-003, README with problems, three new edge-case kinds in EDGE_CASES §10.
- Verified: build OK (into the shared `../p0-04-helm/target`), clippy clean. `chrome://extensions` lists all three enabled (`spike/shots/p0-05-extensions-page.png`). `--toolbar=normal`/`location` show no toolbar in our window. `chrome://serviceworker-internals` shows the three workers, uBOL RUNNING. Dark Reader darkened Wikipedia. With uBOL, `pagead2.googlesyndication.com/pagead/js/adsbygoogle.js` ended on `chrome-extension://<uBOL>/strictblock.html#...`; without extensions: `ERR_ABORTED`, no extension page. Popup/options pages as tabs: `load_end status=200`. Web Store: rejected Google's consent ("Reject all"), clicked "Add to Chrome", Chrome's prompt appeared, clicked "Add extension"; `Secure Preferences` then lists Dark Reader with `location=1 from_webstore=True`, and `Default/Extensions/<id>/4.9.135_0` exists.
- Privacy: two screenshots were redacted before commit (a card naming an extension a local program registered for Chrome; the profile path containing the Windows user name). Extension files are not in the repo.
- Edge cases (§10): MV2 install not tried (D-010 stands). Broad permissions shown at install: yes, Chrome's prompt lists them (checked by hand). Popup above pages: no popup button exists yet (P6-02). Crash/hang, update failure, removal clearing data: not tested (P6). New cases added to EDGE_CASES §10. §1: slow network (store consent page) and first run (fresh profile) checked by hand. Linux: not tested.
- Not verified / unsure: Google's terms for Web Store installs (Q-04); how to stop external-extension sources; which callback catches extension-opened tabs; persisting unpacked extensions via developer mode.
- Decisions proposed: none new (D-003 re-confirmed on Windows; table added).
- Next: P0-06 (Widevine) on Windows.
- Commit: `56f66b5` "P0-05: Chrome UI hidden and MV3 extensions on Windows"  Pushed: yes

### 2026-11-19: Claude Code (VS Code) / Claude Opus 5.5: P0-04b (Windows)
- Did: `spike/p0-04-helm/src/osr.rs` + `--mode=osr`: the Helm as an Alloy windowless browser (transparent background, 60 fps) painted with `UpdateLayeredWindow(ULW_ALPHA)` into a `WS_EX_LAYERED` popup owned by the main window; WndProc forwards mouse (move/leave/buttons/wheel), keys, focus and IME composition/commit; cursor via `on_cursor_change`; `WM_NCHITTEST` returns `HTTRANSPARENT` outside the pill so the shadow doesn't block the page; placement skips while the owner is minimised; Esc returns focus to the page. `ui/helm.html` gets a 48 px shadow margin and the design's shadow in this mode. `HELM_DUMP` writes the last frame so `shot-all.ps1` can composite it with real alpha. `windows-sys` 0.61.2 used directly (already in the tree through `cef`; no new crate). README section, D-009 result, PHASES `[~]`.
- Verified: build, `cargo clippy -- -D warnings`, `cargo fmt`, `cargo test` pass. `bench.ps1 -Modes osr` (5 runs; Windows 11, 125%, 144 Hz, debug): page scroll 144.4 fps p95 7.0–7.1 ms (one run 58.7 fps, not repeatable); Helm click/key to handler 0.36–0.40 / 0.51–0.58 ms avg; input to next Helm paint 25–48 ms avg, max 130–217 ms; `HELM_FPS=120` no better (33–37 ms). Click 20 DIP above the pill reached the page. Main moved +120,+40 → Helm moved 120,40; minimise hides it, restore shows it. Screenshot `spike/shots/p0-04b-osr.png` viewed by eye: anti-aliased pill and soft shadow over Wikipedia.
- Edge cases: §1 keyboard: typing reaches the field (11 keydowns logged), Esc hands focus back (coded; checked by the log only). Scaling: 125% only; DPI change handled in code (`notify_screen_info_changed`) but not tested across monitors. Window minimised/moved: handled + tested. Rapid input: 10 clicks at 200 ms handled. Both OSes: Linux not handled. §3 IME: not tested (no IME on this unattended run); composition is drawn in the page, candidate window placement not done. §12: owned popup, no taskbar button, not above other apps (handled + tested via owner check). Accessibility: not checked (off-screen UI may be invisible to screen readers; unverified).
- Not verified / unsure: IME, screen readers, Linux, shared-texture rendering, behaviour with two monitors at different scaling.
- Decisions proposed: D-009 stays Proposed, now with the P0-04b result.
- Next: P0-05 (Chrome UI hidden + MV3 extensions) on Windows.
- Commit: `fd848b3` "P0-04b: off-screen Helm in a layered window (Windows; awaiting Owner OK)"  Pushed: yes

### 2026-11-19: Claude Code (VS Code) / Claude Opus 5.5: P0-04 (Windows part)
- Did: `spike/p0-04-helm`: one CEF Views window with the web page as a Chrome style BrowserView (only normal child, inset 56 px) and our UI as Alloy style BrowserViews from local HTML (stand-in rail and Helm). `--mode=overlay` (Helm as `add_overlay_view`), `--mode=window` (Helm in a frameless CEF window parented to the main window, follows `on_window_bounds_changed`, clipped with `SetWindowRgn`), `--mode=none` (baseline). Helpers: `run.ps1`, `bench.ps1`, `resize.ps1`, `shot.ps1`, `shot-all.ps1` (composite of the spike's own windows via PrintWindow, never the screen). D-009 updated with results and a Proposed approach; D-003 risk note; P0-04b added to PHASES; findings in the spike README.
- Verified: `cargo build`, `cargo clippy -- -D warnings` clean, `cargo fmt`, `cargo test` (1 test: typed `javascript:` refused, URL vs search, percent decoding) pass. Ran every mode; screenshots `spike/shots/p0-04-overlay-win.png`, `p0-04-overlay-transparent-try.png`, `p0-04-window.png`, `p0-04-{overlay,window}-{min,max}.png`, viewed by eye. `bench.ps1` (Windows 11, 125%, 144 Hz, debug build): scroll 144.4 fps, p95 frame 7.10 ms, 0 frames over 25 ms in none/overlay/window; Helm click to handler avg 0.48 ms (overlay and window), key 0.71 to 0.74 ms avg. Window mode: main moved +120,+40 → Helm moved 120,40; minimise hides the Helm, restore shows it; Helm window has owner = main window.
- Checked in the sources: `cef_types_runtime.h` (one Chrome style BrowserView per window; windowless always Alloy), `cef_types.h` background_color (no transparency for windowed browsers), `cef_window.h` AddOverlayView, CEF master `browser_view_impl.cc` AddedToWidget (fetched from GitHub; the shipped 156 binary may differ in detail but its log line matched).
- Edge cases: §1 window sizes: minimum 800x600 DIP handled + tested (700x500 px request gave 1018x797 px); maximised handled + tested (Helm centred in the page area); snapped, full screen and two monitors not tested. Scaling: 125% only. Keyboard: Enter navigates, Esc leaves the address field; IME not tested (`isComposing` guard only). Window closed: graceful WM_CLOSE in the bench; owned Helm window closes with the main window. Both OSes: Linux not handled (pending). §4: full screen not tested; bottom-of-page content under the Helm is a known gap (P2-05); certificate, auth, dialogs not applicable to this spike. §12: frameless main window not part of this spike (P2-02); the owned Helm window has no taskbar button and is not always-on-top over other apps (handled + tested). §3: typed `javascript:` refused (handled + tested in the unit test).
- Not verified / unsure: option (a) not built (documented reason). Linux entirely. The new P0-04b approach is a proposal only. Swapping the single Chrome style page view for tabs is unverified.
- Decisions proposed: D-009 approach (Proposed, see DECISIONS); Q-06 fallback named.
- Next: P0-04b (off-screen Helm in a layered window). P0-04 stays [~] until the owner says yes and Linux is tested.
- Commit: `94812fb` "P0-04: Helm over page spike on Windows (awaiting Owner OK)"  Pushed: yes

### 2026-11-19: Claude Code (VS Code) / Claude Opus 5.5: P0-02
- Did: `spike/p0-02-cef-window`: a minimal Rust app on the `cef` crate (=156.7.0+156.0.31, tauri-apps/cef-rs) with one CEF Views window, `RuntimeStyle::CHROME`, 1280x800 preferred, 800x600 minimum, loading https://example.com (`--url=` overrides). Sandbox off. Profile in `%LOCALAPPDATA%\constellations-spike\p0-02`. `build.rs` embeds `app.manifest` (Windows 10/11 supportedOS, PerMonitorV2, longPathAware) through the MSVC linker; no extra crate. `shot.ps1` screenshots one process's window with `PrintWindow`. Findings are in the spike README.
- Verified: `cargo build` OK (the `cef-dll-sys` build script downloaded `cef_binary_156.0.31+ge603d97+chromium-156.0.8078.100_windows64_minimal`). `cef_version.h` gives `CEF_VERSION "156.0.31+ge603d97+chromium-156.0.8078.100"`. Ran the exe with the CEF folder on PATH: example.com renders, screenshot `spike/shots/p0-02.png` (1600x1000 = 1280x800 at 125% scaling). No Chrome toolbar or tab strip appears with Views + Chrome style. `cargo clippy -- -D warnings` and `cargo fmt --check` clean.
- Edge cases (§1): no network / unknown host: handled + tested (`--url=https://does-not-exist.invalid/` shows Chrome's error page with Reload, `on_load_error` logs ERR_NAME_NOT_RESOLVED; `spike/shots/p0-02-bad-url.png`). Window closed: handled + tested (WM_CLOSE: all CEF processes gone in 283 ms). Two copies: handled + tested to the spike level (the second copy's `initialize` fails, it prints a message and exits with code 2); URL hand-off to the running copy is a known gap. Display scaling: checked at 125% only (PerMonitorV2 manifest). Minimum window size: set to 800x600, not exercised by hand. Engine errors: logged to stderr. Paths with spaces / non-ASCII user names: not tested (profile path is passed as a UTF-16 CefString; unverified). Linux: not handled (P0-03, pending). Empty data, long text, huge counts, keyboard, settings: not applicable to a one-window spike.
- Found: without the manifest the GPU process crashed on start (exit 0x80000003) and Chromium fell back to software; with it, no GPU errors. Harmless log: Chrome style reads `Software\Google\Chrome\Extensions` registry keys of an installed Chrome.
- Privacy note: a first screenshot helper used CopyFromScreen and one capture caught another window on the desktop; that file was deleted before any commit and the helper now uses PrintWindow (this window only).
- Not verified / unsure: sandbox mode (cef-rs needs its bootstrap.exe layout on Windows); the cloned cef-rs `export-cef-dir` tool was not run (the harness refused to run code from a cloned repo), so CEF lives in the build folder rather than a shared CEF_PATH.
- Decisions proposed: none.
- Next: P0-03 is Linux (pending). Go on to P0-04 on Windows.
- Commit: `b1c79d1` "P0-02: CEF window spike on Windows"  Pushed: yes

### 2026-11-19: Copilot SDK in VS Code: P0-01a (Windows toolchains)
- Did: inspected the Windows toolchain; installed the official `OpenJS.NodeJS.LTS` winget package because Node/npm were missing; reused the CMake and Ninja copies already bundled with Visual Studio Build Tools and added their directories to the user PATH; recorded all Windows versions in `docs/STATUS.md`. At the owner's direction, split P0-01 into P0-01a (Windows, ticked) and P0-01b (Linux, pending) in `docs/PHASES.md`, with a prominent `* LINUX PENDING *` marker. No application code or project dependencies changed.
- Verified: `rustc --version`, `cargo --version` and `rustup show active-toolchain` reported 1.100.0 on stable-x86_64-pc-windows-msvc. `vswhere` resolved the C++ and Windows 11 SDK components to VS Build Tools 2022 17.14.37906.3; `cl` through `vcvars64.bat` reported MSVC 19.44.35230 for x64. With machine and user PATH loaded, `node --version` reported v26.13.1, `npm.cmd --version` 11.20.0, `cmake --version` 3.31.6-msvc6, `ninja --version` 1.12.1 and `python --version` 3.14.6. `winget list --id OpenJS.NodeJS.LTS --exact` confirmed the Node installation. Windows CIM/registry reported Windows 11 Home Single Language 26H2, build 26300.9974. `git diff --check` passed; `git fetch origin` and `git pull --rebase origin Constellations` reported the branch up to date; `git push origin Constellations` pushed `054e3ed` and the remote lookup matched it.
- Edge cases: §1 interrupted installation and slow/unavailable network: handled + checked by hand through winget's installer-hash verification and successful exit status; no partial install was left. Existing installations/data: handled + checked by hand by inventorying tools first and installing only missing Node.js; existing Rust, VS, Python, CMake and Ninja were preserved. Paths with spaces: handled + tested by quoting executable paths and verifying the VS-bundled CMake/Ninja PATH entries. Non-ASCII usernames and long paths: not handled (current path is ASCII; no long-path stress test applies to version checks). Both OSes: Windows handled + tested; Linux not handled because the owner deferred Linux setup. Runtime, UI, settings and web-content cases: not applicable to toolchain setup.
- Not verified / unsure: every Linux toolchain entry is unverified. PowerShell blocks `npm.ps1` under its current execution policy; `npm.cmd` is verified and no security setting was changed. No application build exists yet, so no build/test suite applies.
- Decisions proposed: none; the owner directed Windows-first work with Linux testing later.
- Next: read P0-02 and its edge cases, then build the Windows CEF spike. Resume P0-01b on the owner's Ubuntu dual boot later; the full P0-01 task and Phase 0 gate remain open.
- Commit: `054e3ed` "P0-01a: record Windows toolchains"  Pushed: yes. Completion record: `587eba6` "P0-01a: record completion"  Pushed: yes.

### 2026-11-19: Copilot SDK in VS Code: P0-00
- Did: initialized this checkout, recorded `origin/main` as `755c7e574bea0514effccee60893ba99b18cf820`, verified `origin/Constellations` started at `755c7e5`, checked out `Constellations`, removed Odyssey's 22 tracked files from this branch only, and added the handover, design, tools, `.gitattributes` and `.gitignore`.
- Verified: checkout had no untracked-file collisions; `git status --short` showed exactly the 22 expected Odyssey deletions plus the new project files. After pushing `9968cfb`, `git branch --show-current` printed `Constellations`, `git status --short` was empty, `git log origin/Constellations -1` showed the P0-00 commit, and `git ls-remote origin main` still returned `755c7e574bea0514effccee60893ba99b18cf820`.
- Edge cases: §1 interrupted operations and slow/unavailable network: handled + checked by hand by stopping on command failure and never rewriting pushed history. Paths with spaces: handled + tested by quoting the two Odyssey batch-file paths during `git rm`. Unexpected remote changes and untracked-file collisions: handled + tested by checking the remote tip and checkout result. Empty/missing data, long text, huge counts, repeated input, display scaling, window sizes, accessibility, settings/migrations, multiple instances, sleep/clock changes and engine/OS runtime errors: not applicable because P0-00 changes no application or UI code. Both OSes: not applicable to these Git metadata and documentation changes.
- Not verified / unsure: no application checks exist until P1-06; none apply to P0-00.
- Decisions proposed: none
- Next: P0-01; ask the owner whether Linux testing will use a VM or dual boot.
- Commit: `9968cfb` "P0-00: start Constellations with handover docs and design (Odyssey stays on main)"  Pushed: yes. This completion record is a separate same-task commit so the pushed hash is preserved.

### 2026-11-19: Claude (chat, claude.ai) / Claude Opus 5.5: handover
- Did: wrote AGENTS.md, CLAUDE.md, docs/ (STATUS, PHASES, EDGE_CASES, DECISIONS, DESIGN, HANDOVER), exported the approved design (`design/source` from the published canvas, byte-identical as checked by SHA-256), generated `design/preview` and `design/png` (2x), bundled the font, and added tools (make_previews, render_png, compare_png, visual_check).
- Verified: all 7 PNGs rendered and inspected by eye. The Private board uses its own lilac default accent. `visual_check.py --selftest` scores 0.0 on all 7 boards. `compare_png.py` calibration: identical 0, blur 0, 1 px shift 1.0, 2 px shift 8.1, different boards 22.5 or more (limit 5).
- Not verified: everything in DECISIONS marked unverified; `visual_check.py` can't run against the app until P1-04 exists.
- Known design flaw: the new-tab ring draws a "T" instead of a "+"; build the "+" (DESIGN.md §6).
- Also: checked the `Constellations` branch on GitHub: it was at `755c7e5` (same as `main`, Odyssey Phase 3). Dry-ran the P0-00 git steps (without pushing): they remove exactly Odyssey's 22 tracked files and add the handover.
- Next: P0-00.
- Commit: none (no repo yet; P0-00 creates it)
