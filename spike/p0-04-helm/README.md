# P0-04 spike: our UI + a page + the floating Helm (Windows)

Throwaway code. Same `cef` crate and CEF as P0-02 (`cef =156.7.0+156.0.31`, CEF 156.0.31 / Chromium 156.0.8078.100).

One CEF Views window (Chrome style). The web page is a **Chrome style** BrowserView, the window's only child, inset 56 px from the left. Our UI is **Alloy style** BrowserViews loading local HTML (`ui/shell.html` = stand-in rail and sky, `ui/helm.html` = stand-in Helm). These are spike stand-ins, not the design.

```powershell
cd spike\p0-04-helm
.\run.ps1 -AppArgs "--mode=overlay --probe" -Shot p0-04-x.png   # build, run, screenshot, log
.\bench.ps1 -Modes overlay       # one mode per call: none | overlay | window
.\resize.ps1 -Mode window        # minimum size and maximised captures
cargo test                       # address parsing unit test
```

Modes:
- `overlay` (option c, "what the CEF docs suggest"): rail and Helm are `Window::add_overlay_view` overlays (CUSTOM docking) above the page.
- `window` (option b): rail is an overlay; the Helm is its own frameless CEF window, parented to the main window (`WindowDelegate::parent_window`), moved on `on_window_bounds_changed`/`on_layout_changed`, clipped to a pill with `SetWindowRgn`.
- `none`: no Helm, baseline.

Screenshots are taken with `PrintWindow` per window (`shot.ps1`, `shot-all.ps1`), never a screen copy, so nothing else on the desktop is captured. `shot-all.ps1` composites the spike's own windows and applies each window's region.

## Findings (Windows 11, 125% scaling, 144 Hz display, debug build)

1. **One Chrome style BrowserView per window**, and it must be added first. The CEF header (`include/internal/cef_types_runtime.h`) says a Chrome style window hosts at most one Chrome style BrowserView and any number of Alloy style ones. In practice `browser_view_impl.cc` `AddedToWidget()` refuses a Chrome style BrowserView once the window has a theme profile, and adding *any* BrowserView (Alloy too) sets one. So: add the Chrome page first, then Alloy views. A Chrome style BrowserView added as an overlay was also refused.
2. **Our UI must be Alloy style.** Fine for our own chrome (it needs no extensions). Consequence for tabs: only one Chrome style page can be in a window at a time, so switching tabs means swapping that one view. Not tested yet (P1-02).
3. **Windowed browsers can't be transparent** (`cef_types.h`, `background_color`: alpha 0 means white for windowed browsers; transparency only for windowless). Tested: alpha 0 on the Helm's BrowserView and View background still gives an opaque rectangle (`../shots/p0-04-overlay-transparent-try.png`).
4. **Option c (overlay):** works. The page is visible all around the Helm; the Helm floats over it. But the Helm is a **square** rectangle (black corners around the pill) and has **no shadow** (`../shots/p0-04-overlay-win.png`). The overlay is drawn inside the main window's single native window, so it can't be clipped with a window region.
5. **Option b (owned window):** works. Rounded pill via `SetWindowRgn`, so the page shows at the corners, but the edge is **aliased** (regions have no anti-aliasing) and there is **no shadow** (`../shots/p0-04-window.png`). The Helm window follows the main window exactly (moved +120,+40 px → Helm moved 120,40), hides when the main window is minimised and comes back on restore; it has no taskbar button and is not above other apps.
6. **Option a (off-screen rendering for the page):** not built. The CEF header says *windowless rendering always uses Alloy style*, so the page would lose Chrome style, and with it the extension support D-003/D-010 depend on. A narrower variant is promising and not yet tried: render **only our own UI** (Helm, popovers) off-screen with a transparent background and draw it into a per-pixel-alpha layered window. That would give anti-aliased corners and the drawn shadow while the page stays Chrome style. Cost: we forward mouse, keyboard and IME to that UI ourselves.
7. **Performance is the same in every mode** (JS probe: scroll 6 px per frame for 240 frames; input = event `timeStamp` to handler, 10 synthetic clicks + 10 keys through SendInput/SendKeys):

