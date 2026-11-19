//! P0-04 spike: our HTML UI + a real page + the floating Helm, on Windows.
//! Modes (`--mode=`): `overlay` = CEF Views overlay views (Helm is an overlay above the page
//! overlay), `window` = Helm in its own frameless CEF window owned by the main window,
//! `none` = no Helm (baseline). `--probe` measures page scroll frame pacing (JS rAF);
//! the Helm always logs input latency (event timeStamp to handler).

use cef::*;
use std::cell::RefCell;

const RAIL: i32 = 56;
const HELM_W: i32 = 560;
const HELM_H: i32 = 52;
const HELM_BOTTOM: i32 = 28;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Mode {
    Overlay,
    Window,
    None,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Role {
    Shell,
    Page,
    Helm,
}

#[derive(Default)]
struct State {
    mode: Option<Mode>,
    window: Option<Window>,
    page_ov: Option<OverlayController>,
    helm_ov: Option<OverlayController>,
    helm_window: Option<Window>,
    page: Option<Browser>,
    helm: Option<Browser>,
    open: i32,
    t0: Option<std::time::Instant>,
}

thread_local! { static ST: RefCell<State> = RefCell::new(State::default()); }

fn mode() -> Mode {
    ST.with(|s| s.borrow().mode.unwrap_or(Mode::Overlay))
}

/// file:// URL for a spike UI file; spaces and non-ASCII are percent-encoded.
fn ui_url(name: &str) -> String {
    let dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_default();
    // The exe is in target/<profile>/; ui/ is two levels up.
    let p = dir.join("..").join("..").join("ui").join(name);
    let p = std::fs::canonicalize(&p).unwrap_or(p);
    let s = p.to_string_lossy().replace('\\', "/");
    let s = s.trim_start_matches("//?/");
    let mut out = String::from("file:///");
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"/:-_.~".contains(&b) {
            out.push(b as char)
        } else {
            out.push_str(&format!("%{b:02X}"))
        }
    }
    out
}

/// Place the page and the Helm for a window content size (DIP).
fn layout(w: i32, h: i32) {
    ST.with(|s| {
        let s = s.borrow();
        // page_ov holds the rail overlay (left strip).
        if let Some(ov) = &s.page_ov {
            ov.set_bounds(Some(&Rect {
                x: 0,
                y: 0,
                width: RAIL,
                height: h,
            }));
        }
        let hx = RAIL + (w - RAIL - HELM_W) / 2;
        let hy = h - HELM_BOTTOM - HELM_H;
        if let Some(ov) = &s.helm_ov {
            ov.set_bounds(Some(&Rect {
                x: hx,
                y: hy,
                width: HELM_W,
                height: HELM_H,
            }));
        }
        if let (Some(win), Some(hw)) = (&s.window, &s.helm_window) {
            let c = win.client_area_bounds_in_screen();
            hw.set_bounds(Some(&Rect {
                x: c.x + hx,
                y: c.y + hy,
                width: HELM_W,
                height: HELM_H,
            }));
            round_corners(hw);
        }
    });
}

#[cfg(windows)]
mod win32 {
    #[link(name = "user32")]
    unsafe extern "system" {
        pub fn SetWindowRgn(h: isize, rgn: isize, redraw: i32) -> i32;
        pub fn GetDpiForWindow(h: isize) -> u32;
    }
    #[link(name = "gdi32")]
    unsafe extern "system" {
        pub fn CreateRoundRectRgn(x1: i32, y1: i32, x2: i32, y2: i32, w: i32, h: i32) -> isize;
    }
}

/// Clip the Helm's own top-level window to a pill (windowed CEF browsers can't be transparent).
fn round_corners(hw: &Window) {
    #[cfg(windows)]
    unsafe {
        let h = hw.window_handle().0 as isize;
        if h == 0 {
            return;
        }
        let dpi = win32::GetDpiForWindow(h).max(96) as f32 / 96.0;
        let w = (HELM_W as f32 * dpi).round() as i32;
        let hh = (HELM_H as f32 * dpi).round() as i32;
        let rgn = win32::CreateRoundRectRgn(0, 0, w + 1, hh + 1, hh, hh);
        win32::SetWindowRgn(h, rgn, 1); // the system owns rgn after this call
    }
}

