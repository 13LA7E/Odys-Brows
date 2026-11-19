//! P0-05 spike: the P0-02 window plus `--toolbar=none|location|normal` (Chrome toolbar type) to
//! test Chrome's own UI and MV3 extensions. Chrome switches such as `--load-extension` pass through.

use cef::*;
use std::cell::RefCell;

wrap_window_delegate! {
    struct SpikeWindowDelegate {
        browser_view: RefCell<Option<BrowserView>>,
    }

    impl ViewDelegate {
        fn preferred_size(&self, _view: Option<&mut View>) -> Size {
            Size { width: 1280, height: 800 }
        }
        fn minimum_size(&self, _view: Option<&mut View>) -> Size {
            Size { width: 800, height: 600 }
        }
    }

    impl PanelDelegate {}

    impl WindowDelegate {
        fn on_window_created(&self, window: Option<&mut Window>) {
            let browser_view = self.browser_view.borrow();
            let (Some(window), Some(browser_view)) = (window, browser_view.as_ref()) else {
                return;
            };
            let mut view = View::from(browser_view);
            window.add_child_view(Some(&mut view));
            window.show();
        }

        fn on_window_destroyed(&self, _window: Option<&mut Window>) {
            *self.browser_view.borrow_mut() = None;
        }

        fn can_close(&self, _window: Option<&mut Window>) -> i32 {
            let browser_view = self.browser_view.borrow();
            match browser_view.as_ref().and_then(|v| v.browser()) {
                Some(browser) => browser.host().map(|h| h.try_close_browser()).unwrap_or(1),
                None => 1,
            }
        }

        fn window_runtime_style(&self) -> RuntimeStyle {
            RuntimeStyle::CHROME
        }
    }
}

wrap_browser_view_delegate! {
    struct SpikeBrowserViewDelegate {}

    impl ViewDelegate {}

    impl BrowserViewDelegate {
        fn browser_runtime_style(&self) -> RuntimeStyle {
            RuntimeStyle::CHROME
        }
        fn chrome_toolbar_type(&self, _bv: Option<&mut BrowserView>) -> ChromeToolbarType {
            let cl = command_line_get_global().expect("global command line");
            match CefString::from(&cl.switch_value(Some(&CefString::from("toolbar")))).to_string().as_str() {
                "normal" => ChromeToolbarType::NORMAL,
                "location" => ChromeToolbarType::LOCATION,
                _ => ChromeToolbarType::NONE,
            }
        }
    }
}

wrap_life_span_handler! {
    struct SpikeLifeSpan {}

    impl LifeSpanHandler {
        fn on_before_close(&self, _browser: Option<&mut Browser>) {
            // One browser only: closing it ends the app.
            quit_message_loop();
        }
    }
}

wrap_task! {
    struct LoadLater {
        browser: Browser,
        url: String,
    }

    impl Task {
        fn execute(&self) {
            if let Some(f) = self.browser.main_frame() {
                f.load_url(Some(&CefString::from(self.url.as_str())));
            }
        }
    }
}

thread_local! { static THEN_DONE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) }; }

wrap_load_handler! {
    struct SpikeLoad {}

    impl LoadHandler {
        fn on_load_end(&self, browser: Option<&mut Browser>, frame: Option<&mut Frame>, status: i32) {
            let (Some(b), Some(f)) = (browser, frame) else { return };
            if f.is_main() == 0 {
                return;
            }
            eprintln!("load_end status={status} url={}", CefString::from(&f.url()));
            // `--then=<url>`: a browser-initiated navigation 4 s after the first page, so extensions
            // loaded with --load-extension are registered by then.
            let cl = command_line_get_global().expect("global command line");
            let then = CefString::from(&cl.switch_value(Some(&CefString::from("then")))).to_string();
            if !then.is_empty() && !THEN_DONE.with(|d| d.replace(true)) {
                let mut task = LoadLater::new(b.clone(), then);
                post_delayed_task(ThreadId::UI, Some(&mut task), 4000);
            }
        }
        fn on_load_error(
            &self,
            _browser: Option<&mut Browser>,
            _frame: Option<&mut Frame>,
            error_code: Errorcode,
            error_text: Option<&CefString>,
            failed_url: Option<&CefString>,
        ) {
            // Chrome style shows its own error page; we log for debugging (EDGE §1).
            eprintln!(
                "load error {:?} {} for {}",
                sys::cef_errorcode_t::from(error_code),
                error_text.map(CefString::to_string).unwrap_or_default(),
                failed_url.map(CefString::to_string).unwrap_or_default()
            );
        }
    }
}

wrap_client! {
    struct SpikeClient {}

    impl Client {
        fn life_span_handler(&self) -> Option<LifeSpanHandler> {
            Some(SpikeLifeSpan::new())
        }
        fn load_handler(&self) -> Option<LoadHandler> {
            Some(SpikeLoad::new())
        }
    }
}

wrap_browser_process_handler! {
    struct SpikeBrowserProcessHandler {}

    impl BrowserProcessHandler {
        fn on_context_initialized(&self) {
            let command_line = command_line_get_global().expect("global command line");
            let url = CefString::from(&command_line.switch_value(Some(&CefString::from("url"))))
                .to_string();
            let url = if url.is_empty() { "https://example.com/".to_string() } else { url };
            let url = CefString::from(url.as_str());

            let mut client = SpikeClient::new();
            let mut bv_delegate = SpikeBrowserViewDelegate::new();
            let browser_view = browser_view_create(
                Some(&mut client),
                Some(&url),
                Some(&BrowserSettings::default()),
                None,
                None,
                Some(&mut bv_delegate),
            );
            let mut win_delegate = SpikeWindowDelegate::new(RefCell::new(browser_view));
            window_create_top_level(Some(&mut win_delegate));
        }
    }
}

wrap_app! {
    struct SpikeApp;

    impl App {
        fn browser_process_handler(&self) -> Option<BrowserProcessHandler> {
            Some(SpikeBrowserProcessHandler::new())
        }
    }
}

fn main() {
    let _ = api_hash(sys::CEF_API_VERSION_LAST, 0);

    let args = args::Args::new();
    let Some(cmd_line) = args.as_cmd_line() else {
        eprintln!("could not parse the command line");
        std::process::exit(1);
    };

    // Sub-processes (renderer, GPU, ...) run the same exe and return here.
    let is_browser = cmd_line.has_switch(Some(&CefString::from("type"))) != 1;
    let code = execute_process(Some(args.as_main_args()), None, std::ptr::null_mut());
    if !is_browser {
        std::process::exit(code.max(0));
    }

    // Profile under the user's local data dir; paths with spaces/non-ASCII are passed as CefString (UTF-16).
    let root = std::env::var_os("LOCALAPPDATA")
        .or_else(|| std::env::var_os("XDG_CACHE_HOME"))
        .map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("constellations-spike")
        .join("p0-05");
    let _ = std::fs::create_dir_all(&root);
    let root = CefString::from(root.to_string_lossy().as_ref());

    let mut app = SpikeApp::new();
    let settings = Settings {
        no_sandbox: 1,
        root_cache_path: root,
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
