//! CEF behind `engine-api`. Everything that touches CEF lives in this crate.
//!
//! Facts from Phase 0 that shape this code (see DECISIONS D-003, D-009):
//! - One Chrome style BrowserView per window, and it must be added before any other BrowserView.
//! - Our own UI runs in Alloy style BrowserViews (no extensions needed there).
//! - Windows needs the app manifest (`app/shell/app.manifest`) or the GPU process crashes.

use cef::*;
use std::cell::RefCell;
use std::path::PathBuf;

/// What the shell asks the engine to start with.
pub struct StartOptions {
    /// Folder for the profile (cache, cookies, settings written by the engine).
    pub profile_dir: PathBuf,
    /// URL of our UI page (`ui/index.html`).
    pub ui_url: String,
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
            StartError::InitializeFailed => {
                write!(
                    f,
                    "The browser engine could not start. Another copy may already be open."
                )
            }
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

thread_local! {
    static OPTIONS: RefCell<Option<StartOptions>> = const { RefCell::new(None) };
}

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
            window.show();
        }

        fn on_window_destroyed(&self, _window: Option<&mut Window>) {
            *self.ui.borrow_mut() = None;
        }

        fn can_close(&self, _window: Option<&mut Window>) -> i32 {
            let ui = self.ui.borrow();
            match ui.as_ref().and_then(|v| v.browser()).and_then(|b| b.host()) {
                Some(host) => host.try_close_browser(),
                None => 1,
            }
        }

        fn window_runtime_style(&self) -> RuntimeStyle {
            // Chrome style, so the page view (Chrome style) can live here later (P1-02).
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
            // One window for now: closing it ends the app.
            quit_message_loop();
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

wrap_client! {
    struct UiClient {}

    impl Client {
        fn life_span_handler(&self) -> Option<LifeSpanHandler> {
            Some(UiLife::new())
        }
        fn load_handler(&self) -> Option<LoadHandler> {
            Some(UiLoad::new())
        }
    }
}

wrap_browser_process_handler! {
    struct Process {}

    impl BrowserProcessHandler {
        fn on_context_initialized(&self) {
            let (url, title) = OPTIONS.with(|o| {
                let o = o.borrow();
                let o = o.as_ref().expect("start options are set before CEF starts");
                (o.ui_url.clone(), o.title.clone())
            });
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
    }
}

/// Run the engine. In a helper process this returns [`Role::Helper`] at once; in the main
/// process it opens the window and returns when the user closes it.
pub fn run(options: impl FnOnce() -> StartOptions) -> Result<Role, StartError> {
    let _ = api_hash(sys::CEF_API_VERSION_LAST, 0);
    let args = args::Args::new();
    let cmd_line = args.as_cmd_line().ok_or(StartError::BadCommandLine)?;

    // CEF starts helper processes with the same executable and a `--type=` switch.
    let is_browser = cmd_line.has_switch(Some(&CefString::from("type"))) != 1;
    let code = execute_process(Some(args.as_main_args()), None, std::ptr::null_mut());
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
    OPTIONS.with(|o| *o.borrow_mut() = Some(options));

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
