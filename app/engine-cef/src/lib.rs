//! CEF behind `engine-api`. Everything that touches CEF lives in this crate.
//!
//! Facts from Phase 0 that shape this code (DECISIONS D-003, D-009):
//! - A CEF window hosts at most one Chrome style BrowserView, and refuses a new one once any
//!   BrowserView has been added (P0-04, P1-02). So **each page view lives in its own frameless
//!   Chrome style window**, owned by the main window and placed over the page area. Switching
//!   tabs shows one page window and hides the others.
//! - Our own UI is an Alloy style BrowserView in the main window (no extensions needed there).
//! - Windows needs the app manifest (`app/shell/app.manifest`) or the GPU process crashes.
//!
//! All CEF callbacks and all [`Engine`] calls run on the UI thread, so state is thread-local.

use cef::*;
use engine_api::{
    DownloadInfo, Engine, Event, NewWindowKind, PermissionKind, Rect as ViewRect, ViewId,
};
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;

mod ui_scheme;

/// What the shell asks the engine to start with.
pub struct StartOptions {
    /// Folder for the profile (cache, cookies, settings written by the engine).
    pub profile_dir: PathBuf,
    /// When true, use an in-memory profile: no cookies, cache or storage written to disk (P3-02).
    pub in_memory: bool,
    /// Folder with our UI (`ui/`). `None` when it's missing: the window then says so.
    pub ui_dir: Option<PathBuf>,
    /// Appended to the UI URL, e.g. `#ipc-selftest` or `?fixture=Main`.
    pub ui_suffix: String,
    /// Window title (the product name; one place, DESIGN.md §11).
    pub title: String,
    /// Unpacked extension folders to load at start (`--load-extension`, verified in P0-05).
    pub extension_paths: Vec<String>,
}

static EXTENSION_PATHS: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();

/// Why the engine could not start, in words we can show the user.
#[derive(Debug)]
pub enum StartError {
    /// CEF refused to start, most often because another copy uses the same profile.
    InitializeFailed,
    /// The command line could not be read.
    BadCommandLine,
}

impl std::fmt::Display for StartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StartError::InitializeFailed => write!(
                f,
                "The browser engine could not start. Another copy may already be open."
            ),
            StartError::BadCommandLine => write!(f, "The command line could not be read."),
        }
    }
}

/// What a process run turned out to be.
pub enum Role {
    /// A CEF helper process (renderer, GPU, ...) that has finished: exit with this code.
    Helper(i32),
    /// The main browser process: the UI ran and the user closed it.
    BrowserFinished,
}

struct Page {
    window: Option<Window>,
    browser: Option<Browser>,
    bounds: ViewRect,
    visible: bool,
    sleeping: bool,
    saved_url: Option<String>,
    /// Last scroll position the page reported (for restoring after sleep).
    scroll_y: f64,
    restore_scroll: Option<f64>,
}

#[derive(Default)]
struct State {
    options: Option<StartOptions>,
    main: Option<Window>,
    ui: Option<BrowserView>,
    ui_closed: bool,
    pages: HashMap<ViewId, Page>,
    next_id: ViewId,
    cef_downloads: HashMap<u32, DownloadItemCallback>,
    /// URL prefix of our UI folder; only frames under it get the bridge (P1-03).
    ui_prefix: String,
    shields: Option<Arc<shields::Shields>>,
    /// Permission prompts waiting for the user's answer, by prompt id.
    perm_callbacks: HashMap<u64, PermissionPromptCallback>,
    /// Certificate errors waiting for the user's answer, by our own id.
    cert_callbacks: HashMap<u64, Callback>,
    next_cert_id: u64,
    /// Rectangles (main-window DIP, corner radius) where our UI draws over the page area. Page
    /// windows cover the UI, so each of these is cut out of them (AUDIT-06).
    #[cfg_attr(not(windows), allow(dead_code))]
    overlay_holes: Vec<(ViewRect, i32)>,
    /// While true the notch is slid away and only a thin strip of it is cut out of the page
    /// windows (the hover strip that brings it back).
    #[cfg_attr(not(windows), allow(dead_code))]
    notch_hidden: bool,
}

type Sink = Box<dyn FnMut(Event)>;
type UiSink = Box<dyn FnMut(String)>;
type Ready = Box<dyn FnOnce(CefEngine)>;

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
    static SINK: RefCell<Option<Sink>> = const { RefCell::new(None) };
    static UI_SINK: RefCell<Option<UiSink>> = const { RefCell::new(None) };
    static QUEUE: RefCell<VecDeque<Event>> = const { RefCell::new(VecDeque::new()) };
    static DISPATCHING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static ON_READY: RefCell<Option<Ready>> = const { RefCell::new(None) };
}

/// Deliver an event to the shell. Events raised while the shell is handling one (because it
/// called back into the engine) are queued and delivered right after, in order.
fn emit(event: Event) {
    QUEUE.with(|q| q.borrow_mut().push_back(event));
    if DISPATCHING.with(|d| d.replace(true)) {
        return;
    }
    while let Some(e) = QUEUE.with(|q| q.borrow_mut().pop_front()) {
        let sink = SINK.with(|s| s.borrow_mut().take());
        if let Some(mut sink) = sink {
            sink(e);
            SINK.with(|s| {
                let mut s = s.borrow_mut();
                if s.is_none() {
                    *s = Some(sink);
                }
            });
        }
    }
    DISPATCHING.with(|d| d.set(false));
}

fn maybe_quit() {
    let done = STATE.with(|s| {
        let s = s.borrow();
        s.ui_closed && s.pages.is_empty()
    });
    if done {
        quit_message_loop();
    }
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
        pub fn CombineRgn(dst: isize, src1: isize, src2: isize, mode: i32) -> i32;
        pub fn DeleteObject(ho: isize) -> i32;
    }
    pub const RGN_DIFF: i32 = 4;
}

#[cfg(windows)]
fn update_page_window_region(
    win: &Window,
    width: i32,
    height: i32,
    origin_x: i32,
    holes: &[(ViewRect, i32)],
    notch_hidden: bool,
) {
    unsafe {
        let h = win.window_handle().0 as isize;
        if h == 0 {
            return;
        }
        let dpi = win32::GetDpiForWindow(h).max(96) as f32 / 96.0;

        let w = (width as f32 * dpi).round() as i32;
        let h_px = (height as f32 * dpi).round() as i32;

        // Rounded left corners (DESIGN.md §4: 16 0 0 16). The rectangle is made wider than the
        // window so only the left corners show as rounded.
        let r2 = (32.0 * dpi).round() as i32;
        let base_rgn = win32::CreateRoundRectRgn(0, 0, w + r2, h_px + 1, r2, r2);

        // Notch in top-right corner: height 34 DIP, width 224 DIP.
        // Cutting out the notch leaves the main window's notch controls (shields, minimize,
        // maximize, close) visible and interactive.
        let notch_w = (224.0 * dpi).round() as i32;
        // Hidden: only a 4 DIP strip along the top stays cut out, so the pointer can reach it.
        let notch_h = if notch_hidden {
            (4.0 * dpi).round() as i32
        } else {
            (34.0 * dpi).round() as i32
        };
        // Only the bottom-left corner is rounded (--r-notch: 0 0 0 14px): the round rectangle is
        // made taller and wider than the window so the other corners fall outside it.
        let r2 = (28.0 * dpi).round() as i32;
        // The UI notch sits NOTCH_SHIFT past the right edge (--notch-shift in tokens.css), so its
        // hole starts that much further right.
        const NOTCH_SHIFT: f32 = 0.0;
        let shift = (NOTCH_SHIFT * dpi).round() as i32;
        let notch_rgn = win32::CreateRoundRectRgn(
            (w - notch_w + shift).max(0),
            -r2,
            w + r2,
            notch_h + 1,
            r2,
            r2,
        );
        win32::CombineRgn(base_rgn, base_rgn, notch_rgn, win32::RGN_DIFF);
        win32::DeleteObject(notch_rgn);

        // Helm at bottom: 560 DIP wide, 52 DIP high, 28 DIP from bottom, centered in page area.
        let helm_w = (560.0 * dpi).round() as i32;
        let helm_h = (52.0 * dpi).round() as i32;
        let helm_bottom = (28.0 * dpi).round() as i32;
        let helm_x = ((w - helm_w) / 2).max(0);
        let helm_y = h_px - helm_bottom - helm_h;
        if helm_y > 0 && w >= helm_w {
            let helm_rgn = win32::CreateRoundRectRgn(
                helm_x,
                helm_y,
                helm_x + helm_w,
                helm_y + helm_h,
                helm_h,
                helm_h,
            );
            win32::CombineRgn(base_rgn, base_rgn, helm_rgn, win32::RGN_DIFF);
            win32::DeleteObject(helm_rgn);
        }

        // Our UI's open overlays (popovers, cards, find bar): cut them out too.
        for (r, radius) in holes {
            let x0 = (((r.x - origin_x) as f32) * dpi).round() as i32;
            let y0 = ((r.y as f32) * dpi).round() as i32;
            let x1 = x0 + ((r.width as f32) * dpi).round() as i32;
            let y1 = y0 + ((r.height as f32) * dpi).round() as i32;
            if x1 <= 0 || y1 <= 0 || x0 >= w || y0 >= h_px {
                continue;
            }
            let d = ((*radius as f32) * 2.0 * dpi).round() as i32;
            let rgn =
                win32::CreateRoundRectRgn(x0.max(0), y0.max(0), x1.min(w), y1.min(h_px), d, d);
            win32::CombineRgn(base_rgn, base_rgn, rgn, win32::RGN_DIFF);
            win32::DeleteObject(rgn);
        }

        win32::SetWindowRgn(h, base_rgn, 1);
    }
}