wrap_window_delegate! {
    struct MainWindowDelegate {
        shell: RefCell<Option<BrowserView>>,
        page: RefCell<Option<BrowserView>>,
        helm: RefCell<Option<BrowserView>>,
    }

    impl ViewDelegate {
        fn preferred_size(&self, _view: Option<&mut View>) -> Size {
            Size { width: 1280, height: 800 }
        }
        fn minimum_size(&self, _view: Option<&mut View>) -> Size {
            Size { width: 800, height: 600 }
        }
        fn on_layout_changed(&self, _view: Option<&mut View>, b: Option<&Rect>) {
            if let Some(b) = b {
                layout(b.width, b.height);
            }
        }
    }

    impl PanelDelegate {}

    impl WindowDelegate {
        fn on_window_created(&self, window: Option<&mut Window>) {
            let Some(window) = window else { return };
            ST.with(|s| s.borrow_mut().window = Some(window.clone()));
            // The Chrome style page is the window's only child, inset by the rail width. Our own
            // UI (rail, Helm) is Alloy style overlays on top.
            // Order matters: CEF refuses a Chrome style BrowserView once the window has any
            // theme profile, and adding any BrowserView (Alloy too) sets one
            // (browser_view_impl.cc AddedToWidget). So add the Chrome style page first.
            window.set_background_color(0xFF000000);
            let layout_box = window.set_to_box_layout(Some(&BoxLayoutSettings {
                horizontal: 1,
                inside_border_insets: Insets { top: 0, left: RAIL, bottom: 0, right: 0 },
                ..Default::default()
            }));
            if let Some(bv) = self.page.borrow().as_ref() {
                let mut v = View::from(bv);
                window.add_child_view(Some(&mut v));
                if let Some(l) = &layout_box {
                    l.set_flex_for_view(Some(&mut v), 1);
                }
            }
            if let Some(bv) = self.shell.borrow().as_ref() {
                let ov = window.add_overlay_view(Some(&mut View::from(bv)), DockingMode::CUSTOM, 0);
                ST.with(|s| s.borrow_mut().page_ov = ov);
            }
            if let Some(bv) = self.helm.borrow().as_ref() {
                let mut v = View::from(bv);
                if std::env::args().any(|a| a == "--helm-transparent") {
                    v.set_background_color(0x00000000);
                }
                let ov = window.add_overlay_view(Some(&mut v), DockingMode::CUSTOM, 1);
                ST.with(|s| s.borrow_mut().helm_ov = ov);
            }
            ST.with(|s| {
                let s = s.borrow();
                for ov in [&s.page_ov, &s.helm_ov].into_iter().flatten() {
                    ov.set_visible(1);
                }
            });
            window.show();
            let c = window.client_area_bounds_in_screen();
            layout(c.width, c.height);
        }

        fn on_window_bounds_changed(&self, window: Option<&mut Window>, _b: Option<&Rect>) {
            if let Some(w) = window {
                let c = w.client_area_bounds_in_screen();
                layout(c.width, c.height);
            }
        }

        fn on_window_destroyed(&self, _window: Option<&mut Window>) {
            *self.shell.borrow_mut() = None;
            *self.page.borrow_mut() = None;
            *self.helm.borrow_mut() = None;
            ST.with(|s| {
                let mut s = s.borrow_mut();
                s.page_ov = None;
                s.helm_ov = None;
                s.window = None;
                if let Some(hw) = s.helm_window.take() {
                    hw.close();
                }
            });
        }

        fn can_close(&self, _window: Option<&mut Window>) -> i32 {
            // Ask every browser in the window to close; the window closes when they agree.
            let mut all = 1;
            for bv in [&self.shell, &self.page, &self.helm] {
                if let Some(b) = bv.borrow().as_ref().and_then(|v| v.browser())
                    && let Some(h) = b.host() {
                        all &= h.try_close_browser();
                    }
            }
            all
        }

        fn window_runtime_style(&self) -> RuntimeStyle {
            RuntimeStyle::CHROME
        }
    }
}

