# Decisions

Each decision has an ID, a status and its reasons. Statuses:

- **Accepted**: the owner agreed. Build on it; don't reopen it without the owner.
- **Proposed**: an agent's recommendation waiting for the owner.
- **Open**: not decided; a Phase 0 task or the owner settles it.
- **Superseded by D-xxx**: kept for history.

Research for the original decisions was done on 18–19 Nov 2026. Facts marked *unverified* were not confirmed then; check them before relying on them. Sources are at the end.

---

## D-001 Platforms: Windows and Linux only. Accepted

The owner doesn't care about macOS. That removes Apple signing and notarization and the need for Mac hardware.

## D-002 Separate product on its own branch. Accepted

Not part of Odyssey. The code lives in the same GitHub repo, `13LA7E/Odys-Brows`, on the branch **`Constellations`** (the owner created it on 19 Nov 2026). Odyssey stays on `main`, untouched. P0-00 removes Odyssey's files from the Constellations branch and adds this project. Useful Odyssey code is copied in, not linked. Agents push every finished task to `Constellations` and never touch `main`.

## D-003 Engine: bundled Chromium through CEF, "Chrome runtime / Chrome style". Accepted (confirm in Phase 0)

Why:
- The built-in webview on Linux is WebKitGTK, which is weaker than Chromium. Bundling one engine gives the same behaviour on both systems.
- **The owner wants extensions.** That rules out Electron: Electron says it does not support arbitrary Chrome Web Store extensions and that full compatibility is a non-goal. It loads only unpacked extensions, doesn't remember them between runs, and supports only a subset of the APIs.
- A CEF maintainer said extensions will be fully supported with the **Chrome runtime** (not the older Alloy runtime). Since CEF M125 the Chrome bootstrap is the path forward. Switching is `CefSettings.chrome_runtime = true`.