/// Re-cut the holes in every visible page window (overlays moved or appeared).
#[cfg(windows)]
fn refresh_holes() {
    STATE.with(|s| {
        let s = s.borrow();
        let Some(main) = s.main.as_ref() else { return };
        let c = main.client_area_bounds_in_screen();
        for page in s.pages.values().filter(|p| p.visible) {
            let Some(win) = page.window.as_ref() else {
                continue;
            };
            let b = page.bounds;
            update_page_window_region(
                win,
                (c.width - b.x).max(1),
                c.height.max(1),
                b.x,
                &s.overlay_holes,
                s.notch_hidden,
            );
        }
    });
}

/// Put a page window where the shell wants it: client-area DIP → screen DIP.
fn place(id: ViewId) {
    STATE.with(|s| {
        let s = s.borrow();
        let (Some(main), Some(page)) = (s.main.as_ref(), s.pages.get(&id)) else {
            return;
        };
        let Some(win) = page.window.as_ref() else {
            return;
        };
        let c = main.client_area_bounds_in_screen();
        let b = page.bounds;
        let pw = (c.width - b.x).max(1);
        let ph = c.height.max(1);
        win.set_bounds(Some(&Rect {
            x: c.x + b.x,
            y: c.y + b.y,
            width: pw,
            height: ph,
        }));
        if page.visible {
            #[cfg(windows)]
            update_page_window_region(win, pw, ph, b.x, &s.overlay_holes, s.notch_hidden);
            win.show();
        } else {
            win.hide();
        }
    });
}