wrap_window_delegate! {
    struct HelmWindowDelegate {
        helm: RefCell<Option<BrowserView>>,
    }

    impl ViewDelegate {
        fn preferred_size(&self, _view: Option<&mut View>) -> Size {
            Size { width: HELM_W, height: HELM_H }
        }
    }

    impl PanelDelegate {}

    impl WindowDelegate {
        fn on_window_created(&self, window: Option<&mut Window>) {
            let Some(window) = window else { return };
            if let Some(bv) = self.helm.borrow().as_ref() {
                window.add_child_view(Some(&mut View::from(bv)));
            }
            ST.with(|s| s.borrow_mut().helm_window = Some(window.clone()));
            window.show();
            if let Some(main) = ST.with(|s| s.borrow().window.clone()) {
                let c = main.client_area_bounds_in_screen();
                layout(c.width, c.height);
            }
        }
        fn on_window_destroyed(&self, _window: Option<&mut Window>) {
            *self.helm.borrow_mut() = None;
        }
        fn is_frameless(&self, _window: Option<&mut Window>) -> i32 {
            1
        }
        fn can_resize(&self, _window: Option<&mut Window>) -> i32 {
            0
        }
        fn can_maximize(&self, _window: Option<&mut Window>) -> i32 {
            0
        }
        fn can_minimize(&self, _window: Option<&mut Window>) -> i32 {
            0
        }
        // Parented to the main window: stays above it, no taskbar button, not above other apps.
        fn parent_window(&self, _window: Option<&mut Window>, is_menu: Option<&mut i32>, can_activate_menu: Option<&mut i32>) -> Option<Window> {
            if let Some(m) = is_menu {
                *m = 0;
            }
            if let Some(c) = can_activate_menu {
                *c = 1;
            }
            ST.with(|s| s.borrow().window.clone())
        }
        fn window_runtime_style(&self) -> RuntimeStyle {
            RuntimeStyle::ALLOY
        }
    }
}

wrap_browser_view_delegate! {
    struct BvDelegate { style: RuntimeStyle, width: i32 }
    impl ViewDelegate {
        fn preferred_size(&self, _view: Option<&mut View>) -> Size {
            Size { width: self.width, height: 0 }
        }
    }
    impl BrowserViewDelegate {
        fn browser_runtime_style(&self) -> RuntimeStyle {
            self.style
        }
    }
}

wrap_life_span_handler! {
    struct Life { role: Role }
    impl LifeSpanHandler {
        fn on_after_created(&self, browser: Option<&mut Browser>) {
            let Some(b) = browser else { return };
            println!("created {:?} alloy={:?}", self.role, b.host().map(|h| h.runtime_style() == RuntimeStyle::ALLOY));
            ST.with(|s| {
                let mut s = s.borrow_mut();
                s.open += 1;
                match self.role {
                    Role::Page => s.page = Some(b.clone()),
                    Role::Helm => s.helm = Some(b.clone()),
                    Role::Shell => {}
                }
            });
        }
        fn on_before_close(&self, _browser: Option<&mut Browser>) {
            let left = ST.with(|s| {
                let mut s = s.borrow_mut();
                s.open -= 1;
                match self.role {
                    Role::Page => s.page = None,
                    Role::Helm => s.helm = None,
                    Role::Shell => {}
                }
                s.open
            });
            if left <= 0 {
                quit_message_loop();
            }
        }
    }
}

fn run_js(b: &Browser, js: &str) {
    if let Some(f) = b.main_frame() {
        f.execute_java_script(Some(&CefString::from(js)), None, 0);
    }
}

/// Spike-level URL vs search; typed javascript: URLs are never run (EDGE §3).
fn target_for(v: &str) -> Option<String> {
    let v = v.trim();
    if v.is_empty() || v.to_ascii_lowercase().starts_with("javascript:") {
        return None;
    }
    Some(if v.contains("://") {
        v.to_string()
    } else if v.contains('.') && !v.contains(' ') {
        format!("https://{v}")
    } else {
        format!("https://duckduckgo.com/?q={}", v.replace(' ', "+"))
    })
}

