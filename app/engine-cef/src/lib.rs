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
use engine_api::{Engine, Event, NewWindowKind, Rect as ViewRect, ViewId};
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;

mod ui_scheme;

/// What the shell asks the engine to start with.
pub struct StartOptions {
    /// Folder for the profile (cache, cookies, settings written by the engine).
    pub profile_dir: PathBuf,
    /// Folder with our UI (`ui/`). `None` when it's missing: the window then says so.
    pub ui_dir: Option<PathBuf>,
    /// Appended to the UI URL, e.g. `#ipc-selftest` or `?fixture=Main`.
    pub ui_suffix: String,
    /// Window title (the product name; one place, DESIGN.md §11).
    pub title: String,
}

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
}

#[derive(Default)]
struct State {
    options: Option<StartOptions>,
    main: Option<Window>,
    ui: Option<BrowserView>,
    ui_closed: bool,
    pages: HashMap<ViewId, Page>,
    next_id: ViewId,
    /// URL prefix of our UI folder; only frames under it get the bridge (P1-03).
    ui_prefix: String,
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
        win.set_bounds(Some(&Rect {
            x: c.x + b.x,
            y: c.y + b.y,
            width: b.width.max(1),
            height: b.height.max(1),
        }));
        if page.visible {
            win.show();
        } else {
            win.hide();
        }
    });
}

fn place_all() {
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
            if let Some(ui) = self.ui.borrow().as_ref() {
                window.add_child_view(Some(&mut View::from(ui)));
            }
            STATE.with(|s| {
                let mut s = s.borrow_mut();
                s.main = Some(window.clone());
                s.ui = self.ui.borrow().clone();
            });
            window.show();
            if let Some(ready) = ON_READY.with(|r| r.borrow_mut().take()) {
                ready(CefEngine);
            }
        }

        fn on_window_bounds_changed(&self, _window: Option<&mut Window>, _bounds: Option<&Rect>) {
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
            if let Some(v) = self.view.borrow().as_ref() {
                window.add_child_view(Some(&mut View::from(v)));
            }
            STATE.with(|s| {
                if let Some(p) = s.borrow_mut().pages.get_mut(&self.id) {
                    p.window = Some(window.clone());
                }
            });
            place(self.id);
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
    }
}

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

wrap_client! {
    struct PageClient { id: ViewId }

    impl Client {
        fn life_span_handler(&self) -> Option<LifeSpanHandler> {
            Some(PageLife::new(self.id))
        }
        fn display_handler(&self) -> Option<DisplayHandler> {
            Some(PageDisplay::new(self.id))
        }
        fn load_handler(&self) -> Option<LoadHandler> {
            Some(PageLoad::new(self.id))
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

    fn set_zoom(&mut self, _view: ViewId, _factor: f64) {
        todo!("P4-06 page basics: zoom")
    }

    fn find(&mut self, _view: ViewId, _text: &str, _forward: bool, _match_case: bool, _next: bool) {
        todo!("P4-06 page basics: find in page")
    }

    fn stop_finding(&mut self, _view: ViewId, _clear_selection: bool) {
        todo!("P4-06 page basics: find in page")
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

    fn sleep(&mut self, _view: ViewId) {
        todo!("P5-02 tab sleeping")
    }

    fn wake(&mut self, _view: ViewId) {
        todo!("P5-02 tab sleeping")
    }

    fn open_devtools(&mut self, _view: ViewId) {
        todo!("P4-06 page basics: devtools")
    }

    fn answer_permission(&mut self, _request: u64, _allow: bool) {
        todo!("P4-06 page basics: permission prompts")
    }

    fn answer_certificate_error(&mut self, _request: u64, _proceed: bool) {
        todo!("P4-06 page basics: certificate error page")
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
    let _ = std::fs::create_dir_all(&options.profile_dir);
    let settings = Settings {
        // Sandbox: off until the bootstrap layout is set up (D-007/D-013, P9-01).
        no_sandbox: 1,
        root_cache_path: CefString::from(options.profile_dir.to_string_lossy().as_ref()),
        ..Default::default()
    };
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.ui_prefix = ui_scheme::prefix();
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