thread_local! {
    static IN_PLACE_ALL: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

fn place_all() {
    let reentered = IN_PLACE_ALL.with(|g| {
        if g.get() {
            true
        } else {
            g.set(true);
            false
        }
    });
    if reentered {
        return;
    }
    struct Guard;
    impl Drop for Guard {
        fn drop(&mut self) {
            IN_PLACE_ALL.with(|g| g.set(false));
        }
    }
    let _guard = Guard;

    STATE.with(|s| {
        let s = s.borrow();
        if let (Some(main), Some(ui)) = (s.main.as_ref(), s.ui.as_ref()) {
            let c = main.client_area_bounds_in_screen();
            let v = View::from(ui);
            v.set_bounds(Some(&Rect {
                x: 0,
                y: 0,
                width: c.width,
                height: c.height,
            }));
            main.layout();
        }
    });
    let ids: Vec<ViewId> = STATE.with(|s| s.borrow().pages.keys().copied().collect());
    for id in ids {
        place(id);
    }
}

fn with_browser(id: ViewId, f: impl FnOnce(&Browser)) {
    let b = STATE.with(|s| s.borrow().pages.get(&id).and_then(|p| p.browser.clone()));
    if let Some(b) = b {
        f(&b);
    }
}

// ---- main window ----------------------------------------------------------------------------

wrap_window_delegate! {
    struct MainWindow {
        ui: RefCell<Option<BrowserView>>,
        title: String,
    }

    impl ViewDelegate {
        fn preferred_size(&self, _view: Option<&mut View>) -> Size {
            Size { width: 1280, height: 800 }
        }
        fn minimum_size(&self, _view: Option<&mut View>) -> Size {
            // EDGE_CASES §1: smallest allowed window.
            Size { width: 800, height: 600 }
        }
        fn on_layout_changed(&self, _view: Option<&mut View>, _bounds: Option<&Rect>) {
            place_all();
        }
    }

    impl PanelDelegate {}

    impl WindowDelegate {
        fn on_window_created(&self, window: Option<&mut Window>) {
            let Some(window) = window else { return };
            window.set_background_color(0xFF000000);
            window.set_title(Some(&CefString::from(self.title.as_str())));
            window.set_to_fill_layout();
            if let Some(ui) = self.ui.borrow().as_ref() {
                window.add_child_view(Some(&mut View::from(ui)));
            }
            window.layout();
            STATE.with(|s| {
                let mut s = s.borrow_mut();
                s.main = Some(window.clone());
                s.ui = self.ui.borrow().clone();
            });
            window.show();
            place_all();
            if let Some(ready) = ON_READY.with(|r| r.borrow_mut().take()) {
                ready(CefEngine);
            }
        }

        fn on_window_bounds_changed(&self, window: Option<&mut Window>, _bounds: Option<&Rect>) {
            if let Some(w) = window {
                w.layout();
            }
            place_all();
        }

        fn on_window_destroyed(&self, _window: Option<&mut Window>) {
            *self.ui.borrow_mut() = None;
            STATE.with(|s| s.borrow_mut().main = None);
        }

        fn can_close(&self, _window: Option<&mut Window>) -> i32 {
            // Closing the main window closes every page with it.
            let windows: Vec<Window> = STATE.with(|s| {
                s.borrow().pages.values().filter_map(|p| p.window.clone()).collect()
            });
            for w in windows {
                w.close();
            }
            let ui = self.ui.borrow();
            match ui.as_ref().and_then(|v| v.browser()).and_then(|b| b.host()) {
                Some(host) => host.try_close_browser(),
                None => 1,
            }
        }

        fn is_frameless(&self, _window: Option<&mut Window>) -> i32 {
            1
        }
        fn can_resize(&self, _window: Option<&mut Window>) -> i32 {
            1
        }
        fn can_maximize(&self, _window: Option<&mut Window>) -> i32 {
            1
        }
        fn can_minimize(&self, _window: Option<&mut Window>) -> i32 {
            1
        }

        fn window_runtime_style(&self) -> RuntimeStyle {
            RuntimeStyle::CHROME
        }
    }
}

wrap_browser_view_delegate! {
    struct UiView {}

    impl ViewDelegate {}

    impl BrowserViewDelegate {
        fn browser_runtime_style(&self) -> RuntimeStyle {
            RuntimeStyle::ALLOY
        }
    }
}

wrap_life_span_handler! {
    struct UiLife {}

    impl LifeSpanHandler {
        fn on_before_close(&self, _browser: Option<&mut Browser>) {
            STATE.with(|s| s.borrow_mut().ui_closed = true);
            maybe_quit();
        }
    }
}

wrap_load_handler! {
    struct UiLoad {}

    impl LoadHandler {
        fn on_load_error(
            &self,
            _browser: Option<&mut Browser>,
            _frame: Option<&mut Frame>,
            error_code: Errorcode,
            error_text: Option<&CefString>,
            failed_url: Option<&CefString>,
        ) {
            // Never a silent failure (EDGE_CASES §1): log it for debugging.
            eprintln!(
                "UI failed to load: {:?} {} {}",
                sys::cef_errorcode_t::from(error_code),
                error_text.map(CefString::to_string).unwrap_or_default(),
                failed_url.map(CefString::to_string).unwrap_or_default()
            );
        }
    }
}

/// Name of the process message our UI sends (renderer → browser).
const BRIDGE_MESSAGE: &str = "odyssey-ui-bridge";
/// Command-line switch that tells renderer processes where our UI lives.
const UI_PREFIX_SWITCH: &str = "odyssey-ui-prefix";
/// The JS function our UI calls; only defined in our UI's main frame.
const BRIDGE_FUNCTION: &str = "odysseyBridge";

fn deliver_ui_message(json: String) {
    let sink = UI_SINK.with(|s| s.borrow_mut().take());
    if let Some(mut sink) = sink {
        sink(json);
        UI_SINK.with(|s| {
            let mut s = s.borrow_mut();
            if s.is_none() {
                *s = Some(sink);
            }
        });
    }
}

wrap_drag_handler! {
    struct UiDrag {}

    impl DragHandler {
        fn on_draggable_regions_changed(
            &self,
            _browser: Option<&mut Browser>,
            _frame: Option<&mut Frame>,
            regions: Option<&[DraggableRegion]>,
        ) {
            STATE.with(|s| {
                if let Some(main) = s.borrow().main.as_ref() {
                    main.set_draggable_regions(regions);
                }
            });
        }
    }
}

wrap_client! {
    struct UiClient {}

    impl Client {
        fn drag_handler(&self) -> Option<DragHandler> {
            Some(UiDrag::new())
        }
        fn life_span_handler(&self) -> Option<LifeSpanHandler> {
            Some(UiLife::new())
        }
        fn load_handler(&self) -> Option<LoadHandler> {
            Some(UiLoad::new())
        }
        // Only this client (our UI's browser) listens for bridge messages. Page views use
        // `PageClient`, which has no such handler, so a page can never reach Rust (P1-03).
        fn on_process_message_received(
            &self,
            _browser: Option<&mut Browser>,
            frame: Option<&mut Frame>,
            source_process: ProcessId,
            message: Option<&mut ProcessMessage>,
        ) -> i32 {
            let Some(message) = message else { return 0 };
            if source_process != ProcessId::RENDERER
                || CefString::from(&message.name()).to_string() != BRIDGE_MESSAGE
            {
                return 0;
            }
            // Defence in depth: the sending frame must be our UI's main frame.
            let prefix = STATE.with(|s| s.borrow().ui_prefix.clone());
            let ok_frame = frame.is_some_and(|f| {
                f.is_main() != 0 && CefString::from(&f.url()).to_string().starts_with(&prefix)
            });
            if !ok_frame || prefix.is_empty() {
                eprintln!("bridge: ignored a message from a frame that is not our UI");
                return 1;
            }
            if let Some(args) = message.argument_list() {
                deliver_ui_message(CefString::from(&args.string(0)).to_string());
            }
            1
        }
    }
}

// ---- renderer side of the bridge (runs in renderer processes) ------------------------------

wrap_v8_handler! {
    struct BridgeFunction {}

    impl V8Handler {
        fn execute(
            &self,
            _name: Option<&CefString>,
            _object: Option<&mut V8Value>,
            arguments: Option<&[Option<V8Value>]>,
            _retval: Option<&mut Option<V8Value>>,
            exception: Option<&mut CefString>,
        ) -> i32 {
            let text = arguments
                .and_then(|a| a.first().cloned().flatten())
                .filter(|v| v.is_string() != 0)
                .map(|v| CefString::from(&v.string_value()).to_string());
            let Some(text) = text else {
                if let Some(e) = exception {
                    *e = CefString::from("odysseyBridge expects one JSON string");
                }
                return 1;
            };
            let frame = v8_context_get_current_context().and_then(|c| c.frame());
            let msg = process_message_create(Some(&CefString::from(BRIDGE_MESSAGE)));
            if let (Some(frame), Some(mut msg)) = (frame, msg) {
                if let Some(args) = msg.argument_list() {
                    args.set_string(0, Some(&CefString::from(text.as_str())));
                }
                frame.send_process_message(ProcessId::BROWSER, Some(&mut msg));
            }
            1
        }
    }
}

wrap_render_process_handler! {
    struct BridgeRenderer {}

    impl RenderProcessHandler {
        fn on_context_created(
            &self,
            _browser: Option<&mut Browser>,
            frame: Option<&mut Frame>,
            context: Option<&mut V8Context>,
        ) {
            let (Some(frame), Some(context)) = (frame, context) else { return };
            if frame.is_main() == 0 {
                return;
            }
            let prefix = command_line_get_global()
                .map(|c| {
                    CefString::from(&c.switch_value(Some(&CefString::from(UI_PREFIX_SWITCH))))
                        .to_string()
                })
                .unwrap_or_default();
            let url = CefString::from(&frame.url()).to_string();
            // Only our UI gets the bridge. Web pages, data: pages, extension pages: never.
            if prefix.is_empty() || !url.starts_with(&prefix) {
                return;
            }
            let Some(global) = context.global() else { return };
            let mut handler = BridgeFunction::new();
            let name = CefString::from(BRIDGE_FUNCTION);
            if let Some(mut f) = v8_value_create_function(Some(&name), Some(&mut handler)) {
                let attr = V8Propertyattribute::from(sys::cef_v8_propertyattribute_t(
                    sys::cef_v8_propertyattribute_t::V8_PROPERTY_ATTRIBUTE_READONLY.0
                        | sys::cef_v8_propertyattribute_t::V8_PROPERTY_ATTRIBUTE_DONTDELETE.0,
                ));
                global.set_value_bykey(Some(&name), Some(&mut f), attr);
            }
        }
    }
}

// ---- page views ---------------------------------------------------------------------------

wrap_window_delegate! {
    struct PageWindow {
        id: ViewId,
        view: RefCell<Option<BrowserView>>,
    }

    impl ViewDelegate {}

    impl PanelDelegate {}

    impl WindowDelegate {
        fn on_window_created(&self, window: Option<&mut Window>) {
            let Some(window) = window else { return };
            window.set_to_fill_layout();
            if let Some(v) = self.view.borrow().as_ref() {
                window.add_child_view(Some(&mut View::from(v)));
            }
            window.layout();
            STATE.with(|s| {
                if let Some(p) = s.borrow_mut().pages.get_mut(&self.id) {
                    p.window = Some(window.clone());
                }
            });
            place(self.id);
        }
        fn on_window_bounds_changed(&self, window: Option<&mut Window>, _bounds: Option<&Rect>) {
            if let (Some(w), Some(v)) = (window, self.view.borrow().as_ref()) {
                let c = w.client_area_bounds_in_screen();
                let view = View::from(v);
                view.set_bounds(Some(&Rect {
                    x: 0,
                    y: 0,
                    width: c.width,
                    height: c.height,
                }));
                w.layout();
            }
        }
        fn on_window_destroyed(&self, _window: Option<&mut Window>) {
            *self.view.borrow_mut() = None;
        }
        fn can_close(&self, _window: Option<&mut Window>) -> i32 {
            let view = self.view.borrow();
            match view.as_ref().and_then(|v| v.browser()).and_then(|b| b.host()) {
                Some(host) => host.try_close_browser(),
                None => 1,
            }
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
        // Owned by the main window: above it, moves and minimises with it, no taskbar button.
        fn parent_window(&self, _window: Option<&mut Window>, is_menu: Option<&mut i32>, can_activate_menu: Option<&mut i32>) -> Option<Window> {
            if let Some(m) = is_menu {
                *m = 0;
            }
            if let Some(c) = can_activate_menu {
                *c = 1;
            }
            STATE.with(|s| s.borrow().main.clone())
        }
        fn window_runtime_style(&self) -> RuntimeStyle {
            RuntimeStyle::CHROME
        }
    }
}

wrap_browser_view_delegate! {
    struct PageView {}

    impl ViewDelegate {}

    impl BrowserViewDelegate {
        fn browser_runtime_style(&self) -> RuntimeStyle {
            RuntimeStyle::CHROME
        }
    }
}

wrap_life_span_handler! {
    struct PageLife { id: ViewId }

    impl LifeSpanHandler {
        fn on_after_created(&self, browser: Option<&mut Browser>) {
            let Some(b) = browser else { return };
            STATE.with(|s| {
                if let Some(p) = s.borrow_mut().pages.get_mut(&self.id) {
                    p.browser = Some(b.clone());
                }
            });
        }

        fn on_before_popup(
            &self,
            _browser: Option<&mut Browser>,
            _frame: Option<&mut Frame>,
            _popup_id: i32,
            target_url: Option<&CefString>,
            _target_frame_name: Option<&CefString>,
            target_disposition: WindowOpenDisposition,
            user_gesture: i32,
            _popup_features: Option<&PopupFeatures>,
            _window_info: Option<&mut WindowInfo>,
            _client: Option<&mut Option<Client>>,
            _settings: Option<&mut BrowserSettings>,
            _extra_info: Option<&mut Option<DictionaryValue>>,
            _no_javascript_access: Option<&mut i32>,
        ) -> i32 {
            let kind = if target_disposition == WindowOpenDisposition::NEW_BACKGROUND_TAB {
                NewWindowKind::BackgroundTab
            } else if target_disposition == WindowOpenDisposition::NEW_POPUP {
                NewWindowKind::Popup
            } else if target_disposition == WindowOpenDisposition::NEW_WINDOW {
                NewWindowKind::Window
            } else {
                NewWindowKind::ForegroundTab
            };
            emit(Event::NewWindowRequested {
                opener: self.id,
                url: target_url.map(CefString::to_string).unwrap_or_default(),
                kind,
                user_gesture: user_gesture != 0,
            });
            // The browser opens it as a tab in our Trail (or not); never a native popup.
            1
        }

        fn on_before_close(&self, _browser: Option<&mut Browser>) {
            let window = STATE.with(|s| s.borrow_mut().pages.remove(&self.id).and_then(|p| p.window));
            if let Some(w) = window {
                w.close();
            }
            emit(Event::Closed { view: self.id });
            maybe_quit();
        }
    }
}

wrap_display_handler! {
    struct PageDisplay { id: ViewId }

    impl DisplayHandler {
        fn on_address_change(&self, _browser: Option<&mut Browser>, frame: Option<&mut Frame>, url: Option<&CefString>) {
            if frame.map(|f| f.is_main() != 0) != Some(true) {
                return;
            }
            emit(Event::UrlChanged { view: self.id, url: url.map(CefString::to_string).unwrap_or_default() });
        }

        fn on_title_change(&self, _browser: Option<&mut Browser>, title: Option<&CefString>) {
            emit(Event::TitleChanged { view: self.id, title: title.map(CefString::to_string).unwrap_or_default() });
        }

        // Pages have no bridge. The small script we inject (FORM_WATCH_JS) reports one boolean
        // through the console; a page could fake it, which would only stop its own tab sleeping.
        fn on_console_message(
            &self,
            _browser: Option<&mut Browser>,
            _level: LogSeverity,
            message: Option<&CefString>,
            _source: Option<&CefString>,
            _line: ::std::os::raw::c_int,
        ) -> ::std::os::raw::c_int {
            let text = message.map(CefString::to_string).unwrap_or_default();
            if let Some(v) = text.strip_prefix("__ody_form__:") {
                emit(Event::FormInputChanged { view: self.id, has_input: v == "1" });
                return 1;
            }
            if let Some(v) = text.strip_prefix("__ody_login__:") {
                if v.len() < 4096
                    && let Ok(j) = serde_json::from_str::<serde_json::Value>(v)
                    && let (Some(u), Some(p)) = (j["u"].as_str(), j["p"].as_str())
                {
                    emit(Event::LoginSubmitted {
                        view: self.id,
                        username: u.to_string(),
                        password: p.to_string(),
                    });
                }
                return 1;
            }
            if text == "__ody_fill__:1" {
                emit(Event::LoginFieldClicked { view: self.id });
                return 1;
            }
            if let Some(v) = text.strip_prefix("__ody_scroll__:") {
                let y = v.parse::<f64>().unwrap_or(0.0).max(0.0);
                STATE.with(|s| {
                    if let Some(p) = s.borrow_mut().pages.get_mut(&self.id) {
                        p.scroll_y = y;
                    }
                });
                return 1;
            }
            if let Some(v) = text.strip_prefix("__ody_audio__:") {
                emit(Event::AudioChanged { view: self.id, playing: v == "1" });
                return 1;
            }
            0
        }
    }
}

/// Login forms (AUDIT: passwords). Reports a submitted login through the console, and a real
/// (trusted) click on a login field so the browser can autofill. Pages have no bridge; what the
/// page could see here it already has (the password it was just given).
const LOGIN_WATCH_JS: &str = r#"(function(){try{
function vis(e){var r=e.getBoundingClientRect();return r.width>8&&r.height>8&&getComputedStyle(e).visibility!=='hidden';}
function pwIn(scope){var l=(scope||document).querySelectorAll('input[type=password]');for(var i=0;i<l.length;i++){if(vis(l[i]))return l[i];}return null;}
function userFor(p){var scope=p.form||document;var l=scope.querySelectorAll('input');var u=null;for(var i=0;i<l.length;i++){var e=l[i];if(e===p)break;var t=(e.type||'text').toLowerCase();if((t==='text'||t==='email'||t==='tel')&&vis(e))u=e;}return u;}
function report(p){var v=p.value;if(!v)return;var u=userFor(p);console.debug('__ody_login__:'+JSON.stringify({u:u?u.value:'',p:v}));}
document.addEventListener('submit',function(e){var p=e.target&&e.target.querySelector?pwIn(e.target):null;if(p)report(p);},true);
document.addEventListener('click',function(e){var t=e.target;var b=t&&t.closest?t.closest('button,input[type=submit],input[type=button]'):null;if(b&&(b.type==='submit'||!b.form)){var f=b.form||b.closest('form');var p=pwIn(f||document);if(p&&p.value)report(p);}
if(!e.isTrusted||!t||t.tagName!=='INPUT')return;var ty=(t.type||'text').toLowerCase();if(ty==='password'||((ty==='text'||ty==='email'||ty==='tel')&&pwIn(t.form||document))){console.debug('__ody_fill__:1');}},true);
document.addEventListener('keydown',function(e){var t=e.target;if(e.key==='Enter'&&t&&t.type==='password'&&t.value)report(t);},true);
}catch(e){}})();"#;