| Mode | Scroll fps | Scroll p95 frame | Frames > 25 ms | Click → handler avg / max | Key → handler avg / max |
|---|---|---|---|---|---|
| none | 144.4 | 7.10 ms | 0 | – | – |
| overlay | 144.4 | 7.10 ms | 0 | 0.48 / 1.1 ms | 0.74 / 4.8 ms |
| window | 144.4 | 7.10 ms | 0 | 0.48 / 0.7 ms | 0.71 / 5.3 ms |

   These numbers only show that neither approach slows the page or the Helm. They don't measure the OS-to-screen latency.
8. Window minimum 800×600 DIP works (asked for 700×500 px, got 1018×797 px including the frame at 125%). Maximised, the Helm stays centred in the page area (centre at 995 px of 1920 px = middle of 70…1920). The page content in `p0-04-*-min/max.png` is a stale white frame: `PrintWindow` doesn't capture freshly resized GPU content reliably, so those two images only show the Helm placement.
9. The Helm covers about 52 px of the page above its bottom 28 px. Content that is fixed to the bottom of a page (cookie banners) can sit under it (EDGE §4). Decide in P2-05.

Linux: **not tested** (no Linux machine yet). On Linux, `SetWindowRgn` doesn't exist. Option b would need an X11 shape or a transparent toplevel, and on Wayland the app can't place the Helm window itself, so the overlay or off-screen variants matter more there.

## P0-04b: the Helm rendered off-screen into a layered window (`--mode=osr`, Windows)

`src/osr.rs`. The Helm is an **Alloy, windowless** CEF browser (`WindowInfo::set_as_windowless`, `background_color` alpha 0 = transparent painting, `windowless_frame_rate` 60, `Settings.windowless_rendering_enabled = 1`). It paints premultiplied BGRA buffers (`RenderHandler::on_paint`) into a `WS_EX_LAYERED` popup **owned by the main window** with `UpdateLayeredWindow(ULW_ALPHA)`. The HTML is 48 DIP larger on each side so the design's shadow (`0 12px 32px rgba(0,0,0,.5)`) is drawn. The page stays the Chrome style BrowserView.

What we forward ourselves (WndProc): mouse move/leave/buttons/wheel (pixels → DIP), keys (`WM_KEYDOWN/UP/CHAR`, system keys), focus (`set_focus`), IME (`WM_IME_COMPOSITION` → `ime_set_composition`/`ime_commit_text`, `WM_IME_ENDCOMPOSITION` → `ime_finish_composing_text`), cursor (`DisplayHandler::on_cursor_change` → `SetCursor`). `WM_NCHITTEST` returns `HTTRANSPARENT` outside the pill shape, so clicks in the shadow reach the page (the main window is on the same thread, which `HTTRANSPARENT` needs). Esc in the Helm hands focus back to the main window and the page.

Results (same machine and probe as above; `bench.ps1 -Modes osr`, `$env:HELM_FPS` sets the off-screen frame rate):

| Check | Result |
|---|---|
| Look | Anti-aliased 26 px pill with the drawn shadow over the live page (`../shots/p0-04b-osr.png`; composited from the exact frame passed to `UpdateLayeredWindow`, dumped with `HELM_DUMP`, because `PrintWindow` can't read layered alpha) |
| Page scroll with the Helm present | 144.4 fps, p95 7.0–7.1 ms, 0 frames > 25 ms (5 runs; one run at 58.7 fps was not repeatable) |
| Click / key → handler in the Helm | 0.36–0.40 ms / 0.53–0.58 ms average |
| Input → next Helm paint (`METRIC osrpaint`, from WndProc to `on_paint`) | 25–48 ms average over 5 runs, max 130–217 ms (first input after load). 120 fps did not help (33–37 ms), so 60 stays |
| Click 20 DIP above the pill (in the shadow) | reaches the page (`pagepointer` logged) |
| Main window moved +120,+40 px | Helm moved 120,40 |
| Main window minimised / restored | Helm hidden / shown (`place()` skips while the owner `IsIconic`) |
| IME (Chinese, Japanese, Korean) | **not tested**: needs a person with an IME installed. Code path exists; candidate window placement (`on_ime_composition_range_changed`) not done |
| Linux | not tested. Needs a different window path (X11 ARGB visual or Wayland subsurface) |

Costs and risks: input-to-paint is slower than a windowed view (CEF copies the frame to the CPU; `shared_texture_enabled` with a D3D11 compositor could cut that, untested). Accessibility of an off-screen browser has to be checked (screen readers see the layered window, not a native Chromium tree; *unverified*). Every popover (downloads, answer card, closed-tab preview) would use the same path.