wrap_display_handler! {
    struct Display { role: Role }
    impl DisplayHandler {
        fn on_address_change(&self, _b: Option<&mut Browser>, frame: Option<&mut Frame>, url: Option<&CefString>) {
            let Some(frame) = frame else { return };
            if frame.is_main() == 0 {
                return;
            }
            let url = url.map(CefString::to_string).unwrap_or_default();
            match self.role {
                Role::Page => {
                    if let Some(h) = ST.with(|s| s.borrow().helm.clone()) {
                        let q = url.replace('\\', "\\\\").replace('\'', "\\'");
                        run_js(&h, &format!("setAddress('{q}')"));
                    }
                }
                Role::Helm => {
                    let Some((_, hash)) = url.split_once('#') else { return };
                    let Some(page) = ST.with(|s| s.borrow().page.clone()) else { return };
                    if let Some(v) = hash.strip_prefix("go=") {
                        if let Some(t) = target_for(&percent_decode(v))
                            && let Some(f) = page.main_frame() {
                                f.load_url(Some(&CefString::from(t.as_str())));
                            }
                    } else if hash.starts_with("back=") {
                        page.go_back();
                    } else if hash.starts_with("forward=") {
                        page.go_forward();
                    } else if hash.starts_with("reload=") {
                        page.reload();
                    }
                }
                Role::Shell => {}
            }
        }
        fn on_console_message(&self, _b: Option<&mut Browser>, _l: LogSeverity, msg: Option<&CefString>, _s: Option<&CefString>, _line: i32) -> i32 {
            let m = msg.map(CefString::to_string).unwrap_or_default();
            if m.starts_with("METRIC") {
                let t = ST.with(|s| s.borrow().t0.map(|t| t.elapsed().as_millis()).unwrap_or(0));
                println!("{t} {:?} {:?} {m}", mode(), self.role);
            }
            0
        }
    }
}

/// Scroll the page 6 px per frame for 240 frames and report frame pacing.
const SCROLL_PROBE: &str = r#"
setTimeout(() => { const iv = []; let last = 0, n = 0;
function f(t) { iv.push(t - last); last = t; scrollBy(0, 6);
  if (++n < 240) requestAnimationFrame(f); else { iv.shift(); iv.sort((a, b) => a - b);
  const avg = iv.reduce((a, b) => a + b, 0) / iv.length, p95 = iv[Math.floor(iv.length * .95)], long = iv.filter(x => x > 25).length;
  console.log('METRIC scroll frames=' + iv.length + ' avg_ms=' + avg.toFixed(2) + ' fps=' + (1000 / avg).toFixed(1) + ' p95_ms=' + p95.toFixed(2) + ' over25ms=' + long); } }
requestAnimationFrame(t => { last = t; requestAnimationFrame(f); }); }, 2500);
"#;

wrap_load_handler! {
    struct Load { role: Role }
    impl LoadHandler {
        fn on_load_end(&self, browser: Option<&mut Browser>, frame: Option<&mut Frame>, status: i32) {
            if self.role != Role::Page {
                return;
            }
            let (Some(b), Some(f)) = (browser, frame) else { return };
            if f.is_main() == 0 {
                return;
            }
            println!("page load_end status={status}");
            if std::env::args().any(|a| a == "--probe") {
                run_js(b, SCROLL_PROBE);
            }
        }
        fn on_load_error(&self, _b: Option<&mut Browser>, _f: Option<&mut Frame>, code: Errorcode, text: Option<&CefString>, url: Option<&CefString>) {
            eprintln!(
                "load error {:?} {:?} {} {}",
                self.role,
                sys::cef_errorcode_t::from(code),
                text.map(CefString::to_string).unwrap_or_default(),
                url.map(CefString::to_string).unwrap_or_default()
            );
        }
    }
}

wrap_client! {
    struct SpikeClient { role: Role }
    impl Client {
        fn life_span_handler(&self) -> Option<LifeSpanHandler> {
            Some(Life::new(self.role))
        }
        fn display_handler(&self) -> Option<DisplayHandler> {
            Some(Display::new(self.role))
        }
        fn load_handler(&self) -> Option<LoadHandler> {
            Some(Load::new(self.role))
        }
    }
}

fn make_view(role: Role, url: &str, bg: u32) -> Option<BrowserView> {
    let mut client = SpikeClient::new(role);
    // A Chrome style window hosts at most one Chrome style BrowserView (cef_types_runtime.h),
    // so our own UI (shell, Helm) is Alloy style and only the web page is Chrome style.
    let style = if role == Role::Page {
        RuntimeStyle::CHROME
    } else {
        RuntimeStyle::ALLOY
    };
    let width = match role {
        Role::Shell => RAIL,
        Role::Helm => HELM_W,
        Role::Page => 0,
    };
    let mut d = BvDelegate::new(style, width);
    let settings = BrowserSettings {
        background_color: bg,
        ..Default::default()
    };
    browser_view_create(
        Some(&mut client),
        Some(&CefString::from(url)),
        Some(&settings),
        None,
        None,
        Some(&mut d),
    )
}