/// Fills the login form the user just clicked. Runs once with the credentials baked in; nothing
/// stays in the page afterwards.
fn fill_login_script(username: &str, password: &str) -> String {
    let u = serde_json::to_string(username).unwrap_or_else(|_| String::from("\"\""));
    let p = serde_json::to_string(password).unwrap_or_else(|_| String::from("\"\""));
    format!(
        r#"(function(u,p){{try{{
function vis(e){{var r=e.getBoundingClientRect();return r.width>8&&r.height>8&&getComputedStyle(e).visibility!=='hidden';}}
var a=document.activeElement;var scope=(a&&a.tagName==='INPUT'&&a.form)?a.form:document;
var pw=null;var l=scope.querySelectorAll('input[type=password]');for(var i=0;i<l.length;i++){{if(vis(l[i])){{pw=l[i];break;}}}}
if(!pw)return;
var user=null;var all=(pw.form||document).querySelectorAll('input');for(var j=0;j<all.length;j++){{var e=all[j];if(e===pw)break;var t=(e.type||'text').toLowerCase();if((t==='text'||t==='email'||t==='tel')&&vis(e))user=e;}}
function set(e,v){{if(!e)return;Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set.call(e,v);e.dispatchEvent(new Event('input',{{bubbles:true}}));e.dispatchEvent(new Event('change',{{bubbles:true}}));}}
set(user,u);set(pw,p);
}}catch(x){{}}}})({u},{p});"#
    )
}

/// Reports two booleans through the console: unsubmitted text in a form field, and audio playing
/// (media elements or a running AudioContext created after load). Used to keep tabs awake.
const FORM_WATCH_JS: &str = "(function(){try{var f=false,au=false,ctxs=[];function r(k,v){console.debug('__ody_'+k+'__:'+(v?1:0));}document.addEventListener('input',function(e){var t=e.target;if(t&&(t.tagName==='INPUT'||t.tagName==='TEXTAREA'||t.isContentEditable)&&!f){f=true;r('form',true);}},true);document.addEventListener('submit',function(){if(f){f=false;r('form',false);}},true);var AC=window.AudioContext||window.webkitAudioContext;if(AC){var P=function(){var c=new (Function.prototype.bind.apply(AC,[null].concat([].slice.call(arguments))))();ctxs.push(c);return c;};P.prototype=AC.prototype;window.AudioContext=P;}setInterval(function(){var on=false;var m=document.querySelectorAll('audio,video');for(var i=0;i<m.length;i++){var e=m[i];if(!e.paused&&!e.ended&&!e.muted&&e.volume>0&&e.readyState>2){on=true;break;}}if(!on){for(var j=0;j<ctxs.length;j++){if(ctxs[j].state==='running'){on=true;break;}}}if(on!==au){au=on;r('audio',on);}},2000);var st;addEventListener('scroll',function(){clearTimeout(st);st=setTimeout(function(){console.debug('__ody_scroll__:'+Math.round(window.scrollY));},400);},{passive:true,capture:true});}catch(e){}})();";

wrap_load_handler! {
    struct PageLoad { id: ViewId }

    impl LoadHandler {
        fn on_loading_state_change(&self, _browser: Option<&mut Browser>, is_loading: i32, can_go_back: i32, can_go_forward: i32) {
            emit(Event::LoadingChanged {
                view: self.id,
                loading: is_loading != 0,
                can_go_back: can_go_back != 0,
                can_go_forward: can_go_forward != 0,
            });
        }

        fn on_load_end(&self, _browser: Option<&mut Browser>, frame: Option<&mut Frame>, _http_status_code: i32) {
            let Some(f) = frame else { return };
            if f.is_main() == 0 {
                return;
            }
            let url_str = CefString::from(&f.url()).to_string();
            // A new document starts with no typed input.
            emit(Event::FormInputChanged { view: self.id, has_input: false });
            emit(Event::AudioChanged { view: self.id, playing: false });
            // After a wake, put the page back where it was; otherwise a new document starts at 0.
            let restore = STATE.with(|s| {
                s.borrow_mut().pages.get_mut(&self.id).and_then(|p| {
                    if p.sleeping {
                        return None;
                    }
                    let r = p.restore_scroll.take();
                    p.scroll_y = r.unwrap_or(0.0);
                    r
                })
            });
            if let Some(y) = restore.filter(|y| *y > 0.0) {
                let js = format!("scrollTo(0,{y});setTimeout(function(){{scrollTo(0,{y});}},700);");
                f.execute_java_script(Some(&CefString::from(js.as_str())), None, 0);
            }
            f.execute_java_script(Some(&CefString::from(FORM_WATCH_JS)), None, 0);
            f.execute_java_script(Some(&CefString::from(LOGIN_WATCH_JS)), None, 0);
            let shields = STATE.with(|s| s.borrow().shields.clone());
            if let Some(sh) = shields {
                let css = sh.cosmetic_css(&url_str);
                if !css.is_empty() {
                    let script = format!(
                        "(function(){{try{{var s=document.createElement('style');s.setAttribute('data-constellations','shields');s.textContent={};(document.head||document.documentElement).appendChild(s);}}catch(e){{}}}})();",
                        serde_json::to_string(&css).unwrap_or_else(|_| "''".into())
                    );
                    f.execute_java_script(Some(&CefString::from(script.as_str())), None, 0);
                }
            }
        }

        fn on_load_error(
            &self,
            _browser: Option<&mut Browser>,
            frame: Option<&mut Frame>,
            error_code: Errorcode,
            error_text: Option<&CefString>,
            failed_url: Option<&CefString>,
        ) {
            if frame.map(|f| f.is_main() != 0) != Some(true) {
                return;
            }
            let code = sys::cef_errorcode_t::from(error_code) as i32;
            // ERR_ABORTED (-3) is a navigation we replaced or a download, not a failure.
            if code == -3 {
                return;
            }
            emit(Event::LoadFailed {
                view: self.id,
                url: failed_url.map(CefString::to_string).unwrap_or_default(),
                code,
                text: error_text.map(CefString::to_string).unwrap_or_default(),
            });
        }
    }
}

