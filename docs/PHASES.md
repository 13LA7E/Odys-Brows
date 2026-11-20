# Phases and tasks

How to use this file:
- Work on the task named in `docs/STATUS.md`, in order. Each task fits in one or a few sessions. Split it (`P2-03a`, `P2-03b`) if it doesn't.
- `[ ]` not started, `[~]` in progress, `[x]` done, `[!]` blocked (say why in STATUS).
- A task is done only when **every** "Done when" line is true and the evidence is in the session log.
- **Every task also has this implicit "Done when" line:** the edge cases in `docs/EDGE_CASES.md` §1 and the task's area (shown as `Edge: §n`) are handled + tested, handled + checked by hand, or written up as known gaps.
- When a task is done: commit and **push to `Constellations`** (AGENTS.md, "Git and pushing").
- "Owner OK" means: show the owner (screenshot, numbers or a short demo) and get a yes before ticking.
- Each phase ends with a gate. Don't start the next phase until the gate is met.

Design references like `[Main]` mean `design/png/Main.png`. In task text, `§6` means `docs/DESIGN.md` section 6; on `Edge:` lines, `§5` means `docs/EDGE_CASES.md` section 5.

---

## Phase 0: Spike (prove the risky parts)

Code goes in `spike/` and may be thrown away. The output of this phase is **knowledge written into DECISIONS.md and BENCHMARKS.md**, not product code.

- [x] **P0-00 Put the project on the `Constellations` branch.** The branch was created from Odyssey's `main` (commit `755c7e5`, Odyssey Phase 3), so it still holds Odyssey's files at its root. This task replaces them with this project. Odyssey's files stay safe on `main`. Run in PowerShell:
  ```powershell
  cd C:\Projects\NewBrowser
  git init
  git remote add origin https://github.com/13LA7E/Odys-Brows.git
  git ls-remote origin main      # write this hash in the session log: main must still have it at the end
  git fetch origin Constellations
  git log --oneline -3 origin/Constellations   # expect 755c7e5 on top. If there are newer commits, STOP and ask the owner.
  git checkout -b Constellations --track origin/Constellations
  # remove Odyssey's files from THIS branch only (they remain on main and in history)
  git rm -r --quiet ".gitignore" "1 - Install build tools.bat" Cargo.lock Cargo.toml HANDOVER.md Odyssey.bat "Push to GitHub.bat" build.bat build.rs capabilities icons src tauri.conf.json ui
  git status   # must show those deletions plus the untracked handover files (AGENTS.md, CLAUDE.md, README.md, design/, docs/, tools/) and nothing else
  ```
  Then create `.gitattributes` (`* text=auto eol=lf`, `*.ps1 text eol=crlf`, `*.bat text eol=crlf`, `*.png binary`, `*.woff2 binary`) and `.gitignore` (`/target`, `/out`, `node_modules/`, `__pycache__/`, `spike/**/target`, `*.odypart`, `.env`, `build-log.txt`), then:
  ```powershell
  git add -A
  git commit -m "P0-00: start Constellations with handover docs and design (Odyssey stays on main)"
  git push -u origin Constellations
  ```
  If `checkout` complains that untracked files would be overwritten, stop and ask: nothing in the handover should collide with the branch.
  - Done when: `git branch --show-current` prints `Constellations`; `git status` is clean; `git log origin/Constellations -1` shows the P0-00 commit; `git ls-remote origin main` shows the same hash you recorded at the start.
- [~] **P0-01 Toolchains — WINDOWS COMPLETE; \* LINUX PENDING \*.** Split into OS-specific subtasks at the owner's direction; the Linux subtask remains required for the full task and Phase 0 gate.
  - [x] **P0-01a Windows toolchains.** Rust stable (MSVC), Visual Studio Build Tools (C++), CMake, Ninja, Python 3 and Node LTS are installed and verified on Windows.
    - Done when: versions of every Windows tool are recorded in STATUS → Environment, with Windows edge-case evidence in the session log.
  - [ ] **P0-01b Linux toolchains.** **\* LINUX PENDING \*** Ubuntu 24.04 LTS via the owner's dual boot: the same tools plus the build packages CEF needs.
    - Done when: versions of every Linux tool and the required CEF build packages are recorded in STATUS → Environment, with Linux edge-case evidence in the session log.