Risks to settle in Phase 0:
- Does Chrome style let us draw our own UI with no Chrome toolbar or tab strip? *Unverified.*
- How complete is extension support in current CEF builds? *Unverified.*
- If CEF can't do both, the next option is a Chromium fork (much heavier). Don't go there without the owner.
- **P0-05 results (Windows, 19 Nov 2026; Linux not tested).** Details and screenshots: `spike/p0-05-extensions/README.md`.

  | Check | Windows | Linux |
  |---|---|---|
  | No Chrome toolbar / tab strip in our window | Yes, by default (Views + Chrome style, toolbar type NONE) | not tested |
  | Install from the Chrome Web Store page | Works ("Add to Chrome" → Chrome's prompt → installed), despite a "Switch to Chrome" banner | not tested |
  | Install unpacked (`--load-extension`) | Works (per launch) | not tested |
  | Background service worker (uBOL, Bitwarden, Dark Reader) | Running | not tested |
  | Content scripts (Dark Reader) | Work | not tested |
  | Network blocking (uBOL, declarativeNetRequest) | Works | not tested |
  | Options page | Works as a tab | not tested |
  | Action popup | No button to open it (no toolbar); the popup page works as a tab | not tested |

  **D-003 re-confirmed on Windows** for what extensions need. Problems written up for the owner: extension-opened tabs appear in a native Chrome window outside our UI; Chrome's registry-registered external extensions are picked up and even downloaded into our profile; the popup needs our own button (P6-02). Linux still to test.
- Found in P0-04: a Chrome style window can host **only one Chrome style BrowserView at a time** (`cef_types_runtime.h`). Tabs therefore need to swap the page view in and out of the window. Swapping without losing the page is *unverified*; test it in P1-02.

## D-004 Own engine: no. Accepted

Writing a browser engine is a decade-scale effort for funded teams. Servo started in 2012 and is still 0.x; Ladybird is pre-alpha with a full-time team. We build everything **above** the engine.

## D-005 Thin engine interface. Accepted

All browser code talks to the engine through our own small trait (`engine-api` crate), never to CEF directly. One implementation now (`engine-cef`). Reasons: keeps CEF details in one place, makes the code testable, and leaves the door open to Servo later (it reached crates.io as v0.1.0 in April 2026). Keep the trait small: only what the browser uses.

## D-006 App language: Rust. Accepted

Keeps Odyssey's Rust code (segmented downloader, Shields/adblock, vault logic) and the owner's existing toolchain. CEF bindings for Rust exist from the Tauri team (`tauri-apps/cef-rs`, listed for Windows and Linux on x64 and ARM64).

## D-007 Shell framework: Tauri + CEF, or plain Rust + cef-rs. Open (settled by P0-08)

- Tauri+CEF: as far as found, not an official stable path yet. One project ships a forked Tauri CEF runtime and notes that the stock Tauri CLI produces a broken bundle.
- Plain Rust + cef-rs: more of our own window and packaging code, fewer moving parts we don't control.
- This also decides installers and the auto-updater (see D-013).

**Proposed (P0-08, 19 Nov 2026): plain Rust + cef-rs.** Needs the owner's OK. Reasons, from the Phase 0 spikes:
- Every spike needed direct control of CEF Views that a Tauri runtime layer hides: the order in which Chrome and Alloy style BrowserViews are added (P0-04), overlay views and box layouts, `ChromeToolbarType` (P0-05), windowless rendering into a layered window we own (P0-04b), Chrome commands and popup callbacks for extension-opened tabs (P0-05). All of this works today with the `cef` crate alone (P0-02 to P0-07 are built on it).
- Tauri's CEF runtime (`tauri-runtime-cef`) appeared with Tauri v3.0.0-alpha; its runtime API is documented as unstable, and a March 2026 issue said CEF webviews lacked back/forward/reload/stop in Tauri's API. Current state not rechecked beyond that (*unverified*). We'd depend on an alpha for the core of the product.
- What Tauri would add (windowing, IPC to a webview UI, bundling) we already have in CEF Views and CEF's own process messages; the bundling part is covered by a Tauri-independent packager (D-013).
- Cost: we write our own window, IPC bridge and packaging glue. The spikes show that is small (a few hundred lines each).

## D-008 UI technology: HTML/CSS/JS for the browser chrome. Accepted

Same approach as Odyssey, and the design boards are already HTML, so tokens and markup carry straight over. Rules: one CSS token file from DESIGN.md §3; no entrance animations on elements that re-render; the UI never gets secrets.

## D-009 The Helm floats over the page. Accepted as the goal; fallback Open (P0-04)

The design has the command bar floating over the page. With native child views this is a z-order problem: a similar Tauri+WebView2 browser had to turn popups into separate borderless windows. Phase 0 tests options (CEF off-screen rendering compositing, an overlay window, or a reserved strip at the bottom as fallback). Any fallback needs the owner's OK because it changes the design.

**P0-04 results on Windows (19 Nov 2026, evidence in `spike/p0-04-helm/README.md` and `spike/shots/p0-04-*.png`). Linux not tested.**

- The page is a Chrome style CEF BrowserView, the window's only normal child. Our UI (rail, Helm, popovers) is Alloy style BrowserViews. CEF allows one Chrome style BrowserView per window, and it must be added before any other BrowserView (checked in `cef_types_runtime.h` and `browser_view_impl.cc`).
- Windowed CEF browsers can't be transparent (`cef_types.h`), so a Helm drawn in a normal BrowserView is an opaque rectangle.
- (c) CEF Views overlay (`Window::add_overlay_view`): works, the Helm floats over the page, but it is square (black corners) with no shadow.
- (b) Separate frameless window parented to the main window: works, follows moves, hides on minimise. `SetWindowRgn` gives a rounded pill, but with aliased edges and no shadow. Windows only as built.
- (a) Off-screen rendering for the page: not built, because CEF says windowless is always Alloy style, which loses Chrome style extensions (D-003, D-010).
- Scrolling and input latency are the same with no Helm, (b) and (c): 144 fps, p95 frame 7.1 ms; click/key to handler under 1 ms on average.

**Proposed (needs the owner):** build the Helm and popovers as **Alloy style views drawn off-screen with a transparent background into a per-pixel-alpha window** above the page (Windows: a layered window owned by the main window; Linux: a transparent toplevel or subsurface, to be tested). Only our own UI is off-screen; the page stays Chrome style. This is the only option found that can draw the design exactly (anti-aliased 26 px pill and the drawn shadow). It needs a follow-up spike (P0-04b) to prove input, IME and focus. **Interim fallback** if that fails: (c) overlay views with square-cornered Helm and popovers. That changes the design, so it needs the owner's OK (Q-06).

**P0-04b result (19 Nov 2026, Windows): the proposed approach works.** The Helm as an Alloy windowless browser painted into a layered window owned by the main window draws exactly as designed: an anti-aliased pill with the shadow, over the live page (`spike/shots/p0-04b-osr.png`). Page scrolling is unchanged (144 fps). Clicks in the shadow reach the page (`WM_NCHITTEST` → `HTTRANSPARENT`). It follows moves and hides on minimise. Cost: input-to-paint 25–48 ms on average (frames are copied through the CPU). IME is coded but untested, accessibility of off-screen UI is *unverified*, and Linux needs its own window path. Details: `spike/p0-04-helm/README.md`. **Still Proposed: owner OK needed.**

## D-010 Extensions: supported, Manifest V3 only. Accepted

- Google removed every Manifest V2 extension, uBlock Origin included, from the Chrome Web Store on 31 Aug 2026, and Chromium itself has removed MV2 support. A bundled Chromium is MV3-only.
- Brave keeps uBlock Origin alive by hard-coding an exception for four extensions using code that Chromium dropped. We can't do that with CEF, so we don't try.
- Our built-in Shields (D-011) doesn't depend on extensions, so ad blocking stays strong.
- Installing from the Chrome Web Store in a non-Google browser: other Chromium browsers do it, but Google's terms for this were **not checked**. Settle before shipping (Q-04).

## D-011 Built-in ad and tracker blocking ("Shields"). Accepted

Port from Odyssey (`adblock` crate, Brave's engine). Hook it into CEF's request handling instead of WebView2's. Also helps memory (fewer ad frames).

## D-012 Efficiency goal: use clearly less memory than Chrome and Edge, measured. Accepted

- Pages cost the same in every Chromium browser, so the wins come from: putting background tabs to sleep early (Chrome waits until memory is low), built-in blocking, a light browser UI, and showing per-extension cost.
- Published comparisons disagree and most are vendor tests (Brave's own test: about 1.2 GB for Brave against 1.75 GB for Chrome; another puts Firefox lowest at 50 tabs). So we measure ourselves (P0-07, P5-04) before claiming anything.
- Don't use Chromium's single-process mode; it's for debugging.

## D-013 Packaging. Accepted in shape, details depend on D-007

- Windows: an installer (NSIS `-setup.exe` and/or MSI) with a signed auto-updater.
- Linux: AppImage and .deb first; rpm/Flatpak later.
- Built in CI on a Windows runner and an Ubuntu runner (Linux bundles need Linux toolchains).
- Code signing on Windows (Q-02). Without it, SmartScreen warns, and even signed apps build reputation over weeks.
- **Proposed with D-007 (P0-08):** package with `cargo-packager` (CrabNebula, Tauri-independent; 0.11.8 on crates.io; formats per its README: NSIS `.exe` and WiX `.msi` on Windows, `.deb`, AppImage and pacman on Linux; no rpm) and update with `cargo-packager-updater` (0.2.3). Still to check in P9-01: that it can ship CEF's runtime files (DLLs/.so, .pak, locales) and that the updater verifies signatures (*unverified*: its README doesn't say). Fallback: a hand-written NSIS script and our own signed-manifest updater.

## D-014 AI: off by default, provider setting, browser works without it. Accepted

Providers: none (default); a local server (Ollama, LM Studio, any OpenAI-compatible URL); an API key for a cloud service; the owner's Odysseus server. Keys go in the OS secure store (Windows Credential Manager, Linux Secret Service), never in plain files, the UI or web pages. The Helm answer card names which provider answered.

## D-015 DRM (Widevine): wanted, not promised. Open (P0-06)

- Google's Widevine page lists CEF and Electron as supported.
- CEF disables components by default; one third-party guide enables Widevine with a `--component-updater` switch. That's a hint, not an official recipe.
- Linux was not confirmed at all.
- Test streaming playback on both systems in P0-06 and report honestly. Licensing or signing needs for a CEF app are *unverified*.

**P0-06 results (19 Nov 2026). Details: `spike/p0-06-drm/README.md`.**

| | Windows 11 | Linux |
|---|---|---|
| Widevine CDM | **Works.** Listed in `chrome://components`; version 4.10.3112.0 downloaded into the profile after "Check for update" (no switch needed). Pages see `com.widevine.alpha` (L3: `SW_SECURE_CRYPTO`/`SW_SECURE_DECODE`, `cenc` + `cbcs`) | not tried |
| Widevine playback | **Works with VP9**: Shaka demo assets Angel One (VP9 768x576) and Sintel (VP9 1280x546) played with Shaka's public license proxy | not tried |
| H.264 / AAC | **Fail: not supported** by the stock CEF build (no proprietary codecs), so H.264-only streams can't play | not tried |
| VMP signing / HD robustness for commercial services | not tried (*unverified*) | not tried |

## D-016 Agent-neutral repo. Accepted

All instructions live in `AGENTS.md`, read natively by Kimi Code, OpenCode (the harness NVIDIA documents for Nemotron), Google Antigravity (Gemini) and Codex. `CLAUDE.md` contains only `@AGENTS.md`. Claude Code reads AGENTS.md on its own only when there is no CLAUDE.md, and only from v2.1.277 on, so the import keeps it working everywhere. There is no `GEMINI.md`: Antigravity gives it priority over AGENTS.md, so it would split the rules. State lives in `docs/STATUS.md`, not in any tool's memory or chat history.

## D-017 Each tab is its own owned Chrome style window. Proposed (P1-02)

Found in P1-02 (spike `spike/p0-04-helm --swap`): after the first page view is removed from the main window, CEF refuses a new Chrome style BrowserView there ("Cannot add multiple Chrome style BrowserViews"), because the window keeps a theme profile once any BrowserView was added (`browser_view_impl.cc` AddedToWidget). Moving a view whose browser was created in a hidden helper window was tried too; the app exited without a clear error, so that path was dropped.

So each page view (tab) lives in its **own frameless Chrome style CEF window, owned by the main window** (`WindowDelegate::parent_window`) and placed over the page area in screen DIP. Switching tabs shows one page window and hides the rest. The main window holds only our Alloy style UI. Implemented in `app/engine-cef` and checked by `constellations --engine-selftest` (navigate, back, forward, reload, second view, visibility swap, close).

Consequences to handle:
- Z-order: page windows and the off-screen Helm window (D-009 proposal) are all owned by the main window; the Helm must be kept above the active page window (re-raise it when a page window activates).
- Activation: clicking a page activates its window, not the main window. Keyboard shortcuts must work in both (P2-06).
- Linux/Wayland: apps can't place top-level windows themselves; owned page windows may not work there. Test in P0-03/P1 on Linux; the fallback there may be one CEF window per tab without our overlay UI (owner decision). *Unverified.*
- Memory: one more native window per tab (small); measured again in P5-04.

---

## Open questions for the owner

| ID | Question | Needed by |
|---|---|---|
| Q-01 | Product name (and check it's free to use). The design uses "odyssey". | P9 |
| Q-02 | Windows code signing: Azure Artifact Signing is about $9.99/month, but individual sign-up was limited to the USA and Canada when checked; an OV certificate is about $150–300/year. Which, and under what name? Re-check before buying. | P9 |
| Q-03 | Licences of the ad-block filter lists: can we bundle them, or must we download them on first run? | P5 |
| Q-04 | Installing extensions from the Chrome Web Store: allowed by Google's terms? If not, what extension source? | P6 |
| Q-05 | Licence for this project's own code (open source or closed)? | P1 |
| Q-06 | Fallback if the Helm can't float over the page (D-009). | after P0-04 |
| Q-07 | Import bookmarks/passwords from other browsers? | P8 |
| Q-08 | H.264/AAC: the stock CEF build has none (P0-06). Build CEF with proprietary codecs (patent licensing to check), or accept VP9/AV1-only video? | before P9 |

---

## Sources (checked 18–19 Nov 2026)

- CEF Chrome runtime and extensions: https://github.com/chromiumembedded/cef/issues/1947, https://groups.google.com/g/cef-announce/c/s1WaovAopFo
- CEF Rust bindings: https://github.com/tauri-apps/cef-rs
- Electron extension support: https://electronjs.org/docs/latest/api/extensions
- MV2 removal: https://www.neowin.net/news/google-chrome-is-killing-all-ublock-origin-bypasses-microsoft-edge-opera-to-follow/, https://gigazine.net/gsc_news/en/20260901-manifest-v2-extensions
- Widevine support list: https://developers.google.com/widevine/drm/overview
- castLabs Electron (for comparison): https://github.com/castlabs/electron-releases
- Servo: https://servo.org/about/
- Ladybird: https://ladybird.org
- Tauri distribution formats: https://tauri.app/distribute/
- Chromium memory model: https://www.chromium.org/developers/memory-usage-backgrounder/
- Kimi Code reads AGENTS.md: https://www.kimi.com/help/kimi-code/cli-customization
- Nemotron with OpenCode and other agents: https://docs.nvidia.com/nemotron/0.1.0/usage-cookbook/Nemotron-3-Super/OpenScaffoldingResources/README.html
- Claude Code and AGENTS.md: https://devops.com/claude-code-adds-agents-md-fallback-cutting-instruction-file-sprawl/
- Antigravity rules (AGENTS.md, GEMINI.md): https://antigravity.google/docs/rules