wrap_download_handler! {
    struct PageDownload { id: ViewId }

    impl DownloadHandler {
        fn can_download(
            &self,
            _browser: Option<&mut Browser>,
            _url: Option<&CefString>,
            _request_method: Option<&CefString>,
        ) -> i32 {
            1
        }

        fn on_before_download(
            &self,
            _browser: Option<&mut Browser>,
            download_item: Option<&mut DownloadItem>,
            suggested_name: Option<&CefString>,
            callback: Option<&mut BeforeDownloadCallback>,
        ) -> i32 {
            let Some(item) = download_item else { return 0 };
            let Some(callback) = callback else { return 0 };
            let id = item.id();
            let url = CefString::from(&item.url()).to_string();
            let name = suggested_name
                .map(CefString::to_string)
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| CefString::from(&item.suggested_file_name()).to_string());
            let dir = segdl::default_download_dir();
            let target = segdl::unique_path(&dir, &name);
            let target_str = target.to_string_lossy().to_string();

            segdl::clear_taken(&segdl::path_key(&target_str), &url);

            callback.cont(Some(&CefString::from(target_str.as_str())), 0);

            let total = item.total_bytes();
            let file_name = target
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or(name);

            emit(Event::Download(DownloadInfo {
                id,
                view: Some(self.id),
                url: url.clone(),
                file_name: file_name.clone(),
                received_bytes: 0,
                total_bytes: if total > 0 { Some(total) } else { None },
                bytes_per_second: 0,
                complete: false,
                cancelled: false,
                interrupted: false,
                path: target_str.clone(),
            }));

            if segdl::enabled() && (url.starts_with("http://") || url.starts_with("https://")) {
                let req = segdl::Request {
                    url: url.clone(),
                    path: target,
                    cookie: None,
                    referer: None,
                };
                let view_id = self.id;
                let url_for_emit = url.clone();
                let file_name_for_emit = file_name;
                let path_for_emit = target_str.clone();
                let emit_fn: segdl::Emit = Arc::new(move |ev, payload| {
                    if ev == "download-started" {
                        let total = payload.get("total").and_then(|v| v.as_i64());
                        emit(Event::Download(DownloadInfo {
                            id,
                            view: Some(view_id),
                            url: url_for_emit.clone(),
                            file_name: file_name_for_emit.clone(),
                            received_bytes: 0,
                            total_bytes: total,
                            bytes_per_second: 0,
                            complete: false,
                            cancelled: false,
                            interrupted: false,
                            path: path_for_emit.clone(),
                        }));
                    } else if ev == "download-progress" {
                        let bytes = payload.get("bytes").and_then(|v| v.as_i64()).unwrap_or(0);
                        let total = payload.get("total").and_then(|v| v.as_i64());
                        let speed = payload
                            .get("speed")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.0) as i64;
                        emit(Event::Download(DownloadInfo {
                            id,
                            view: Some(view_id),
                            url: url_for_emit.clone(),
                            file_name: file_name_for_emit.clone(),
                            received_bytes: bytes,
                            total_bytes: total,
                            bytes_per_second: speed,
                            complete: false,
                            cancelled: false,
                            interrupted: false,
                            path: path_for_emit.clone(),
                        }));
                    } else if ev == "download-finished" {
                        let success = payload
                            .get("success")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);
                        emit(Event::Download(DownloadInfo {
                            id,
                            view: Some(view_id),
                            url: url_for_emit.clone(),
                            file_name: file_name_for_emit.clone(),
                            received_bytes: 0,
                            total_bytes: None,
                            bytes_per_second: 0,
                            complete: success,
                            cancelled: !success,
                            interrupted: false,
                            path: path_for_emit.clone(),
                        }));
                    }
                });

                segdl::spawn(emit_fn, req, move |taken| {
                    if taken {
                        let mut task = Later::new(std::rc::Rc::new(RefCell::new(Some(Box::new(move || {
                            STATE.with(|s| {
                                if let Some(cb) = s.borrow_mut().cef_downloads.remove(&id) {
                                    cb.cancel();
                                }
                            });
                        })))));
                        post_task(ThreadId::UI, Some(&mut task));
                    }
                });
            }

            1
        }

        fn on_download_updated(
            &self,
            _browser: Option<&mut Browser>,
            download_item: Option<&mut DownloadItem>,
            callback: Option<&mut DownloadItemCallback>,
        ) {
            let Some(item) = download_item else { return };
            let id = item.id();
            let url = CefString::from(&item.url()).to_string();
            let full_path = CefString::from(&item.full_path()).to_string();
            let path_key = segdl::path_key(&full_path);

            if let Some(cb) = callback.as_ref() {
                STATE.with(|s| {
                    s.borrow_mut().cef_downloads.insert(id, (*cb).clone());
                });
            }

            if segdl::was_taken(&path_key, &url) {
                if let Some(cb) = callback.as_ref() {
                    cb.cancel();
                }
                STATE.with(|s| {
                    s.borrow_mut().cef_downloads.remove(&id);
                });
                return;
            }

            let complete = item.is_complete() != 0;
            let cancelled = item.is_canceled() != 0;
            let interrupted = item.is_interrupted() != 0;

            if complete || cancelled || interrupted {
                STATE.with(|s| {
                    s.borrow_mut().cef_downloads.remove(&id);
                });
            }

            let total = item.total_bytes();
            let file_name = Path::new(&full_path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            emit(Event::Download(DownloadInfo {
                id,
                view: Some(self.id),
                url,
                file_name,
                received_bytes: item.received_bytes(),
                total_bytes: if total > 0 { Some(total) } else { None },
                bytes_per_second: item.current_speed(),
                complete,
                cancelled,
                interrupted,
                path: full_path.clone(),
            }));
        }
    }
}

/// The permission kind we show the user for CEF's permission bit mask.
fn permission_kind(mask: u32) -> PermissionKind {
    use sys::cef_permission_request_types_t as T;
    if mask & T::CEF_PERMISSION_TYPE_CAMERA_STREAM as u32 != 0 {
        PermissionKind::Camera
    } else if mask & T::CEF_PERMISSION_TYPE_MIC_STREAM as u32 != 0 {
        PermissionKind::Microphone
    } else if mask & T::CEF_PERMISSION_TYPE_GEOLOCATION as u32 != 0 {
        PermissionKind::Location
    } else if mask & T::CEF_PERMISSION_TYPE_NOTIFICATIONS as u32 != 0 {
        PermissionKind::Notifications
    } else if mask & T::CEF_PERMISSION_TYPE_CLIPBOARD as u32 != 0 {
        PermissionKind::Clipboard
    } else {
        PermissionKind::Other
    }
}

/// Plain words for the common certificate errors.
fn certificate_error_text(code: i32) -> String {
    match code {
        -200 => "The certificate doesn't match the site's address.".to_string(),
        -201 => "The site's certificate has expired or isn't valid yet.".to_string(),
        -202 => "The certificate wasn't issued by an authority we trust.".to_string(),
        -203 => "The certificate contains errors.".to_string(),
        -207 => "The certificate type isn't allowed.".to_string(),
        -208 => "The certificate has been revoked.".to_string(),
        -210 => "The certificate is weak.".to_string(),
        other => format!("The site's certificate can't be trusted (error {other})."),
    }
}

wrap_permission_handler! {
    struct PagePermission { id: ViewId }

    impl PermissionHandler {
        fn on_show_permission_prompt(
            &self,
            _browser: Option<&mut Browser>,
            prompt_id: u64,
            requesting_origin: Option<&CefString>,
            requested_permissions: u32,
            callback: Option<&mut PermissionPromptCallback>,
        ) -> ::std::os::raw::c_int {
            let Some(cb) = callback else { return 0 };
            let cb = cb.clone();
            STATE.with(|s| s.borrow_mut().perm_callbacks.insert(prompt_id, cb));
            emit(Event::PermissionRequested {
                view: self.id,
                request: prompt_id,
                origin: requesting_origin.map(CefString::to_string).unwrap_or_default(),
                kind: permission_kind(requested_permissions),
            });
            1
        }

        fn on_dismiss_permission_prompt(
            &self,
            _browser: Option<&mut Browser>,
            prompt_id: u64,
            _result: PermissionRequestResult,
        ) {
            STATE.with(|s| s.borrow_mut().perm_callbacks.remove(&prompt_id));
        }
    }
}

wrap_request_handler! {
    struct PageRequest { id: ViewId }

    impl RequestHandler {
        fn on_certificate_error(
            &self,
            _browser: Option<&mut Browser>,
            cert_error: Errorcode,
            request_url: Option<&CefString>,
            _ssl_info: Option<&mut Sslinfo>,
            callback: Option<&mut Callback>,
        ) -> ::std::os::raw::c_int {
            let Some(cb) = callback else { return 0 };
            let cb = cb.clone();
            let code = sys::cef_errorcode_t::from(cert_error) as i32;
            let request = STATE.with(|s| {
                let mut s = s.borrow_mut();
                s.next_cert_id += 1;
                let id = s.next_cert_id;
                s.cert_callbacks.insert(id, cb);
                id
            });
            emit(Event::CertificateError {
                view: self.id,
                request,
                url: request_url.map(CefString::to_string).unwrap_or_default(),
                error: certificate_error_text(code),
            });
            1
        }

        fn resource_request_handler(
            &self,
            _browser: Option<&mut Browser>,
            _frame: Option<&mut Frame>,
            _request: Option<&mut Request>,
            _is_navigation: i32,
            _is_download: i32,
            _request_initiator: Option<&CefString>,
            _disable_default_handling: Option<&mut i32>,
        ) -> Option<ResourceRequestHandler> {
            Some(PageResourceRequest::new(self.id))
        }
    }
}

wrap_resource_request_handler! {
    struct PageResourceRequest { id: ViewId }

    impl ResourceRequestHandler {
        fn on_before_resource_load(
            &self,
            _browser: Option<&mut Browser>,
            frame: Option<&mut Frame>,
            request: Option<&mut Request>,
            _callback: Option<&mut Callback>,
        ) -> ReturnValue {
            let Some(req) = request else {
                return ReturnValue::CONTINUE;
            };
            let url_str = CefString::from(&req.url()).to_string();
            if url_str.is_empty() || url_str.starts_with("odyssey-ui://") || url_str.starts_with("data:") {
                return ReturnValue::CONTINUE;
            }

            let source_url = frame
                .map(|f| CefString::from(&f.url()).to_string())
                .unwrap_or_default();

            let res_type = match req.resource_type() {
                ResourceType::MAIN_FRAME => "main_frame",
                ResourceType::SUB_FRAME => "subdocument",
                ResourceType::STYLESHEET => "stylesheet",
                ResourceType::SCRIPT => "script",
                ResourceType::IMAGE => "image",
                ResourceType::FONT_RESOURCE => "font",
                ResourceType::SUB_RESOURCE => "sub_resource",
                ResourceType::OBJECT => "object",
                ResourceType::MEDIA => "media",
                ResourceType::XHR => "xmlhttprequest",
                ResourceType::PING => "ping",
                ResourceType::CSP_REPORT => "csp_report",
                _ => "other",
            };
            let method = CefString::from(&req.method()).to_string();

            let shields = STATE.with(|s| s.borrow().shields.clone());
            if let Some(sh) = shields {
                let should_block = sh.check_network_request(
                    &url_str,
                    &source_url,
                    res_type,
                    &method,
                    self.id,
                );
                if should_block {
                    let count = sh.blocked_count_for_tab(self.id);
                    emit(Event::ShieldsCountChanged {
                        view: self.id,
                        count,
                    });
                    return ReturnValue::CANCEL;
                }
            }

            ReturnValue::CONTINUE
        }
    }
}

