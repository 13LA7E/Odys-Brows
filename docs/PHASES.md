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
- [x] **AUDIT-01** Helm and notch were empty grey shapes over web pages. Reproduced with a real screen capture of the app window (BitBlt with CAPTUREBLT; the earlier DOM/PrintWindow checks could not see it). Cause: DirectComposition. `--disable-direct-composition` fixes it; `tools/live_check/screen_helm.py` checks it on screen. Before/after: `docs/evidence/2026-11-21/AUDIT-01-*.png`. Seen only on this machine (Intel + NVIDIA laptop). `CONSTELLATIONS_DIRECT_COMPOSITION=1` turns the switch off for comparison.
- [x] **AUDIT-02** `target=_blank` and `window.open` with a gesture open tabs as children of the opener; a timer `window.open` without a gesture is blocked. **Verified live** (`tools/live_check/popups_history.py`). Real OAuth popups (`window.opener`) not investigated.
- [x] **AUDIT-03** The window is built in cached regions (rail, panel, Helm, notch, popovers, new-tab page, sky) keyed by the data each draws, and reconciled in place; untouched nodes are never removed or reinserted. **Verified live** (`inplace.py`): Helm, sky, panel and notch persist across 50 unrelated updates, typed text survives, the rail is rebuilt only when its data changes. The download ring's progress is updated in place.
- [x] **AUDIT-04** Verified live: after 200 resizes with Drifting on, exactly one sky loop (`window.__skyLoopCount()`). Idle CPU not measured.
- [x] **AUDIT-05** Verified live with a local 120 MB file: UI entry with "N of 120 MB, ... 8 connections", Helm ring, popover; pause stops, resume continues, cancel marks it cancelled. Open and Show in folder not exercised.
- [x] **AUDIT-06** Overlays over pages: the UI reports its open overlays (downloads popover, answer card, Shields panel, find bar, closed-tab preview, prompt cards) and the page windows get a hole there. **Verified live** on Windows with a Win32 region check (hole under the card, none elsewhere, removed afterwards). Linux not done.
- [x] **AUDIT-07** Find: field names fixed; contract test (`ui_messages_match_the_bridge_contract`) checks every UI message against `bridge.rs`. Find bar visibility over a page depends on AUDIT-06.
- [~] **AUDIT-08** `focus_ui()`, F6 without Ctrl, and Ctrl+L now clicks the address box. Not testable without real key input; needs a human check.
- [x] **AUDIT-09** Verified live: History opens with real day groups, search filters, forget removes, clear day empties. Two more bugs found and fixed while checking: every visit was recorded twice, and `forget` silently failed on every top-level visit (NULL `opener_id`).
- [x] **AUDIT-10** Linux vault uses the Secret Service via `keyring` (feature set unverified on Linux). The Passwords page still says "Windows DPAPI" on every OS (not changed).
- [x] **AUDIT-11** Atomic vault save; a corrupt file is moved aside and saves are refused. Tests added. The notice now shows on the Passwords page (not checked live).
- [x] **AUDIT-12** Open uses `explorer.exe` or `xdg-open` with one argument; programs are refused. Tests added.
- [x] **AUDIT-13** Verified live: the key never appears in `ai.json` or in UI messages, and a save without a key keeps it (Credential Manager). Survival across a restart was not asserted.
- [x] **AUDIT-14** Verified live: a sleeping tab keeps URL and title, wakes to the same page and the same scroll position. Also fixed the freeze in `sleep()`.
- [x] **AUDIT-15** Verified live: tabs with audio playing, typed text or a download don't sleep; the idle time is a setting (Settings > General, default 10 min, Never possible). Audio is detected from inside the page (media elements and AudioContexts created after load) because CEF's audio handler is never called for this browser style; a context created before load is not seen.
- [x] **AUDIT-16** Ctrl+Shift+T reopens the newest closed tab. Test added.
- [x] **AUDIT-17** Event queue order fixed (`VecDeque`).
- [x] **AUDIT-18** Verified live: after a restart tabs, settings and the internal page are restored and only the active tab loads; the others load when first shown. Also fixed: the UI's active tab did not match the tab Rust showed, and tab order could change after restart.
- [~] **AUDIT-19** Verified live against a fake Ollama server: works with and without `/v1`, a missing model gives the `ollama pull` message, answers stream. Not tested against a real Ollama; page text still not sent; Odysseus provider unverified.
- [x] **AUDIT-20** Zoom steps through Chrome levels per tab. Test added. No on-screen level indicator.
- [x] **AUDIT-21** Permission prompts (camera, microphone, location, notifications, clipboard) and certificate errors now reach the UI as a quiet card above the Helm. **Verified live**: geolocation Block gives `PERMISSION_DENIED` in the page; a self-signed HTTPS site shows the card and Continue anyway loads it. These cards are not in the design: **needs the owner's OK on a screenshot.**
- [x] **AUDIT-22** The extensions page is real: enabled unpacked Manifest V3 extensions are passed to Chromium at start (`--load-extension`); **verified live** that a content script runs after a restart and not after disabling. Changes apply on the next start (the page says so). The invented 18.5 MB and 0.1% cost numbers are gone (cost per extension is not measured, so P6-03 stays open). Chrome's registry-registered external extensions are still read (unverified how to switch it off). The notch button is still not in the design: needs the owner's OK.
- [ ] **AUDIT-23** Auto-updater. **Blocked on D-013** (where updates are hosted and who signs them). No update code exists; an honest check-only version needs a manifest URL.
- [x] **AUDIT-24** The staging script now finds the CEF runtime and copies runtime files only; payload and installed folder are 420 MB (almost all `libcef.dll`). The installer is built by `tools/installer/install_local.ps1` (no NSIS here); the NSIS script is unchanged apart from the icon and was not run.
- [x] **AUDIT-25** Verified live: Shields shows 0 at start (the old build showed 12).
- [x] **AUDIT-26** Verified live: `--private` runs the live UI with the private rail and copy, and a session of browsing plus a settings save leaves no `session.json`, history or settings files. Ctrl+Shift+N not tested (needs real key input).
- [x] **AUDIT-27** Verified live: Settings reuses its tab; internal pages get titles. Also fixed: once history data existed, Settings and Extensions pages showed the History page.
- [~] **AUDIT-28** Done: rounded left corners on the page window, rounded bottom-left corner of the notch hole (design: 0 0 0 14px), the notch auto-hides over web pages (owner request 21 Nov: it covered pages' top-right controls; shows for 2.5 s, slides away, a 4 px hover strip brings it back, `notch_autohide.py`) and sits at the design position (shift 0), stray trunk line with zero tabs, a click on our window brings it forward (not checked with another app in front). Not done: the page scrollbar still runs under the notch hole.
- [~] **AUDIT-29** (autofill is now built, see Passwords below) Done and verified: Next/Prev tab per space, a web tab sent to an internal page stops running, closed tabs expire on read and survive a restart, `--profile-dir` now moves the shell's own files, Url sent once, Alt+F4 in a page closes the browser (not tested with a real key press). Not done: autofill (needs the owner's decision about secrets and pages) and `VaultReveal` (owner's decision).
- [~] **AUDIT-30** Vault clippy error removed with the rewrite. Rule added to AGENTS.md. Linux CI not re-run.
- [ ] **AUDIT-31** Linux page-window holes. The overlay-hole code is Windows only (`#[cfg(windows)]`); Linux needs its own version and a Linux machine to test.
- [x] **AUDIT-32** Stars link to the cursor, ported from Odyssey `ui/fx.js` with the same numbers (150 px reach, 9 nearest, 85 px star links, parallax 16 x 11, accent colour for the nearest line; the click ring and particles were removed at the owner's request on 21 Nov). Settings > Appearance toggle "Stars follow the cursor" (default on; not in the approved design, so fixture mode leaves it out); off for reduced motion, fixtures and Stars other than Drifting; no per-frame allocations. **Verified live** (`cursor_stars.py`: lit pixels near the cursor 219 -> 2969, 218 with the toggle off). Evidence: `docs/evidence/2026-11-21/cursor-stars*.{png,gif}`. Also fixed: the saved Stars setting was ignored after a restart.
- [x] **AUDIT-33** Motion on shared tokens (`ui/tokens.css`: `--dur-fast/base/slow`, `--ease-*`, `--rise`, `--press-scale`, hover washes; all in one place). `ui/motion.css` animates only opacity, transform and colours, only on persistent nodes. Done: hover and press washes and press scale; cards and popovers rise/slide in once and fade out before leaving the DOM; new stars and branches animate once and keep running across rail rebuilds; active ring fades; rail width eases (page window waits when collapsing); page and Settings-section cross-fades; smooth download ring with a completion pulse; thin loading line under the Helm. Over a web page cards only slide (a native page can't blend with the UI) and the hole follows them. Off in fixture mode (visual check unchanged: 0.0 Main, 3.1 Settings); reduced motion keeps opacity fades only. **Verified live** (`motion.py`, 13 checks) with GIFs in `docs/evidence/2026-11-21/`. Not done: a closed tab's star does not fade out (the rail is rebuilt, so there is nothing left to animate); no Reduce motion setting (only if you want one).
- [x] **AUDIT-34** The sky stops when nobody can see it: paused on web pages, after 30 s without activity, and when the window is hidden. Idle drift is drawn at 5 fps by a timer (30 fps while the pointer moves), because a requestAnimationFrame callback on every refresh kept the GPU process at ~86% of a core. Release build, whole browser, one core = 100%: Still idle 2.7-6.6%, Drifting idle 10% (was 120%), Drifting paused after 30 s 2.6%, web page shown 2-5%. While the pointer moves constantly (synthetic CDP events at 60 Hz, which add their own cost) about 80-90%: real use is lighter but still the heaviest case.

### Found while checking every phase live (21 Nov 2026, not in the audit)

All fixed and verified live unless noted.
- A `disable-features` switch I added in the first repair pass crashed the **debug** build in `libcef.dll` at start (both self-tests). Fixed by merging into any existing value.
- Putting any tab to sleep froze the whole app (pre-existing).
- History recorded every visit twice (pre-existing).
- `forget` in History did nothing for top-level visits (pre-existing).
- After a history query, Settings and Extensions pages rendered the History page (pre-existing).
- After a restart the UI's active tab differed from the tab Rust had shown, and tab order could change.
- Still open: permission and certificate prompts are stubs; vault corruption notice is not shown in the UI; the extensions page does not control real extensions; `VaultReveal` sends the plain password to the UI.

### Second owner request (21 Nov 2026): D-024 and the polish

- The owner approved D-024 in the chat and asked for it to be committed first. The three rule edits (AGENTS.md first Hard rule, DESIGN.md section 10 Motion bullet, DECISIONS.md D-024) are made in the working tree but **committing them was blocked**; they are left uncommitted for the owner to commit. The code commits do not depend on that commit.

### Passwords like Brave (owner request, 21 Nov 2026)

- [x] **Save prompt:** a script in each page reports a submitted login form (submit, a submit button, or Enter in the password field); the browser takes the site from its own record of the tab's URL, never from the page. A card above the Helm offers Never / Not now / Save (or Update when the site and username are saved with a different password). Not in private windows; off with the switch "Offer to save passwords"; a per-site Never list (shown and editable on the Passwords page).
- [x] **Autofill:** only on a real, trusted click on a login field (a page script's `click()` or `focus()` does nothing), only on https or this machine, one saved login fills at once, several show a chooser card with usernames only. Switch "Autofill passwords".
- [x] **Passwords page:** title, the two switches, Add / Import / Export, the list (Reveal, Delete) and the Never list. No descriptive text.
- [x] **Import / export:** CSV in the Chrome / Brave layout (`name,url,username,password,note`); import reads Chrome, Brave, Edge and Firefox files by column name, skips rows without a site or password and logins that already exist, and refuses other files (limit 5000 rows, 240 KB). Export writes `constellations-passwords-<time>.csv` to Downloads after a confirmation (plain text).
- **Verified live** (`tools/live_check/passwords.py`, 18 checks; `passwords_page.py`, 4 checks) and with 4 new vault unit tests. The password is never sent to the UI (checked), only into the page the user clicked.
- **Needs the owner:** the AGENTS.md rule says never pass passwords to the UI or to web pages. Autofill necessarily puts a password into the page the user chose to fill, and Reveal still sends a password to the UI. The rule should get an exception line; I did not edit AGENTS.md. Known limits: login forms inside iframes are not detected or filled; no OS-password prompt before Export or Reveal; no breach check; no auto sign-in.
- [x] **Windows verification for Reveal and Export (owner request, 21 Nov):** both ask Windows to verify the user first (Windows Hello PIN, fingerprint, face or password; if Hello isn't set up, the classic "enter your Windows password" prompt checked with `LogonUserW`). The password is decrypted in Rust but only sent to the UI after verification; a cancelled check shows "Not verified" and sends nothing, Export writes nothing. Revealed passwords hide again after 30 s or with Hide. Autofill does not ask (as in Chrome and Brave). **The real Windows dialogs were not exercised by automation** (a script can't click them): debug builds honour `CONSTELLATIONS_TEST_AUTH=allow|deny` for the live checks; release builds never do. Needs a hand test once. Linux: no OS verification yet (Reveal and Export work without it): needs testing on Linux.
- [x] **Search engine setting:** the Helm always searched DuckDuckGo; Settings > General now decides (DuckDuckGo, Google, Bing). Verified live. Other General controls are not wired to anything: On startup (the app always restores tabs), Downloads location (display only), and the show-clock / trail-hint / Do-Not-Track settings.
- [x] **Settings pages scroll:** the settings content is its own scroll area; History and other internal pages scroll too (`settings_fixes.py`, 11 checks).
- [x] **Helm auto-hides and the notch and Helm animate (owner request, 21 Nov):** over a web page the address bar shows for 4.5 s, then slides down out of the way; a 4 px strip at the bottom (and Ctrl+L, typing, asking, an open download or a new address) brings it back. Both bars now slide over `--dur-slow` (320 ms, one token). The notch's slide was invisible before (black over black, then the hole snapped shut): the hole in the page window now follows each bar frame by frame, so the page shows through as it moves (notch hole 33 -> 4 px in 22 steps; Helm passes 14 positions). The Helm's hole now comes from the UI like the other overlays (the fixed Helm hole in `engine-cef` is gone). Live: `helm_autohide.py` (15), `notch_autohide.py` (9), `screen_helm.py` (real screen, strips hovered first); frames `docs/evidence/2026-11-21/autohide*.{gif,png}` (slowed to 1.5 s).

### Stage F: customisation parity with Odyssey (owner request, 21 Nov; see docs/AUDIT-2026-11-20.md, Stage F)

Order: settings storage first, then 38, 40, 41, 35, 36, 37, 39, 42. Each option is checked against Odyssey's code (`C:\Projects\Odyssey\ui\app.js`, `ui\fx.js`) before it is ported, with a screenshot of each new Settings section to the owner first.

- [x] **STORE Shared settings storage.** One schema with defaults and validation, saved in the shell's `settings.json` with atomic writes, every option applying live and updating in place; unit tests for default, save, reload and invalid value. Reuses what exists (search engine, motion, cursor stars, sleep time, password switches).
- [x] **AUDIT-38** General options (search engines + custom engine, restore tabs, link target, download connections).
- [x] **AUDIT-40** Privacy options (clear history, questions, downloads, site data on exit; site icons).
- [x] **AUDIT-41** Changeable keyboard shortcuts.
- [x] **AUDIT-35** Themes and a theme editor (dark presets first; light themes only with owner approval).
- [x] **AUDIT-36** Sky options (extra sky types, sliders, low power).
- [x] **AUDIT-37** New-tab page options.
- [x] **AUDIT-39** Interface options (density, radius, scale, rail side, Helm buttons, answer card).
- [x] **AUDIT-42** Motion options (Reduce motion control; speed multiplier only if wanted).

**Stage F progress (21 Nov): owner answers are in D-025 (dark only; rail left; new tab = clock options and wallpapers only; no window blur/opacity).**

- **STORE done:** `app/shell/src/settings.rs` is the one schema (key, allowed values, default). Load and save both go through `normalize`: invalid or missing values fall back to their default, unknown keys are dropped, oversized or non-object files become defaults; it never fails. Kinds ready for later options: choice, bool, integer choice and range, float range, text, custom sanitiser. A test checks that `DEFAULT_SETTINGS` in `ui/settings.js` matches the schema. 6 unit tests (defaults, junk, round trip, invalid one by one, custom engine, UI defaults). Existing settings reused (not duplicated): stars, shootingStars, starsFollowCursor, accent, trail, searchEngine, connections, sleepMinutes, offerSavePasswords, autofillPasswords.
- **AUDIT-38 done** (checked against Odyssey `app.js` `SEARCH_ENGINES`, `customEngine`, `restoreTabs`, `linkTarget`, `dlConns`): engines DuckDuckGo, Google, Brave, Bing, Startpage, Ecosia and Custom (http or https and `%s` or `{query}`; anything else is rejected in the UI and again in Rust); On startup is now real ("Restore tabs" or "New tab"; before it was saved but ignored); "Open links from History and answers" (New tab or Same tab; answer sources are clickable); Faster downloads (1, 4, 8, 16) stays where the design has it (Appearance) and is applied to `segdl` on load and save. Odyssey's "answers on web results" and "assistant page" belong to the AI section and are not part of this. **Verified live** (`general_settings.py`, 16 checks): junk settings cleaned, every engine through the real control, custom engine and its refusal message, restore vs new tab across restarts, link target from History. Screenshot: `out/live/settings-general-00.png`.
- Found and fixed on the way: History rows never showed Open or Forget in the live UI (only the design's pre-hovered row did); they appear on hover now.
- **AUDIT-40 done** (checked against Odyssey `renderPrivacySettings` and `clearOnExit`): switches for **Browsing history** and **Cookies and site data** under "Clear when Constellations closes", a working **Global Privacy Control** switch, and "Clear history now". The clearing runs at a safe point: after the engine has shut down (files are locked while it runs) and again at the next start when the last run didn't end cleanly (a `running.lock` file marks a run; it is removed only after a clean exit whose clearing fully worked). History covers our history database, the closed-tabs list, and Chromium's own History, Favicons, Top Sites and suggestion files; site data covers cookies, local and session storage, IndexedDB, service workers, caches and related files (Chromium's names; missing ones are skipped; **names are unverified for future Chromium versions**). Never touched: settings, saved logins, open tabs. GPC sends `Sec-GPC: 1` with http(s) requests (it was a switch connected to nothing before). **Not ported, on purpose:** Odyssey's "recent questions", "downloads list" and "load site icons" toggles, because Constellations has no stored question list, no saved downloads list and loads no site icons: the switches would do nothing. Say so if you want those features first. **Verified live** (`privacy_clear.py`, 17 checks): switches off leaves everything; clean exit with both on removes the history database, closed tabs and the cookie database and leaves settings and the open tab; a restart finds no cookie or local storage; site-data-only leaves history; a killed run is cleared at the next start; the server sees `Sec-GPC: 1` and, with the switch off, no header. Unit tests: 5 for `privacy.rs`, 1 for quitting. Screenshot: `docs/evidence/2026-11-21/settings-privacy.png`.
- **Bug found and fixed on the way:** after a normal close (close button, Alt+F4, taskbar) the saved session was empty, because each page closing with the window removed its tab from the session (earlier restore checks killed the process instead). The engine now announces `AppClosing` first; the shell saves the session once and then ignores the pages closing with the app (they also no longer land in the closed-tabs list).
- **AUDIT-41 done** (Odyssey has a shortcut editor in `ui/app.js`; its command list was used as the model, the rules are new): Settings > Shortcuts lists 17 commands (new tab, private window, close/reopen/next/previous tab, address bar, find, history, settings, reload, back, forward, zoom in/out/reset, print, developer tools) with their keys, **Change** (press the new keys; Esc cancels), **Reset** per command (shown only once changed) and **Reset all shortcuts**. All rules live in one place, `app/shell/src/shortcuts.rs`, so the Settings page, our own window and page windows agree: a combination needs Ctrl or Alt (or is an F-key); editing keys (Ctrl+C/V/X/A/Z/Y) and Windows keys (Alt+F4, Alt+Tab, Ctrl+Shift+Esc...) are refused with a message; a key another command already uses is refused and the message names that command; a default key whose command was moved is **swallowed** (so F5 does not still reload through Chromium after Reload moved). Alt+F4 (quit) cannot be changed. Changes are saved in `settings.json` (`shortcuts`, only the changed commands) and work at once in page windows (engine key hook gets the table) and in our window (`ui/shortcuts.js` maps the physical key, so it is the same on every keyboard layout). Not changeable in a private window (nothing is saved there). **Verified live** (`shortcuts.py`, 23 checks, with real key presses via SendInput): defaults, Ctrl+T in a page window and in our window, rebinding reload to Ctrl+K and pressing it, F5 and Ctrl+R no longer reload, conflict / editing key / bare letter / another command's default refused with messages, Change + Esc, the conflict message under the row, saved in settings.json, still bound after a restart and working in a page window straight away, Reset all. Unit tests: 3 new in `shortcuts.rs` (rules, binding table, swallowed defaults), the schema test covers `shortcuts`. Screenshot: `docs/evidence/2026-11-21/settings-shortcuts.png`. **Not verified:** Linux key codes (the table uses Windows virtual-key codes, which CEF also reports on Linux: unverified); AltGr layouts.
- **Noticed, not caused by this task:** the `ClosedTabs` board scores 8.9 in `visual_check` (limit 5) at the parent commit too; the diff is the serif article text in the fake web page (font substitution on this PC), not our UI. Needs a look by whoever next touches fixtures.
- **AUDIT-35 done** (checked against Odyssey `THEME_PRESETS`, `openThemeEditor`, `importThemeFromJson`; harmony tools and light themes left out, see D-025): Settings > Appearance has a **Theme** row: Default (the approved design, adds no overrides) and six dark presets (Void, Nebula, Aurora, Lagoon, Ember, Graphite), plus your own. A theme is six colours (background, cards and menus, pages, text, accent, stars); strong/soft/muted text and the text on the accent are worked out from them. **Editor:** New theme / Edit / copy a preset / Delete, a colour picker and a hex box per colour, live preview, Cancel undoes it; saving is refused with a plain sentence when the colours are light (dark only), the text is hard to read (contrast under 4.5) or the accent is too dark to see. **Export / Import** as text (Odyssey copies to the clipboard; here it appears in a box you can copy from, so it works without clipboard permission); Import validates every colour and never runs anything from the text. Saved in `settings.json` (`theme`, `customThemes`, at most 12, each re-checked in Rust `clean_custom_themes`). A theme's accent wins over the accent swatches (the row says so). Private windows keep their own look. The sky uses the theme's star colour. Fixture mode leaves the rows out so the Settings board still matches the design (3.1). **Verified live** (`themes.py`, 21 checks): bad saved themes dropped; custom theme applied at start; Default adds nothing; preset paints and saves; back to Default removes every override; unreadable and light colours refused; live preview and Cancel; save, export, three kinds of bad import refused, good import, delete; still applied after a restart. 1 new Rust unit test. Screenshot: `docs/evidence/2026-11-21/settings-themes.png`. **Not verified:** web pages are never recoloured (only our own pages), by design; the star colour of the tab trail dots is not themed.
- **AUDIT-36 done** (checked against Odyssey `fx.js` `cfg` and its sky settings): Settings > Appearance gets **Star density** (25, 50, 75, 100%), **Drift speed** (Slow, Normal, Fast; only while Stars is Drifting), **Shooting star rate** (Rare, Normal, Often; only while Shooting stars is on) and **Low power** (a still sky: no drifting, comets or cursor lines, so no animation loop runs at all). With Stars Off none of them show. Defaults reproduce the design exactly (100%, Normal, Normal, off; the Settings board still scores 3.1 and the options are left out of fixture mode). **Not ported, on purpose:** Odyssey's extra sky types (aurora, borealis, warp, nebula glow), twinkle and parallax strength: aurora and nebula are glows and gradients, which the design forbids (AGENTS.md "no gradients, glows or glitter", D-024 allows only quiet motion). Say so if you want any of them. "Sky behind pages" is not needed: web pages cover the sky and it already pauses there (AUDIT-34). **Regression found and fixed:** shooting stars had stopped appearing at all since AUDIT-32 (my edit removed the comet spawning code); restored, with the new rate. **Verified live** (`sky_options.py`, 14 checks): 50% and 25% keep half and a quarter of the stars, 100% is the full 170, Fast and Slow change the drift, Low power runs no loop, Stars Off, a comet really crosses (Often), rows appear only when useful. Also `sky_idle` 3/3, `cursor_stars` 4/4. Screenshot: `docs/evidence/2026-11-21/settings-sky.png`.
- **AUDIT-37 done** (checked against Odyssey `renderHomeBackground`, `clockFormat`, `greetingName`, `wallpaperDim`, `wallpaperBlur`, `handleWallpaperFile`; pinned sites, search-box styles, suggestions and recent questions are left out per D-025): Settings > New tab now has real options (the old two switches were saved nowhere): **Clock and date** on/off, **Clock format** (24-hour, 12-hour), **Greeting** on/off, **Your name** (added to the greeting, max 30 letters, shown as plain text), **Tab trail hint** on/off, and **Wallpaper**: choose one PNG, JPEG or WebP (4 MB at most), with **Dim** (0 to 80%) and **Blur** (Off, Light, Medium, Strong), **Change** and **Remove**. The wallpaper is shown on the new-tab page only (not on web pages, not in private windows), right of the rail. It is saved in the profile folder (`wallpaper.png|jpg|webp`), never in `settings.json`; Rust (`wallpaper.rs`) checks the type, the size and that the file really starts like that image type, writes atomically and ignores a damaged file. With no wallpaper nothing is drawn and the dim/blur rows are hidden. Defaults equal the design (Main board 0.0). The bridge allows one message above 256 KB (the wallpaper, up to 6 MB; anything else stays at 256 KB). New dependency: `base64` in the shell crate (already used by the vault) to check the image. **Verified live** (`newtab_options.py`, 27 checks): defaults, 12-hour, name added and a markup name shown as text, each switch removes its line, a PNG saved and drawn with dim 40% then 80% and blur 16px, HTML-as-PNG, SVG, text and a too-big file refused with a message while the old wallpaper stays, a 3.9 MB image chosen in the real file input is saved and a 4.3 MB one is refused, Settings shows Change/Remove/Dim/Blur only with a wallpaper, not drawn on a web page, still there after a restart, Remove deletes the file, a damaged file is ignored. 4 new Rust unit tests (3 in `wallpaper.rs`, message limits). Screenshot: `docs/evidence/2026-11-21/newtab-wallpaper.png`.
- **AUDIT-39 done** (checked against Odyssey `cornerRadius`, `uiScale`, `answerFont`, `answerFontSize`, `answerWidth`, toolbar order). Settings > Appearance, new groups **Interface** and **Answers**: **Corner roundness** (Sharp, Default, Round; every radius token scales, and the places that had a fixed pixel radius now use the tokens, with the same default values), **Interface size** (90, 100, 110, 125%; the whole interface scales with CSS zoom, and the page windows follow: the rail width sent to Rust and the Helm and card holes are in screen pixels), **Back and forward buttons** and **Reload button** (show or hide in the Helm), **Answer font** (Sans, Serif, Mono), **Answer text size** and **Answer width** (Narrow 480, Default 560, Wide 680). Default for every option reproduces the design (Main 0.0, HelmAnswer 4.1, Settings 3.1; the options are left out of fixture mode). **Left out, recorded in D-026 (Proposed):** density and button reordering (fixed-coordinate layouts), plus the items D-025 already excluded. **Verified live** (`interface_options.py`, 18 checks): no overrides at default, Helm radius 26 / 39 / 4 px, nav and reload hidden, answer card 680 px, Mono, 17.5 px then back to 560 px and 15.5 px, 125%: Helm 700 px on screen, nothing overflows, the reported hole equals the Helm on screen and the page window starts at 70 px (56 x 1.25); 90%: 504 px; 100% leaves no zoom behind. Screenshot: `docs/evidence/2026-11-21/settings-interface.png`. **Not verified:** very small windows at 125%; Windows display scaling above 100% together with Interface size (only 100% display scaling was available).
- **AUDIT-42 done** (checked against Odyssey `reduceMotion`, `OdyFX.setStatic`): **Reduce motion** in Settings > Appearance > Interface: **Auto** (default, follows the system setting), **On** (cards and pages only fade, buttons don't shrink when pressed, the rail and download ring don't animate, and the sky is a still picture with no loop, no comets and no cursor lines), **Off** (full motion even when the system asks to reduce it). The existing system-setting rules in `motion.css` now apply for Auto and On through the same rules; the sky asks one function, so Auto/On/Off agree everywhere. Defaults leave the design untouched. **Not built:** a motion speed multiplier (the plan said only if you want it; durations are tokens in `tokens.css`, so it is a small job if wanted). **Verified live** (`motion_options.py`, 10 checks, with the system preference emulated through the debugging connection): Auto and full motion, Drifting runs a loop, On makes cards fade and stops the loop, Auto with a "reduce" system fades and Off with a "reduce" system keeps motion, back to Auto, the Settings row exists. Also `motion.py` 13/13.
- **Regression found by the full re-run, fixed:** my AUDIT-41 commit removed `AppClosing` from the shell's event list, so a normal close lost the saved tabs again. `privacy_clear.py` caught it (it failed on "open tab restored"); fixed in its own commit and 17/17 again. If you installed a build made from `0691652`, `d19806f`, `919c3ab` or `265e196`, rebuild.
- **Spaces, rail overlap, extensions (owner report 21 Nov).** (1) With the rail expanded the new-tab clock and hint drew on top of it: they now start at the rail width plus 64 px (same place as before when the rail is collapsed; the Main board is unchanged) and slide with the rail. (2) **Spaces (Home, Work, Personal) did not work:** every tab showed in every space, switching never hid the old page, an empty space left the old page showing, and a restart forgot which space you were in. Now each tab carries its space; the rail and new-tab trail list only the current space's tabs; an empty space opens with a new tab; switching back returns to the tab you were on in that space; the saved space is restored. `spaces.py` 11/11 live. (3) **Extensions page:** a Chrome Web Store button (opens the store in a tab) and **Pin / Unpin** per extension. A pinned, enabled extension gets a button in the notch, and the notch **grows** with each one; the hole the page windows leave for the notch follows its real width (it was a fixed 224 px). Pins are saved (`pinnedExtensions`, max 24 ids) and survive a restart; a disabled extension leaves the notch. The notch button opens the Extensions page: extension popups are not built. `ext_pin.py` 10/10 live. Also run: `interface_options` 18, `notch_autohide` 9, `restore` 8.
- **Assistant polish (owner report 21 Nov; items 1 to 8 and extras).** **1 Thinking animation:** while the assistant works, three stars link up and pulse with a "Thinking" label, in the Helm card and on the chat page; a step line ("Searching the web for ...", "Reading ...", "Opening ... in a new tab") shows what it is doing, with a pulsing dot on the current one. **2 Order:** the card now shows your question first, then steps, then the answer, and the "Answered by local AI" footer **only after the answer is complete** (it used to sit under "Thinking..." from the start). Answers are rendered as Markdown (bold, lists, code, links) with plain DOM nodes, so the raw `**` in your screenshot is gone and nothing the model writes can become markup (`ui/markdown.js`). **3 History:** every conversation is saved (`chats.json` in the profile: at most 100 chats, 200 messages each, 20,000 characters per message, written atomically; a damaged file is an empty history) and listed on the new assistant page with open, delete, and "Clear all history" (asks twice). "Clear history on exit" also deletes it. Private windows save nothing. **4 Mode switch:** **Ctrl+J** (changeable in Settings > Shortcuts as "Ask the assistant") toggles the Helm between the address bar and asking; the star spins in, an "Ask" label slides in, and the address bar fades back when you return. **5 Browser control, partly:** the assistant can search, read a page and **open a page in a new tab**; it cannot click, type, fill forms or read your open pages (no extension-style control yet). **6 Dedicated chat:** `odyssey://chat` (the rail's Odysseus star opens it too, tab title "Assistant"): history on the left, conversation in the middle, Enter to send, Shift+Enter for a new line, Stop button while it works, empty state with three suggestions, "Continue in chat" button on the Helm answer card. **7 Search:** tools are asked for as one JSON line, so they work with any local or cloud model: `search` (DuckDuckGo's plain page, top 5 results shown as source chips), `read_page` (text of a public page, 6,000 characters), `open_tab`; at most 5 tools per question; a Settings switch "Search and open pages" (AI section, on by default) turns it off. Safety: search results and page text are marked untrusted in the prompt; only public http(s) addresses are used (no localhost, private or link-local addresses, no user names in URLs, redirects re-checked); pages are fetched without cookies; the tool JSON is never shown. Not covered: a public name that resolves to a private address (DNS rebinding) is **not checked** (unverified); private windows turn tools off. **8 Motion:** your message and the answer slide in, the send button becomes Stop, the Helm switch animates, all stopped by Reduce motion. **Follow-ups:** asking again in the Helm card continues the same conversation (the whole history is sent). **Verified live** (`ai_chat.py`, 31 checks, with a fake model): Ctrl+J and its animation, thinking, question/answer/footer order, Markdown, follow-up history (system, user, assistant, user), the assistant opening a tab, saved on disk, Continue in chat, chat page send/Stop/stream/typing survives refresh, new chat, history list, delete, clear all, private window saves nothing; also `private_ai` 9/9. **Verified with the real local model** (qwen via Ollama, real web): asked to search for a stock price, it searched DuckDuckGo, showed the step and sources and summarised (it did not read the page itself). Rust: 5 tests for tool parsing, address safety, HTML to text, search parsing and the stream filter, 5 mock-server tests for the agent loop, 3 for the chat store. Screenshot: `docs/evidence/2026-11-21/chat-page.png`.
- **Ideas not built yet (your "think of more things"):** answer about the page you are on (send the page text as context: the plumbing exists in Rust, the UI does not send it yet); regenerate and copy buttons on answers; rename chats; export a chat; per-site "ask about selection" in the right-click menu; read-aloud; voice input; keyboard hint chips in the Helm; click and type control of pages (needs a careful permission design first). Say which you want next.
- **Assistant page control, full (owner request 21 Nov: "fully implement it").** The assistant can now work in the tab you are looking at: **snapshot** (reads the page: URL, title, scroll position, up to 70 numbered things you can click or fill, and the page text), **click**, **type** (with optional Enter), **select** a menu option, **press** a key (Enter, Tab, Escape, arrows, Page Up/Down), **scroll**, **navigate**, **back / forward / reload**, **list_tabs**, **switch_tab**, **close_tab**, **new_tab** and **wait**. After every action it looks at the page again, and only the newest snapshot is kept in the conversation. Up to 15 steps per question (5 without page control). **How:** the shell runs a small script in the page (`app/shell/src/agent_js.rs`) and the answer comes back through the console as `AgentResult` (the page gets no function to call us; ids are not guessable from earlier ones). **Safety rules, all tested:** (1) it never types into password, card, security-code or one-time-code fields, nor into anything that is not a text box (refused in the page and the model is told); (2) before clicking something whose label sounds like buying, paying, sending, deleting, submitting, subscribing, accepting, installing or signing out, or a button in a form that has a password field, **it asks you** (a card over the page: "The assistant wants to click “Place order” on shop.test. Allow it?", No or Yes, do it); No tells the model not to retry; the question is void when the assistant is stopped or finished; (3) only public http(s) addresses for navigate and new_tab; (4) each step is shown in the answer ("Typing in “Search”", "Clicking “Place order”"); (5) a Settings switch **Control the page** (AI section, on by default, needs "Search and open pages") turns the whole thing off, and then the model is not even told the tools exist; (6) private windows turn it off; (7) page text is marked untrusted in the prompt. **Verified live** (`ai_control.py`, 17 checks, a scripted fake model driving a real page): it typed into the search box, chose a menu option, clicked Search (the page shows the result), never touched the password field, scrolled, asked before "Place order", **No** left it unplaced, **Yes** placed it exactly once, list_tabs and new_tab, and the off switch. 4 more Rust tests for the loop (read, click, look again; no means no; yes lets it through and a password is refused; local address refused). **Verified with the real local model** (qwen 9B via Ollama): after prompt changes it typed into the box, clicked Search and the page then showed "results for star chart". Honest limit: small local models are unreliable at multi-step tasks (before the prompt changes this one read the page and then described results it had not produced); a bigger or cloud model will do better. **Not built / not verified:** clicks and typing are scripted events from the page (`isTrusted` is false), so a few sites that insist on real mouse or keyboard input may ignore them; drag and drop, file upload, iframes and shadow DOM are not reached; no screenshots for vision models; an action in a background tab is not possible (it works on the tab being shown). Screenshot: `docs/evidence/2026-11-21/agent-confirm.png`.
- **Tool fix:** `tools/visual_check.py` served files one request at a time, so the first board sometimes failed to load a module ("Failed to fetch dynamically imported module"). It is threaded now; every board passes again (limit unchanged).