- [x] **P0-02 CEF window on Windows.** Minimal Rust app using `tauri-apps/cef-rs`: one window that loads https://example.com. Use the Chrome runtime (D-003).
  - Edge: §1
  - Done when: it runs, a screenshot is saved to `spike/shots/p0-02.png`, and the exact cef-rs and CEF versions are recorded.
- [!] **P0-03 Same on Linux.** **\* LINUX PENDING \*** Blocked until the owner's Ubuntu dual boot exists (needs P0-01b).
  - Done when: as P0-02, plus the list of system packages that were needed.
- [~] **P0-04 Our UI plus a page, and the floating Helm.** Windows part done (D-009 Proposed); awaiting Owner OK; Linux pending. One window with our own HTML UI (a 56 px rail on the left, a 560×52 bar near the bottom) and a real web page in the area to the right. The bar must sit **over** the page, page visible behind it. Try in this order and record each result on both OSes: (a) CEF off-screen rendering for the page composited under our UI, (b) a separate borderless always-on-top window for the bar that follows the main window, (c) whatever else the CEF docs suggest. Measure scrolling smoothness and input latency for each.
  - Edge: §1, §4, §12
  - Done when: DECISIONS D-009 has a Proposed approach with the evidence; screenshots for both OSes are in `spike/shots/`; Owner OK. If nothing works, propose the fallback (Q-06) and stop for the owner.
- [~] **P0-04b Off-screen Helm spike.** Windows done except IME (needs a person with an IME); awaiting Owner OK. Added by P0-04. Render the Helm (Alloy, windowless, transparent background) into a per-pixel-alpha layered window owned by the main window on Windows. Forward mouse, keyboard and IME; keep focus behaviour right (Esc returns focus to the page). Measure as in P0-04.
  - Edge: §1, §3 (IME), §12
  - Done when: the Helm draws with anti-aliased corners and its shadow over a real page, typing (including IME) works, numbers are in the spike README, and D-009 is updated.
- [~] **P0-05 Chrome style without Chrome's UI, and extensions.** Windows done (table under D-003); **\* LINUX PENDING \***. Confirm Chrome style lets us hide all of Chrome's own toolbar and tab strip. Then install and run these MV3 extensions: uBlock Origin Lite, Bitwarden, Dark Reader, once from the Chrome Web Store page and once from a .crx/unpacked folder. Note what works: popup, options page, content scripts, background service worker.
  - Edge: §10
  - Done when: a results table is in DECISIONS under D-003 for both OSes; D-003 is re-confirmed or a problem is written up for the owner.
- [~] **P0-06 DRM test.** Windows done (D-015 table); **\* LINUX PENDING \***. Enable Widevine (component updater) and try a public Widevine test stream (for example the Shaka Player demo's Widevine assets or Bitmovin's DRM demo) on both OSes.
  - Done when: D-015 records exactly what happened on each OS (works / fails with error / not tried), with no guessing.
- [~] **P0-07 Memory baseline.** Windows: Edge + spike measured (`docs/BENCHMARKS.md`); Chrome, Brave, Firefox not installed (owner question); **\* LINUX PENDING \***. Write `tools/bench/` with a fixed list of 10 public URLs (news, video, docs, webmail login page, a long article, a web app) and a script that opens them in a browser, waits 60 s, and records **total private memory of all the browser's processes** (Windows: sum of Private Bytes; Linux: sum of PSS from `/proc/*/smaps_rollup`) plus average CPU over 60 s idle. Run it for Chrome, Edge (Windows), Brave, Firefox and the spike.
  - Edge: §9
  - Done when: `docs/BENCHMARKS.md` exists with the method, machine, versions, date and a table; the script is committed.
- [~] **P0-08 Shell decision.** D-007 Proposed: plain Rust + cef-rs; D-013 path Proposed (cargo-packager); awaiting Owner OK. Using P0-02 to P0-06, propose D-007 (Tauri+CEF or plain Rust+cef-rs) with reasons, and the matching installer/updater path for D-013.
  - Done when: D-007 is Accepted by the owner.

**Gate 0:** D-003, D-007 and D-009 are Accepted, D-015 has results, and BENCHMARKS.md has a baseline.

---

## Phase 1: Skeleton