#[cfg(windows)]
type OsEvent = sys::MSG;
#[cfg(not(windows))]
type OsEvent = sys::XEvent;

wrap_keyboard_handler! {
    struct PageKeyboard { id: ViewId }

    impl KeyboardHandler {
        fn on_pre_key_event(
            &self,
            _browser: Option<&mut Browser>,
            event: Option<&KeyEvent>,
            _os_event: Option<&mut OsEvent>,
            is_keyboard_shortcut: Option<&mut ::std::os::raw::c_int>,
        ) -> ::std::os::raw::c_int {
            let Some(ev) = event else { return 0 };
            if ev.type_ != KeyEventType::RAWKEYDOWN && ev.type_ != KeyEventType::KEYDOWN {
                return 0;
            }

            let modifiers = ev.modifiers;
            // SHIFT = 2, CONTROL = 4, ALT = 8, COMMAND = 128
            let is_ctrl = (modifiers & 4) != 0 || (modifiers & 128) != 0;
            let is_shift = (modifiers & 2) != 0;
            let is_alt = (modifiers & 8) != 0;
            let key = ev.windows_key_code;

            if is_ctrl && !is_alt {
                if key == 0x54 {
                    // Ctrl+T / Ctrl+Shift+T
                    if is_shift {
                        emit(Event::ShortcutRelightClosedTab);
                    } else {
                        emit(Event::ShortcutNewTab);
                    }
                    if let Some(s) = is_keyboard_shortcut { *s = 1; }
                    return 1;
                } else if key == 0x4E && is_shift {
                    // Ctrl+Shift+N
                    emit(Event::ShortcutNewPrivateWindow);
                    if let Some(s) = is_keyboard_shortcut { *s = 1; }
                    return 1;
                } else if key == 0x57 || key == 0x73 {
                    // Ctrl+W / Ctrl+F4
                    emit(Event::ShortcutCloseTab { view: self.id });
                    if let Some(s) = is_keyboard_shortcut { *s = 1; }
                    return 1;
                } else if key == 0x4C || key == 0x75 {
                    // Ctrl+L / F6
                    emit(Event::ShortcutFocusAddress);
                    if let Some(s) = is_keyboard_shortcut { *s = 1; }
                    return 1;
                } else if key == 0x52 {
                    // Ctrl+R
                    emit(Event::ShortcutReload { view: self.id });
                    if let Some(s) = is_keyboard_shortcut { *s = 1; }
                    return 1;
                } else if key == 0x48 {
                    // Ctrl+H
                    emit(Event::ShortcutOpenHistory);
                    if let Some(s) = is_keyboard_shortcut { *s = 1; }
                    return 1;
                } else if key == 188 {
                    // Ctrl+,
                    emit(Event::ShortcutOpenSettings);
                    if let Some(s) = is_keyboard_shortcut { *s = 1; }
                    return 1;
                } else if key == 0x09 {
                    // Ctrl+Tab / Ctrl+Shift+Tab
                    if is_shift {
                        emit(Event::ShortcutPrevTab);
                    } else {
                        emit(Event::ShortcutNextTab);
                    }
                    if let Some(s) = is_keyboard_shortcut { *s = 1; }
                    return 1;
                } else if key == 0x46 {
                    // Ctrl+F
                    emit(Event::ShortcutFindInPage { view: self.id });
                    if let Some(s) = is_keyboard_shortcut { *s = 1; }
                    return 1;
                } else if key == 187 || key == 0x6B {
                    // Ctrl+= / Ctrl++
                    emit(Event::ShortcutZoomIn { view: self.id });
                    if let Some(s) = is_keyboard_shortcut { *s = 1; }
                    return 1;
                } else if key == 189 || key == 0x6D {
                    // Ctrl+-
                    emit(Event::ShortcutZoomOut { view: self.id });
                    if let Some(s) = is_keyboard_shortcut { *s = 1; }
                    return 1;
                } else if key == 0x30 || key == 0x60 {
                    // Ctrl+0
                    emit(Event::ShortcutZoomReset { view: self.id });
                    if let Some(s) = is_keyboard_shortcut { *s = 1; }
                    return 1;
                } else if key == 0x50 {
                    // Ctrl+P
                    emit(Event::ShortcutPrint { view: self.id });
                    if let Some(s) = is_keyboard_shortcut { *s = 1; }
                    return 1;
                }
            } else if is_alt && !is_ctrl {
                if key == 0x73 {
                    // Alt+F4 closes the whole browser, not just this page's window
                    emit(Event::ShortcutQuit);
                    if let Some(s) = is_keyboard_shortcut { *s = 1; }
                    return 1;
                } else if key == 0x25 {
                    // Alt+Left
                    emit(Event::ShortcutBack { view: self.id });
                    if let Some(s) = is_keyboard_shortcut { *s = 1; }
                    return 1;
                } else if key == 0x27 {
                    // Alt+Right
                    emit(Event::ShortcutForward { view: self.id });
                    if let Some(s) = is_keyboard_shortcut { *s = 1; }
                    return 1;
                }
            } else if !is_ctrl && !is_alt {
                if key == 0x75 {
                    // F6 focuses the address bar without Ctrl too
                    emit(Event::ShortcutFocusAddress);
                    if let Some(s) = is_keyboard_shortcut { *s = 1; }
                    return 1;
                } else if key == 0x74 {
                    // F5
                    emit(Event::ShortcutReload { view: self.id });
                    if let Some(s) = is_keyboard_shortcut { *s = 1; }
                    return 1;
                } else if key == 0x7B {
                    // F12
                    emit(Event::ShortcutDevTools { view: self.id });
                    if let Some(s) = is_keyboard_shortcut { *s = 1; }
                    return 1;
                }
            }
            0
        }
    }
}

wrap_client! {
    struct PageClient { id: ViewId }

    impl Client {
        fn keyboard_handler(&self) -> Option<KeyboardHandler> {
            Some(PageKeyboard::new(self.id))
        }
        fn life_span_handler(&self) -> Option<LifeSpanHandler> {
            Some(PageLife::new(self.id))
        }
        fn display_handler(&self) -> Option<DisplayHandler> {
            Some(PageDisplay::new(self.id))
        }
        fn load_handler(&self) -> Option<LoadHandler> {
            Some(PageLoad::new(self.id))
        }
        fn download_handler(&self) -> Option<DownloadHandler> {
            Some(PageDownload::new(self.id))
        }
        fn request_handler(&self) -> Option<RequestHandler> {
            Some(PageRequest::new(self.id))
        }
        fn permission_handler(&self) -> Option<PermissionHandler> {
            Some(PagePermission::new(self.id))
        }
    }
}

// ---- the Engine ---------------------------------------------------------------------------

/// Handle to the CEF engine. Cheap to copy; all state lives on the UI thread.
#[derive(Clone, Copy, Debug)]
pub struct CefEngine;

impl CefEngine {
    /// Where events go. Replaces any earlier sink.
    pub fn set_event_sink(&self, sink: impl FnMut(Event) + 'static) {
        SINK.with(|s| *s.borrow_mut() = Some(Box::new(sink)));
    }

    /// Where messages from our UI go (one JSON string each, sent by `odysseyBridge(json)`).
    pub fn set_ui_message_sink(&self, sink: impl FnMut(String) + 'static) {
        UI_SINK.with(|s| *s.borrow_mut() = Some(Box::new(sink)));
    }

    /// Send one JSON message to our UI (`window.odysseyReceive(message)`).
    pub fn send_to_ui(&self, json: &str) {
        // U+2028/2029 are valid in JSON but end a line in older JS parsers: escape them.
        let json = json
            .replace('\u{2028}', "\\u2028")
            .replace('\u{2029}', "\\u2029");
        let script = format!("window.odysseyReceive && window.odysseyReceive({json});");
        CefEngine.run_ui_script(&script);
    }

    /// The main window's client area size in DIP (0×0 before it exists).
    pub fn client_size(&self) -> (i32, i32) {
        STATE.with(|s| {
            s.borrow()
                .main
                .as_ref()
                .map(|m| {
                    let c = m.client_area_bounds_in_screen();
                    (c.width, c.height)
                })
                .unwrap_or((0, 0))
        })
    }

    /// Tell the engine where our UI is drawing over the page area, so those rectangles are cut
    /// out of the page windows (AUDIT-06). Rectangles are main-window DIP plus a corner radius.
    pub fn set_overlay_rects(&self, rects: Vec<(ViewRect, i32)>) {
        let changed = STATE.with(|s| {
            let mut s = s.borrow_mut();
            let same = s.overlay_holes.len() == rects.len()
                && s.overlay_holes.iter().zip(&rects).all(|(a, b)| {
                    a.1 == b.1
                        && a.0.x == b.0.x
                        && a.0.y == b.0.y
                        && a.0.width == b.0.width
                        && a.0.height == b.0.height
                });
            if !same {
                s.overlay_holes = rects;
            }
            !same
        });
        if changed {
            // Only the hole shapes change: don't re-place the page windows for this.
            #[cfg(windows)]
            refresh_holes();
        }
        #[cfg(not(windows))]
        let _ = changed;
    }

