# Status

**Every agent updates this file at the end of every session**, even when stopping half way. The next agent starts here. Keep the top section short and current; the history goes in the session log.

## Now

| | |
|---|---|
| Phase | 0: Spike |
| Current task | **P0-06 DRM test** (Windows) |
| State | **P0-05 Windows part done** (D-003 re-confirmed on Windows, problems written up). **P0-04 and P0-04b Windows parts done, awaiting Owner OK** (D-009 Proposed: off-screen Helm in a layered window works). P0-02 done. **\* LINUX PENDING \*** P0-01b, P0-03 and the Linux half of P0-04 wait for the owner's Ubuntu dual boot; the Phase 0 gate stays open. |
| Next step | P0-06: enable Widevine through CEF's component updater in a spike (start from `spike/p0-05-extensions`), play a public Widevine test stream (Shaka Player demo or Bitmovin DRM demo), record exactly what happens in D-015. |
| Branch | `Constellations` on `13LA7E/Odys-Brows`. Never `main`. |
| Last pushed commit | `fd848b3` (P0-04b); P0-05 commit follows (see the session log) |
| Last agent | Claude Code (VS Code) / Claude Opus 5.5 |

## Blockers

- **\* LINUX PENDING \*** P0-01b needs the deferred Ubuntu toolchain setup and version record. P0-03 (P0-02 on Linux) also waits for it. Windows work can continue, but the full P0-01 task and Phase 0 gate remain open.

## Awaiting Owner OK

- **P0-04 / D-009:** the Helm-over-page approach. Evidence: `spike/p0-04-helm/README.md`, `spike/shots/p0-04-overlay-win.png` (overlay: square corners), `spike/shots/p0-04-window.png` (owned window: rounded but aliased, no shadow). Proposal: off-screen Helm in a layered window (P0-04b); interim fallback square corners (Q-06). Left `[~]` in PHASES.
- **P0-04b:** the off-screen Helm works on Windows (`spike/shots/p0-04b-osr.png`, numbers in `spike/p0-04-helm/README.md`). Needs: the owner's OK on the look and the 25–48 ms input-to-paint, and an IME test by someone with a Chinese/Japanese/Korean IME (`run.ps1 -AppArgs "--mode=osr"`, click the Helm, type with the IME). Left `[~]`.

## Questions for the owner

- P0-05: installing from the Chrome Web Store works in CEF. Is that acceptable under Google's terms (Q-04)? Chosen meanwhile: keep testing with it, decide before P6.

- D-009 (Proposed in P0-04): may we build the Helm and popovers as off-screen Alloy views in a layered window? If P0-04b fails, is a square-cornered Helm (overlay views) acceptable for now (Q-06)? Chosen meanwhile: continue with P0-04b, the option that keeps the design exact.

- Linux test setup: the owner will use an Ubuntu dual boot later; Linux setup and testing are deferred for P0-01b and P0-03.
- Open decisions Q-01 to Q-07 are listed in `docs/DECISIONS.md`. None blocks Phase 0.

## Known gaps (edge cases not handled yet)

- **P0-01b — \* LINUX PENDING \*** Linux toolchain installation, versions and CEF build packages are not verified.
- **P0-02:** single-instance hand-off not built (a second copy on the same profile exits with a message; the real app must forward its URL, P1 or later). Running from a path with spaces or a non-ASCII user name not tested. Sandbox off in the spike (decide with P0-08).
- **P0-05:** extension-opened tabs open a native Chrome window (must be caught; callback unverified); Chrome's registry-registered external extensions are picked up and downloaded into our profile (must be turned off; method unverified); the action popup has no button (P6-02); unpacked extensions don't persist via `--load-extension`; Linux not tested.
- **P0-04b:** IME untested; IME candidate window placement not done; screen-reader access to the off-screen Helm unverified; input-to-paint 25–48 ms (shared-texture path untested); Linux window path not designed.
- **P0-04:** Linux not tested. Option (a) (page off-screen) not built because CEF windowless is always Alloy style (no Chrome style extensions). Tab switching with a single Chrome style BrowserView per window is unverified (test in P1-02). Page content fixed to the bottom can sit under the Helm (decide in P2-05). The min/max resize captures show only Helm placement (PrintWindow returned stale page frames).
- PowerShell's current execution policy blocks `npm.ps1`; use the verified `npm.cmd` shim. No execution-policy setting was changed.

## Needs testing on

- **Linux:** **\* LINUX PENDING \*** P0-01b toolchain setup and versions, P0-03 (the P0-02 spike on Linux), the Linux column of the P0-05 table, the Linux half of P0-04 (no SetWindowRgn on Linux; Wayland can't position windows), plus later Linux runtime and UI verification.

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

### 2026-11-19: Claude Code (VS Code) / Claude Opus 5.5: P0-05 (Windows)
- Did: `spike/p0-05-extensions` (P0-02 window + `--toolbar=` + `--then=`), `run.ps1`, `shot-main.ps1`. Fetched uBlock Origin Lite 2026.1116.1650, Bitwarden 2026.10.2 and Dark Reader 4.9.135 as CRX3 from Google's update service and unpacked them outside the repo (path-checked). Tested Chrome UI hiding, unpacked load, Web Store install, service workers, content scripts, DNR blocking, options and popup pages. Results table under D-003, README with problems, three new edge-case kinds in EDGE_CASES §10.
- Verified: build OK (into the shared `../p0-04-helm/target`), clippy clean. `chrome://extensions` lists all three enabled (`spike/shots/p0-05-extensions-page.png`). `--toolbar=normal`/`location` show no toolbar in our window. `chrome://serviceworker-internals` shows the three workers, uBOL RUNNING. Dark Reader darkened Wikipedia. With uBOL, `pagead2.googlesyndication.com/pagead/js/adsbygoogle.js` ended on `chrome-extension://<uBOL>/strictblock.html#...`; without extensions: `ERR_ABORTED`, no extension page. Popup/options pages as tabs: `load_end status=200`. Web Store: rejected Google's consent ("Reject all"), clicked "Add to Chrome", Chrome's prompt appeared, clicked "Add extension"; `Secure Preferences` then lists Dark Reader with `location=1 from_webstore=True`, and `Default/Extensions/<id>/4.9.135_0` exists.
- Privacy: two screenshots were redacted before commit (a card naming an extension a local program registered for Chrome; the profile path containing the Windows user name). Extension files are not in the repo.
- Edge cases (§10): MV2 install not tried (D-010 stands). Broad permissions shown at install: yes, Chrome's prompt lists them (checked by hand). Popup above pages: no popup button exists yet (P6-02). Crash/hang, update failure, removal clearing data: not tested (P6). New cases added to EDGE_CASES §10. §1: slow network (store consent page) and first run (fresh profile) checked by hand. Linux: not tested.
- Not verified / unsure: Google's terms for Web Store installs (Q-04); how to stop external-extension sources; which callback catches extension-opened tabs; persisting unpacked extensions via developer mode.
- Decisions proposed: none new (D-003 re-confirmed on Windows; table added).
- Next: P0-06 (Widevine) on Windows.
- Commit: see git log "P0-05: Chrome UI hidden and MV3 extensions on Windows"  Pushed: yes

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