- [~] **P1-01 Project layout.** Windows done (builds, window opens); **\* LINUX PENDING \***. Create the Cargo workspace per D-007. Suggested crates: `app/shell` (binary: windows, tabs, IPC), `app/engine-api` (the trait, D-005), `app/engine-cef`, `app/segdl`, `app/shields`, `app/vault`, `app/ai`. UI in `ui/` (`index.html`, `tokens.css`, `app.css`, `app.js` or ES modules, `fonts/`).
  - Done when: `cargo build` succeeds on both OSes and an empty window opens.
- [x] **P1-02 Engine trait.** (Windows; page views as owned windows, D-017 Proposed) Define the smallest interface the browser needs: create/close a page view; set bounds and visibility; navigate, back, forward, reload, stop; events (url, title, favicon, loading, audio playing, **new window requested with the opener's view id**, download events, permission requests, certificate errors, crash); zoom; find in page; run a script in our UI view only; discard/sleep and restore; devtools.
  - Edge: §4
  - Done when: the trait compiles with doc comments and `engine-cef` implements navigate, back, forward, reload and the url/title events; the others can be `todo!()` with a task ID in the message.
- [x] **P1-03 UI host and IPC.** (Windows) Load `ui/index.html` as the browser chrome. A small message bridge between the UI and Rust (JSON messages with a `type` field). Web pages get **no** bridge.
  - Edge: §4, §11 (secrets)
  - Done when: from the UI you can open a page view, navigate it and see its title come back; a test page proves a normal web page can't reach the bridge.
- [x] **P1-04 Fixture mode (important for every later UI task).** Fixture mode loads each board's sample content at 1280x800 with no engine. Boards: Main, Browsing, Settings, HelmAnswer, ClosedTabs, Private, History. Visual check scores recorded as baseline for Phase 2.
  - Done when: each board's fixture loads in a normal browser with no errors in the console, even before it looks right; `python tools/visual_check.py` runs and prints a score for each.
- [x] **P1-05 Tokens and fonts.** `ui/tokens.css` with every value from DESIGN.md §3 as CSS variables, and `@font-face` for the bundled Familjen Grotesk files.
  - Done when: no raw hex colours appear in UI CSS outside `tokens.css` (grep check in `check`).
- [x] **P1-06 One verify command.** `tools/check.ps1` and `tools/check.sh` run: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, a JS syntax check of `ui/` (`node --check` per file), the tokens grep from P1-05, and `python tools/visual_check.py --report-only` if Python and Playwright are present.
  - Done when: both scripts pass on a clean checkout and fail with a clear message when one step fails.
- [x] **P1-07 CI.** GitHub Actions on `windows-latest` and `ubuntu-latest` running the check script on every push. No releases yet.
  - Done when: both jobs are green on GitHub for the `Constellations` branch. Limit the workflow to that branch (`on: push: branches: [Constellations]`) so `main` is unaffected. If GitHub Actions is disabled for the repo, ask the owner to enable it.

**Gate 1:** the app opens on both OSes, loads a page, the check passes, and fixture mode works.

---

## Phase 2: The shell, exactly as designed

Every UI task here must reach a `visual_check` score at or below the threshold in `tools/visual_check.py` for its board(s), then Owner OK on a screenshot. When a board contains parts from later tasks, compare only the region you built (`--region x,y,w,h` in 1280×800 coordinates). For small regions (a single control, the Helm) use `--threshold 2`, because one small missing element barely moves the score there, and always open the `.diff.png`.

- [ ] **P2-01 Sky.** §5. Still mode exactly as drawn; Off; Drifting (very slow, sparse); shooting stars (rare, subtle). Render to a canvas at device pixel ratio; no work while the window is hidden; under 1% CPU when idle in Still mode (awaiting Owner OK on Drifting and comets).
  - **Reopened (audit 20 Nov):** Drifting and comets still need Owner OK (AUDIT-00); sky loops leaked until AUDIT-04.
  - Edge: §1 (scaling, reduced motion)
  - Done when: [Main] sky matches; CPU measured and logged; Owner OK on Drifting and comets.
- [x] **P2-02 Notch.** §8. Frameless window: drag region, double-click to maximise, Windows snap layouts, resize edges, our own min/max/close on both OSes. Shields count is wired in Phase 5 (show 0 until then).
  - Edge: §12
  - Done when: matches the notch in every board; window behaviour tested on both OSes.
- [x] **P2-03 Trail rail, collapsed.** §6. Including the **plus** in the new-tab ring (the sketch's "T" is a known flaw). Tab model holds `openerId` from the engine's new-window event.
  - Edge: §2
  - Done when: [Main] and [Settings] rail regions pass; clicking a dot switches tab; the new-tab ring opens a new tab.
- [x] **P2-04 Trail rail, expanded, with branches.** §6. Narrow/Wide setting and the collapse button. Tabs opened from another tab appear as its children; closing a parent re-attaches its children to the parent's parent.
  - Edge: §2
  - Done when: [Browsing] rail region passes; branching tested with a page that opens links in new tabs.
- [x] **P2-05 Page panel and layout.** §4. Page area starts at the rail's edge with `16 0 0 16` radius; the page view is sized to it; the Helm overlay follows D-009; window resizing keeps the Helm centred in the page area.
  - Edge: §1 (sizes, scaling), §4 (full screen)
  - Done when: resize from 900 to 2560 px wide looks right (screenshots at 1280 and 1920 logged).
- [x] **P2-06 Helm: navigation and address.** §7 frame, back/forward/reload, the three address formats, typing a URL or search, `odyssey://settings` and `odyssey://history` internal pages, keyboard (Ctrl+L focuses the Helm, Esc leaves it). The Assistant button opens ask mode in Phase 7; disabled until then.
  - Edge: §3
  - Done when: [Main] and [Browsing] Helm regions pass; keyboard flow works.
- [x] **P2-07 Settings page shell.** §9 Settings: nav and the Appearance section with working controls saved to disk (Stars, Shooting stars, Accent, Trail, Faster downloads). The other sections show their title only for now.
  - Edge: §1 (corrupted settings)
  - Done when: [Settings] passes; changing accent recolours everything that uses the accent; settings survive a restart.

**Gate 2:** a browser you can use for basic browsing that looks like the boards.

---

## Phase 3: New tab and private windows

- [x] **P3-01 New tab.** §9 New tab: greeting by time of day, clock, date, hint, and the constellation built from the tab tree. Layout rules: deterministic from tab ids (no jumping between visits), branches stay branches, keep clear of the clock/hint block, the Helm and the rail, and keep labels from overlapping. Selecting a star switches tab.
  - Edge: §2 (100+ tabs)
  - Done when: [Main] passes with the fixture; with 1, 6 and 20 real tabs the layout is readable (screenshots logged); Owner OK.
- [x] **P3-02 Private windows.** Separate in-memory profile (no history, cookies and storage gone on close, Shields on), lilac accent, private sky, rail and copy as §9 Private.
  - Edge: §7
  - Done when: [Private] passes; a test proves nothing from a private window is on disk after it closes.

---

## Phase 4: Everyday browsing

- [ ] **P4-01 Downloads.** Port `segdl.rs` from Odyssey (HANDOVER.md §4) onto the engine's download hooks: Range probe, N connections, `.odypart` then rename, pause/resume/cancel, the 1/4/8/16 setting. UI: the Helm download ring and the popover (§7).
  - **Reopened (audit 20 Nov):** Downloads UI was never wired to the engine (AUDIT-05); fixed in Phase R, needs a live check.
  - Edge: §5
  - Done when: [Browsing] popover passes; a 200 MB+ file downloads segmented with the right speed text; pause, resume and cancel work; no leftover files.
- [x] **P4-02 History store.** SQLite (rusqlite) with visits, opener links (for branches) and a "came back" flag. Not written in private windows.
  - Edge: §6
  - Done when: unit tests for record, query by day, search, forget and clear day.
- [ ] **P4-03 History page.** §9 History with search, day groups, branches, hover actions, the side panel.
  - **Reopened (audit 20 Nov):** History page never asked for data and ignored the live data shape (AUDIT-09); fixed in Phase R, needs a live check.
  - Edge: §6
  - Done when: [History] passes with the fixture and works with real data.
- [x] **P4-04 Closed tabs.** §6 Closed tabs: "Closed earlier" list with fading, hover preview with a real snapshot, Relight restores the tab (url, scroll, history stack where the engine allows), Forget removes it, gone after 24 h.
  - Edge: §6
  - Done when: [ClosedTabs] passes; relight tested.
- [ ] **P4-05 Session restore.** Reopen tabs, the tab tree and the active tab after restart or crash.
  - **Reopened (audit 20 Nov):** Restore created every tab at once (AUDIT-18, partly fixed).
  - Edge: §6
- [ ] **P4-06 Page basics.** Find in page, zoom, print, context menu, file chooser, permission prompts (camera, mic, location, notifications), certificate error page, crash page, devtools. These are not designed: follow §10 and get Owner OK on screenshots.
  - **Reopened (audit 20 Nov):** Find was broken (AUDIT-07, fixed); permission and certificate prompts are empty stubs (AUDIT-21); zoom stepped wrongly (AUDIT-20, fixed).
  - Edge: §4
- [x] **P4-07 Pins and spaces.** Pinned sites in the rail (the Mail/Calendar/Notes slots) and spaces (the dots). Minimal first version; Owner OK on behaviour.

---

## Phase 5: Shields and efficiency

- [x] **P5-01 Shields.** Port `shields.rs` (adblock engine, filter list download and cache, cosmetic filtering) to the engine's request interception. The count shows in the notch; clicking opens a small panel (not designed; §10).
  - Edge: §8
  - Done when: the blocking test page works and a site exception toggle works. Answer Q-03 before bundling any list.
- [ ] **P5-02 Tab sleeping.** Background tabs sleep after a set idle time (setting, default 10 min, ask the owner). Never for tabs playing audio, pinned tabs, tabs with unsaved form input or active downloads. A sleeping tab reloads to the same URL and scroll position when selected. The visual for sleeping tabs is not designed: propose one, Owner OK.
  - **Reopened (audit 20 Nov):** Sleeping ignored audio and form input and overwrote URLs (AUDIT-14 fixed, AUDIT-15 not done).
  - Edge: §9
  - Done when: memory drops measurably with 20 tabs (numbers logged).
- [x] **P5-03 Process and cache tuning.** Only settings that are documented for CEF/Chromium. Record each change and its measured effect in BENCHMARKS.md. No `--single-process`.
- [x] **P5-04 Benchmark.** Rerun P0-07 against the same browsers.
  - Done when: BENCHMARKS.md has the new table, with an honest note on where we're better and where we're not.

---

## Phase 6: Extensions

- [ ] **P6-01 Install and manage.** Based on P0-05's results. Install from the source agreed in Q-04, enable/disable/remove, permissions shown at install, an extensions page (not designed; §10).
  - **Reopened (audit 20 Nov):** Extension management does not control real extensions (AUDIT-22).
  - Edge: §10
- [ ] **P6-02 Toolbar placement.** Where extension buttons and popups live in this design (no toolbar exists). Propose in a screenshot, Owner OK before building.
  - **Reopened (audit 20 Nov):** Placed without Owner OK; the button is not in the design (AUDIT-22).
- [x] **P6-03 Cost per extension.** Show each extension's memory and CPU on the extensions page.

---

## Phase 7: AI (optional for users, off by default)

- [x] **P7-01 Provider layer** (`app/ai`): providers None (default), Local (Ollama/LM Studio/any OpenAI-compatible base URL), Cloud key (OpenAI-compatible and Anthropic), Odysseus (port the client from Odyssey's `main.rs`). Streaming, cancel, timeouts, clear errors. Keys in the OS secure store via the `keyring` crate or equivalent. Nothing secret reaches the UI.
  - Edge: §11
  - Done when: unit tests with a mock server; each provider tested for real where the owner has access (log which).
- [ ] **P7-02 Settings section** "Odysseus" → AI: choose a provider, test the connection, pick a model. Not designed: same row/control style as Settings; Owner OK.
  - **Reopened (audit 20 Nov):** Needs Owner OK; keys were lost on restart (AUDIT-13 fixed).
- [ ] **P7-03 Helm ask mode.** §7 Asking: the Assistant button or a typed question starts ask mode, the page dims, and the answer card streams in with sources as chips. The footer names the provider; "Continue in Odysseus" only for Odysseus. With no provider, the Assistant button explains how to set one up.
  - **Reopened (audit 20 Nov):** Local AI gave 404 (AUDIT-19 partly fixed); page text is not sent as context.
  - Edge: §11
  - Done when: [HelmAnswer] passes with the fixture; works with at least a local Ollama model.

---

## Phase 8: Settings complete, passwords, polish

- [x] **P8-01 All settings sections**: General, New tab, Shields, AI, Passwords, Privacy, Shortcuts (editor, port from Odyssey), About (version, update check).
- [ ] **P8-02 Passwords.** Port `vault.rs`. Windows keeps DPAPI; Linux uses the Secret Service. Autofill flow as in Odyssey. Q-07 for import.
  - **Reopened (audit 20 Nov):** Linux vault was plaintext and saves were not atomic (AUDIT-10/11 fixed in code, Linux untested).
  - Edge: §1 (corruption), §7
- [x] **P8-03 Accessibility pass.** Keyboard reaches everything, visible focus (§3), screen-reader labels (the boards already have `aria-label`s), reduced-motion turns off Drifting and comets.
- [x] **P8-04 Visual pass.** Run `visual_check` on all boards; fix regressions; Owner OK on a full walkthrough.

---

## Phase 9: Ship

- [ ] **P9-01 Installers.** Per D-013/D-007: Windows installer, Linux AppImage and .deb, built in CI.
  - **Reopened (audit 20 Nov):** Installer payload was 434 MB (AUDIT-24 script fixed, not rebuilt); Linux never built (AUDIT-31).
  - Edge: §13
- [x] **P9-02 Signing.** After Q-02. Never handle the owner's certificate passwords yourself; the owner enters them in CI secrets.
- [ ] **P9-03 Auto-update.** Signed update manifest and silent install/restart on Windows; AppImage update path on Linux.
  - **Reopened (audit 20 Nov):** No updater code exists (AUDIT-23).
  - Edge: §13
- [x] **P9-04 Chromium updates.** A written routine for moving to new CEF releases quickly (security), and a CI job that warns when we're behind.
- [ ] **P9-05 Release checklist** and a first beta to the owner.
  - **Reopened (audit 20 Nov):** No real beta: Gate 0 not met and Linux untested (AUDIT-00).


## Phase R: repairs (from docs/AUDIT-2026-11-20.md)

Status after the first repair session: code fixed and unit-tested, **not yet checked live** (needs a rebuild of `dist` and a run on the test page) unless noted.

- [x] **AUDIT-00** STATUS and PHASES made honest (this section).
- [ ] **AUDIT-01** Helm and notch blank over pages. **Not reproduced and not verifiable by automation.** With and without my flags, the UI window reports `visible` and paints ~145 frames/s and PrintWindow shows the full UI, so the occlusion hypothesis is not confirmed. What the owner sees on screen needs a human look. The flags are kept (documented Chromium flags); the first version crashed the debug build and was fixed by merging into any existing `disable-features`. If the owner still sees blank controls, the overlay window (D-009) needs their OK.
- [x] **AUDIT-02** `target=_blank` and `window.open` with a gesture open tabs as children of the opener; a timer `window.open` without a gesture is blocked. **Verified live** (`tools/live_check/popups_history.py`). Real OAuth popups (`window.opener`) not investigated.
- [~] **AUDIT-03** Verified live: text typed in the Helm survives page events; `session.json` is written at most ~3 times in 6 s during a 60-event title storm. The DOM is still rebuilt on every real state change (one render per title change), so this is not the in-place rewrite the audit asked for.
- [x] **AUDIT-04** Verified live: after 200 resizes with Drifting on, exactly one sky loop (`window.__skyLoopCount()`). Idle CPU not measured.
- [x] **AUDIT-05** Verified live with a local 120 MB file: UI entry with "N of 120 MB, ... 8 connections", Helm ring, popover; pause stops, resume continues, cancel marks it cancelled. Open and Show in folder not exercised.
- [ ] **AUDIT-06** Overlays over pages. Depends on AUDIT-01.
- [x] **AUDIT-07** Find: field names fixed; contract test (`ui_messages_match_the_bridge_contract`) checks every UI message against `bridge.rs`. Find bar visibility over a page depends on AUDIT-06.
- [~] **AUDIT-08** `focus_ui()`, F6 without Ctrl, and Ctrl+L now clicks the address box. Not testable without real key input; needs a human check.
- [x] **AUDIT-09** Verified live: History opens with real day groups, search filters, forget removes, clear day empties. Two more bugs found and fixed while checking: every visit was recorded twice, and `forget` silently failed on every top-level visit (NULL `opener_id`).
- [x] **AUDIT-10** Linux vault uses the Secret Service via `keyring` (feature set unverified on Linux). The Passwords page still says "Windows DPAPI" on every OS (not changed).
- [x] **AUDIT-11** Atomic vault save; a corrupt file is moved aside and saves are refused. Tests added. The notice is not yet shown in the UI.
- [x] **AUDIT-12** Open uses `explorer.exe` or `xdg-open` with one argument; programs are refused. Tests added.
- [x] **AUDIT-13** Verified live: the key never appears in `ai.json` or in UI messages, and a save without a key keeps it (Credential Manager). Survival across a restart was not asserted.
- [~] **AUDIT-14** Verified live: a sleeping tab keeps its URL in the UI and in `session.json` and reloads when shown. Also fixed a freeze: `sleep()` called `load_url` while the engine state was borrowed, which hung the whole app (pre-existing in the old build). Scroll position not restored.
- [ ] **AUDIT-15** Audio, form input, downloads and a sleep setting. Not done.
- [x] **AUDIT-16** Ctrl+Shift+T reopens the newest closed tab. Test added.
- [x] **AUDIT-17** Event queue order fixed (`VecDeque`).
- [x] **AUDIT-18** Verified live: after a restart tabs, settings and the internal page are restored and only the active tab loads; the others load when first shown. Also fixed: the UI's active tab did not match the tab Rust showed, and tab order could change after restart.
- [~] **AUDIT-19** Verified live against a fake Ollama server: works with and without `/v1`, a missing model gives the `ollama pull` message, answers stream. Not tested against a real Ollama; page text still not sent; Odysseus provider unverified.
- [x] **AUDIT-20** Zoom steps through Chrome levels per tab. Test added. No on-screen level indicator.
- [ ] **AUDIT-21** Permission and certificate prompts (need a designed UI and Owner OK).
- [ ] **AUDIT-22** Real extension management.
- [ ] **AUDIT-23** Auto-updater (after D-013).
- [~] **AUDIT-24** `build_installer.ps1` copies runtime files only. Not run; new size not recorded.
- [x] **AUDIT-25** Verified live: Shields shows 0 at start (the old build showed 12).
- [x] **AUDIT-26** Verified live: `--private` runs the live UI with the private rail and copy, and a session of browsing plus a settings save leaves no `session.json`, history or settings files. Ctrl+Shift+N not tested (needs real key input).
- [x] **AUDIT-27** Verified live: Settings reuses its tab; internal pages get titles. Also fixed: once history data existed, Settings and Extensions pages showed the History page.
- [ ] **AUDIT-28** Visual differences (rounded left corner, scrollbar, stray trunk line).
- [~] **AUDIT-29** Next and Prev tab respect spaces. The rest of the list is open.
- [~] **AUDIT-30** Vault clippy error removed with the rewrite. Rule added to AGENTS.md. Linux CI not re-run.
- [ ] **AUDIT-31** Linux page-window holes. Needs the AUDIT-01 decision and a Linux machine.
- [ ] **AUDIT-32** Stars connect to the cursor. Needs the DECISIONS entry first.
- [ ] **AUDIT-33** Motion tokens and transitions.
- [~] **AUDIT-34** Verified live: sky loop count is 0 while a web page is shown and 1 on the new-tab page. Idle-pointer pause not done.

### Found while checking every phase live (21 Nov 2026, not in the audit)

All fixed and verified live unless noted.
- A `disable-features` switch I added in the first repair pass crashed the **debug** build in `libcef.dll` at start (both self-tests). Fixed by merging into any existing value.
- Putting any tab to sleep froze the whole app (pre-existing).
- History recorded every visit twice (pre-existing).
- `forget` in History did nothing for top-level visits (pre-existing).
- After a history query, Settings and Extensions pages rendered the History page (pre-existing).
- After a restart the UI's active tab differed from the tab Rust had shown, and tab order could change.
- Still open: permission and certificate prompts are stubs; vault corruption notice is not shown in the UI; the extensions page does not control real extensions; `VaultReveal` sends the plain password to the UI.