    /// Fill the login form the user clicked in this page (autofill). The site was already checked
    /// by the caller against the browser's own record of the tab's URL.
    pub fn fill_login(&self, view: ViewId, username: &str, password: &str) {
        let script = fill_login_script(username, password);
        with_browser(view, |b| {
            if let Some(f) = b.main_frame() {
                f.execute_java_script(Some(&CefString::from(script.as_str())), None, 0);
            }
        });
    }

    /// Slide the notch away (or back): while hidden only a thin hover strip is cut out of the
    /// page windows, so the whole page is usable underneath.
    pub fn set_notch_hidden(&self, hidden: bool) {
        let changed = STATE.with(|s| {
            let mut s = s.borrow_mut();
            let changed = s.notch_hidden != hidden;
            s.notch_hidden = hidden;
            changed
        });
        if changed {
            #[cfg(windows)]
            refresh_holes();
        }
    }

    /// The main window's native handle (0 when not known), for dialogs shown over it.
    pub fn main_window_handle(&self) -> isize {
        #[cfg(windows)]
        {
            STATE.with(|s| {
                s.borrow()
                    .main
                    .as_ref()
                    .map(|m| m.window_handle().0 as isize)
                    .unwrap_or(0)
            })
        }
        #[cfg(not(windows))]
        {
            0
        }
    }

    /// Move keyboard focus from a page window to our UI (Ctrl+L, F6). Pages are separate
    /// top-level windows, so the UI's own `focus()` can't take OS focus by itself (AUDIT-08).
    pub fn focus_ui(&self) {
        let (main, ui) = STATE.with(|s| {
            let st = s.borrow();
            (st.main.clone(), st.ui.clone())
        });
        if let Some(main) = main {
            main.activate();
        }
        if let Some(ui) = ui {
            ui.request_focus();
        }
    }

    /// Close the main window (and with it every page), ending [`run`].
    pub fn quit(&self) {
        if let Some(main) = STATE.with(|s| s.borrow().main.clone()) {
            main.close();
        }
    }

    /// Run `f` on the UI thread after `delay_ms` (timers for the shell and tests).
    pub fn after(&self, delay_ms: i64, f: impl FnOnce() + 'static) {
        let mut task = Later::new(std::rc::Rc::new(RefCell::new(Some(Box::new(f)))));
        post_delayed_task(ThreadId::UI, Some(&mut task), delay_ms);
    }

    /// Post a closure from any thread to execute on the UI thread.
    pub fn post_ui(&self, f: impl FnOnce() + Send + 'static) {
        let mut task = SendTask::new(std::sync::Arc::new(std::sync::Mutex::new(Some(Box::new(
            f,
        )))));
        post_task(ThreadId::UI, Some(&mut task));
    }
}

type Job = Box<dyn FnOnce()>;

wrap_task! {
    struct Later { job: std::rc::Rc<RefCell<Option<Job>>> }

    impl Task {
        fn execute(&self) {
            if let Some(job) = self.job.borrow_mut().take() {
                job();
            }
        }
    }
}

type SendJob = std::sync::Arc<std::sync::Mutex<Option<Box<dyn FnOnce() + Send>>>>;

wrap_task! {
    struct SendTask { job: SendJob }

    impl Task {
        fn execute(&self) {
            if let Ok(mut lock) = self.job.lock()
                && let Some(job) = lock.take()
            {
                job();
            }
        }
    }
}

impl Engine for CefEngine {
    fn create_view(&mut self, url: &str) -> ViewId {
        let id = STATE.with(|s| {
            let mut s = s.borrow_mut();
            s.next_id += 1;
            let id = s.next_id;
            s.pages.insert(
                id,
                Page {
                    window: None,
                    browser: None,
                    bounds: ViewRect::default(),
                    visible: false,
                    sleeping: false,
                    saved_url: None,
                    scroll_y: 0.0,
                    restore_scroll: None,
                },
            );
            id
        });
        let mut client = PageClient::new(id);
        let mut delegate = PageView::new();
        let view = browser_view_create(
            Some(&mut client),
            Some(&CefString::from(url)),
            Some(&BrowserSettings {
                background_color: 0xFFFFFFFF,
                ..Default::default()
            }),
            None,
            None,
            Some(&mut delegate),
        );
        let mut window = PageWindow::new(id, RefCell::new(view));
        window_create_top_level(Some(&mut window));
        id
    }

    fn close_view(&mut self, view: ViewId) {
        let window = STATE.with(|s| s.borrow().pages.get(&view).and_then(|p| p.window.clone()));
        if let Some(w) = window {
            w.close();
        }
    }

    fn set_bounds(&mut self, view: ViewId, bounds: ViewRect) {
        STATE.with(|s| {
            if let Some(p) = s.borrow_mut().pages.get_mut(&view) {
                p.bounds = bounds;
            }
        });
        place(view);
    }

    fn set_visible(&mut self, view: ViewId, visible: bool) {
        if visible {
            self.wake(view);
        }
        STATE.with(|s| {
            if let Some(p) = s.borrow_mut().pages.get_mut(&view) {
                p.visible = visible;
            }
        });
        place(view);
    }

    fn navigate(&mut self, view: ViewId, url: &str) {
        with_browser(view, |b| {
            if let Some(f) = b.main_frame() {
                f.load_url(Some(&CefString::from(url)));
            }
        });
    }

    fn go_back(&mut self, view: ViewId) {
        with_browser(view, |b| b.go_back());
    }

    fn go_forward(&mut self, view: ViewId) {
        with_browser(view, |b| b.go_forward());
    }

    fn reload(&mut self, view: ViewId) {
        with_browser(view, |b| b.reload());
    }

    fn stop(&mut self, view: ViewId) {
        with_browser(view, |b| b.stop_load());
    }

    fn set_zoom(&mut self, view: ViewId, factor: f64) {
        with_browser(view, |b| {
            if let Some(host) = b.host() {
                let level = if factor > 0.0 {
                    factor.ln() / 1.2_f64.ln()
                } else {
                    0.0
                };
                host.set_zoom_level(level);
            }
        });
    }

    fn find(&mut self, view: ViewId, text: &str, forward: bool, match_case: bool, next: bool) {
        with_browser(view, |b| {
            if let Some(host) = b.host() {
                let s = CefString::from(text);
                host.find(Some(&s), forward as i32, match_case as i32, next as i32);
            }
        });
    }

    fn stop_finding(&mut self, view: ViewId, clear_selection: bool) {
        with_browser(view, |b| {
            if let Some(host) = b.host() {
                host.stop_finding(clear_selection as i32);
            }
        });
    }

    fn run_ui_script(&mut self, script: &str) {
        let frame = STATE.with(|s| {
            s.borrow()
                .ui
                .as_ref()
                .and_then(|v| v.browser())
                .and_then(|b| b.main_frame())
        });
        if let Some(f) = frame {
            f.execute_java_script(Some(&CefString::from(script)), None, 0);
        }
    }

    fn open_devtools(&mut self, view: ViewId) {
        with_browser(view, |b| {
            if let Some(host) = b.host() {
                if host.has_dev_tools() != 0 {
                    host.close_dev_tools();
                } else {
                    host.show_dev_tools(None, None, None, None);
                }
            }
        });
    }

    fn print(&mut self, view: ViewId) {
        with_browser(view, |b| {
            if let Some(host) = b.host() {
                host.print();
            }
        });
    }

    fn answer_permission(&mut self, request: u64, allow: bool) {
        let cb = STATE.with(|s| s.borrow_mut().perm_callbacks.remove(&request));
        if let Some(cb) = cb {
            let result = if allow {
                sys::cef_permission_request_result_t::CEF_PERMISSION_RESULT_ACCEPT
            } else {
                sys::cef_permission_request_result_t::CEF_PERMISSION_RESULT_DENY
            };
            cb.cont(result.into());
        }
    }

    fn answer_certificate_error(&mut self, request: u64, proceed: bool) {
        let cb = STATE.with(|s| s.borrow_mut().cert_callbacks.remove(&request));
        if let Some(cb) = cb {
            if proceed {
                cb.cont();
            } else {
                cb.cancel();
            }
        }
    }

    fn download_control(&mut self, id: u32, action: &str) {
        STATE.with(|s| {
            let guard = s.borrow();
            if let Some(cb) = guard.cef_downloads.get(&id) {
                match action {
                    "pause" => cb.pause(),
                    "resume" => cb.resume(),
                    "cancel" => cb.cancel(),
                    _ => {}
                }
            }
        });
    }

    fn minimize_window(&mut self) {
        STATE.with(|s| {
            if let Some(w) = s.borrow().main.as_ref() {
                w.minimize();
            }
        });
    }

    fn toggle_maximize_window(&mut self) {
        STATE.with(|s| {
            if let Some(w) = s.borrow().main.as_ref() {
                if w.is_maximized() != 0 {
                    w.restore();
                } else {
                    w.maximize();
                }
            }
        });
    }

    fn close_window(&mut self) {
        self.quit();
    }

    fn toggle_shields_exception(&mut self, host: &str) -> bool {
        STATE.with(|s| {
            s.borrow()
                .shields
                .as_ref()
                .map(|sh| sh.toggle_exception(host))
                .unwrap_or(true)
        })
    }

    fn is_shields_active_for(&self, host: &str) -> bool {
        STATE.with(|s| {
            s.borrow()
                .shields
                .as_ref()
                .map(|sh| !sh.is_host_exempt(host))
                .unwrap_or(true)
        })
    }