wrap_browser_process_handler! {
    struct Bph {}
    impl BrowserProcessHandler {
        fn on_context_initialized(&self) {
            let cl = command_line_get_global().expect("command line");
            let url = CefString::from(&cl.switch_value(Some(&CefString::from("url")))).to_string();
            let url = if url.is_empty() {
                "https://en.wikipedia.org/wiki/Celestial_navigation".to_string()
            } else {
                url
            };
            let m = mode();
            let shell = make_view(Role::Shell, &ui_url("shell.html"), 0xFF000000);
            let page = make_view(Role::Page, &url, 0xFFFFFFFF);
            // Windowed browsers ignore alpha 0 (cef_types.h background_color), so opaque.
            let helm_bg = if cl.has_switch(Some(&CefString::from("helm-transparent"))) != 0 { 0x00000000 } else { 0xFF000000 };
            let helm = if m == Mode::Overlay {
                make_view(Role::Helm, &ui_url("helm.html"), helm_bg)
            } else {
                None
            };
            let mut d = MainWindowDelegate::new(RefCell::new(shell), RefCell::new(page), RefCell::new(helm));
            window_create_top_level(Some(&mut d));
            if m == Mode::Window {
                let helm = make_view(Role::Helm, &ui_url("helm.html"), helm_bg);
                let mut hd = HelmWindowDelegate::new(RefCell::new(helm));
                window_create_top_level(Some(&mut hd));
            }
        }
    }
}

wrap_app! {
    struct SpikeApp;
    impl App {
        fn browser_process_handler(&self) -> Option<BrowserProcessHandler> {
            Some(Bph::new())
        }
    }
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%'
            && i + 2 < b.len() + 1
            && let Some(v) = s
                .get(i + 1..i + 3)
                .and_then(|h| u8::from_str_radix(h, 16).ok())
        {
            out.push(v);
            i += 3;
            continue;
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn main() {
    let _ = api_hash(sys::CEF_API_VERSION_LAST, 0);
    let args = args::Args::new();
    let Some(cmd_line) = args.as_cmd_line() else {
        std::process::exit(1)
    };
    let is_browser = cmd_line.has_switch(Some(&CefString::from("type"))) != 1;
    let code = execute_process(Some(args.as_main_args()), None, std::ptr::null_mut());
    if !is_browser {
        std::process::exit(code.max(0));
    }

    let m = match CefString::from(&cmd_line.switch_value(Some(&CefString::from("mode"))))
        .to_string()
        .as_str()
    {
        "window" => Mode::Window,
        "none" => Mode::None,
        _ => Mode::Overlay,
    };
    ST.with(|s| {
        let mut s = s.borrow_mut();
        s.mode = Some(m);
        s.t0 = Some(std::time::Instant::now());
    });

    let root = std::env::var_os("LOCALAPPDATA")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("constellations-spike")
        .join("p0-04");
    let _ = std::fs::create_dir_all(&root);
    let mut app = SpikeApp::new();
    let settings = Settings {
        no_sandbox: 1,
        root_cache_path: CefString::from(root.to_string_lossy().as_ref()),
        ..Default::default()
    };
    if initialize(
        Some(args.as_main_args()),
        Some(&settings),
        Some(&mut app),
        std::ptr::null_mut(),
    ) != 1
    {
        eprintln!("CEF failed to initialise (is another copy using the same profile?)");
        std::process::exit(2);
    }
    run_message_loop();
    shutdown();
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn typed_addresses() {
        assert_eq!(target_for("javascript:alert(1)"), None);
        assert_eq!(target_for(" JavaScript:x"), None);
        assert_eq!(target_for("   "), None);
        assert_eq!(
            target_for("example.com").as_deref(),
            Some("https://example.com")
        );
        assert_eq!(
            target_for("star maps").as_deref(),
            Some("https://duckduckgo.com/?q=star+maps")
        );
        assert_eq!(percent_decode("a%20b%"), "a b%");
        assert_eq!(percent_decode("%E2%9C%A6"), "\u{2726}");
    }
}