    fn shields_count(&self, view: ViewId) -> u64 {
        STATE.with(|s| {
            s.borrow()
                .shields
                .as_ref()
                .map(|sh| sh.blocked_count_for_tab(view))
                .unwrap_or(0)
        })
    }

    fn sleep(&mut self, view: ViewId) {
        // Don't call into CEF while STATE is borrowed: load_url fires callbacks that borrow it
        // again, which froze the whole app.
        let frame = STATE.with(|s| {
            let mut s = s.borrow_mut();
            let page = s.pages.get_mut(&view)?;
            if page.visible || page.sleeping {
                return None;
            }
            let frame = page.browser.as_ref().and_then(|b| b.main_frame());
            if let Some(frame) = &frame {
                let url = CefString::from(&frame.url()).to_string();
                if !url.is_empty() && url != "about:blank" {
                    page.saved_url = Some(url);
                }
            }
            page.sleeping = true;
            frame
        });
        if let Some(frame) = frame {
            frame.load_url(Some(&CefString::from("about:blank")));
        }
    }

    fn wake(&mut self, view: ViewId) {
        let saved_url = STATE.with(|s| {
            let mut s = s.borrow_mut();
            let page = s.pages.get_mut(&view)?;
            if page.sleeping {
                page.sleeping = false;
                page.restore_scroll = Some(page.scroll_y);
                page.saved_url.take()
            } else {
                None
            }
        });
        if let Some(url) = saved_url {
            with_browser(view, |b| {
                if let Some(f) = b.main_frame() {
                    f.load_url(Some(&CefString::from(url.as_str())));
                }
            });
        }
    }

    fn is_sleeping(&self, view: ViewId) -> bool {
        STATE.with(|s| {
            s.borrow()
                .pages
                .get(&view)
                .map(|p| p.sleeping)
                .unwrap_or(false)
        })
    }
}

// ---- start-up -----------------------------------------------------------------------------

wrap_browser_process_handler! {
    struct Process {}

    impl BrowserProcessHandler {
        fn on_before_child_process_launch(&self, command_line: Option<&mut CommandLine>) {
            let prefix = STATE.with(|s| s.borrow().ui_prefix.clone());
            if let Some(c) = command_line {
                c.append_switch_with_value(
                    Some(&CefString::from(UI_PREFIX_SWITCH)),
                    Some(&CefString::from(prefix.as_str())),
                );
            }
        }

        fn on_context_initialized(&self) {
            let (dir, suffix, title) = STATE.with(|s| {
                let s = s.borrow();
                let o = s.options.as_ref().expect("start options are set before CEF starts");
                (o.ui_dir.clone(), o.ui_suffix.clone(), o.title.clone())
            });
            let url = match dir {
                Some(root) => {
                    let mut factory = ui_scheme::UiSchemeFactory::new(root);
                    register_scheme_handler_factory(
                        Some(&CefString::from(ui_scheme::SCHEME)),
                        Some(&CefString::from(ui_scheme::HOST)),
                        Some(&mut factory),
                    );
                    format!("{}index.html{suffix}", ui_scheme::prefix())
                }
                // Never a blank screen without a word (EDGE_CASES §1).
                None => "data:text/html;charset=utf-8,<body style='background:#000;color:#aeb7bf;font:15px system-ui;padding:40px'>The browser's interface files are missing. Please reinstall.</body>".to_string(),
            };
            let mut client = UiClient::new();
            let mut delegate = UiView::new();
            let ui = browser_view_create(
                Some(&mut client),
                Some(&CefString::from(url.as_str())),
                Some(&BrowserSettings { background_color: 0xFF000000, ..Default::default() }),
                None,
                None,
                Some(&mut delegate),
            );
            let mut window = MainWindow::new(RefCell::new(ui), title);
            window_create_top_level(Some(&mut window));
        }
    }
}

wrap_app! {
    struct EngineApp;

    impl App {
        fn on_before_command_line_processing(
            &self,
            process_type: Option<&CefString>,
            command_line: Option<&mut CommandLine>,
        ) {
            let is_browser = process_type.map(CefString::to_string).unwrap_or_default().is_empty();
            if let Some(cmd) = command_line {
                if is_browser
                    && let Some(paths) = EXTENSION_PATHS.get().filter(|p| !p.is_empty())
                {
                    cmd.append_switch_with_value(
                        Some(&CefString::from("load-extension")),
                        Some(&CefString::from(paths.join(",").as_str())),
                    );
                }
                // P5-03: Process and cache tuning (documented Chromium flags)
                cmd.append_switch_with_value(
                    Some(&CefString::from("disk-cache-size")),
                    Some(&CefString::from("268435456")),
                );
                cmd.append_switch_with_value(
                    Some(&CefString::from("enable-features")),
                    Some(&CefString::from("IntensiveWakeUpThrottling,WebContentsDiscard")),
                );
                // AUDIT-01: page views are separate top-level windows that cover our UI window.
                // Chromium's native occlusion tracking would treat the UI as hidden and stop
                // painting it, leaving the Helm and notch blank. Keep it painting.
                // Merge into an existing `disable-features` instead of adding a second one.
                let name = CefString::from("disable-features");
                let existing = if cmd.has_switch(Some(&name)) != 0 {
                    CefString::from(&cmd.switch_value(Some(&name))).to_string()
                } else {
                    String::new()
                };
                let merged = if existing.is_empty() {
                    "CalculateNativeWinOcclusion".to_string()
                } else {
                    format!("{existing},CalculateNativeWinOcclusion")
                };
                cmd.append_switch_with_value(
                    Some(&name),
                    Some(&CefString::from(merged.as_str())),
                );
                // AUDIT-01 (found with a real screen capture, 21 Nov 2026): with DirectComposition
                // on, Windows shows the UI window's content only where no page window sits on
                // top, ignoring the holes cut into the page windows, so the Helm and notch were
                // empty grey shapes. Without it the holes show the UI.
                // CONSTELLATIONS_DIRECT_COMPOSITION=1 turns the fix off (for comparing CPU use).
                if std::env::var_os("CONSTELLATIONS_DIRECT_COMPOSITION").is_none() {
                    cmd.append_switch(Some(&CefString::from("disable-direct-composition")));
                }
                cmd.append_switch(Some(&CefString::from("disable-backgrounding-occluded-windows")));
                cmd.append_switch(Some(&CefString::from("disable-renderer-backgrounding")));
                cmd.append_switch_with_value(
                    Some(&CefString::from("js-flags")),
                    Some(&CefString::from("--max_old_space_size=512")),
                );
            }
        }
        fn browser_process_handler(&self) -> Option<BrowserProcessHandler> {
            Some(Process::new())
        }
        fn render_process_handler(&self) -> Option<RenderProcessHandler> {
            Some(BridgeRenderer::new())
        }
        // Runs in every process (browser and helpers).
        fn on_register_custom_schemes(&self, registrar: Option<&mut SchemeRegistrar>) {
            if let Some(r) = registrar {
                r.add_custom_scheme(Some(&CefString::from(ui_scheme::SCHEME)), ui_scheme::options());
            }
        }
    }
}

/// Run the engine. In a helper process this returns [`Role::Helper`] at once. In the main
/// process it opens the window, calls `on_ready` once the main window exists, and returns when
/// the user closes it.
pub fn run(
    options: impl FnOnce() -> StartOptions,
    on_ready: impl FnOnce(CefEngine) + 'static,
) -> Result<Role, StartError> {
    let _ = api_hash(sys::CEF_API_VERSION_LAST, 0);
    let args = args::Args::new();
    let cmd_line = args.as_cmd_line().ok_or(StartError::BadCommandLine)?;

    // CEF starts helper processes with the same executable and a `--type=` switch.
    let is_browser = cmd_line.has_switch(Some(&CefString::from("type"))) != 1;
    // Helper processes need the app too: the renderer side of the UI bridge lives in it.
    let mut helper_app = EngineApp::new();
    let code = execute_process(
        Some(args.as_main_args()),
        Some(&mut helper_app),
        std::ptr::null_mut(),
    );
    if !is_browser {
        return Ok(Role::Helper(code.max(0)));
    }

    let options = options();
    let _ = EXTENSION_PATHS.set(options.extension_paths.clone());
    if !options.in_memory && !options.profile_dir.as_os_str().is_empty() {
        let _ = std::fs::create_dir_all(&options.profile_dir);
    }
    let settings = Settings {
        // Sandbox: off until the bootstrap layout is set up (D-007/D-013, P9-01).
        no_sandbox: 1,
        root_cache_path: if options.in_memory || options.profile_dir.as_os_str().is_empty() {
            CefString::default()
        } else {
            CefString::from(options.profile_dir.to_string_lossy().as_ref())
        },
        ..Default::default()
    };
    let shields_dir = if options.profile_dir.as_os_str().is_empty() {
        std::env::temp_dir().join("constellations-shields")
    } else {
        options.profile_dir.join("shields")
    };
    let shields = Arc::new(shields::Shields::new(shields_dir));
    shields.clone().update_filters_background();

    STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.ui_prefix = ui_scheme::prefix();
        s.shields = Some(shields);
        s.options = Some(options);
    });
    ON_READY.with(|r| *r.borrow_mut() = Some(Box::new(on_ready)));

    let mut app = EngineApp::new();
    if initialize(
        Some(args.as_main_args()),
        Some(&settings),
        Some(&mut app),
        std::ptr::null_mut(),
    ) != 1
    {
        return Err(StartError::InitializeFailed);
    }
    run_message_loop();
    shutdown();
    Ok(Role::BrowserFinished)
}
