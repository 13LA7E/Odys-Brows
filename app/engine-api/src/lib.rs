//! The engine interface (D-005). All browser code talks to the web engine through this crate,
//! never to CEF directly. Keep it small: only what the browser uses.
//!
//! Model: the engine owns *page views* (one per tab). Each has bounds in the main window's
//! client area (DIP) and can be shown or hidden. Our own UI is not a page view; the engine hosts
//! it separately and only it can receive scripts from Rust ([`Engine::run_ui_script`]).
//! Web pages never get a bridge to Rust (AGENTS.md, "Web content never gets IPC").

use std::path::{Path, PathBuf};

/// Identifies one page view (one tab). Never reused within a run.
pub type ViewId = u64;

/// A rectangle in device-independent pixels, relative to the main window's client area.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// How a page asked for a new window (from `window.open`, a link with a target, or the user).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NewWindowKind {
    /// A new foreground tab (the default for links with `target=_blank`).
    ForegroundTab,
    /// A new background tab (middle click, Ctrl+click).
    BackgroundTab,
    /// A popup with its own size (`window.open` with features).
    Popup,
    /// A new top-level window (Shift+click).
    Window,
}

/// What a page asks permission for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PermissionKind {
    Camera,
    Microphone,
    Location,
    Notifications,
    Clipboard,
    Other,
}

/// A download's state, reported as it changes.
#[derive(Clone, Debug, PartialEq)]
pub struct DownloadInfo {
    /// Engine id of the download, unique within a run.
    pub id: u32,
    pub view: Option<ViewId>,
    pub url: String,
    /// Suggested file name (from Content-Disposition or the URL).
    pub file_name: String,
    pub received_bytes: i64,
    /// `None` when the server didn't say how big the file is.
    pub total_bytes: Option<i64>,
    pub bytes_per_second: i64,
    pub complete: bool,
    pub cancelled: bool,
    pub interrupted: bool,
    /// Where the file is (or will be) saved.
    pub path: String,
}

/// Something that happened in a page view. Delivered on the UI thread, in order.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// The main frame's URL changed (navigation, history, fragment).
    UrlChanged { view: ViewId, url: String },
    /// The page title changed. Empty when the page has none (show the host instead).
    TitleChanged { view: ViewId, title: String },
    /// The page's favicon URL changed (`None`: no favicon).
    FaviconChanged { view: ViewId, url: Option<String> },
    /// Loading started or stopped; also whether back/forward are possible now.
    LoadingChanged {
        view: ViewId,
        loading: bool,
        can_go_back: bool,
        can_go_forward: bool,
    },
    /// A page failed to load (DNS, TLS, offline, ...). `code` is the engine's error code.
    LoadFailed {
        view: ViewId,
        url: String,
        code: i32,
        text: String,
    },
    /// The page started or stopped playing audio.
    AudioChanged { view: ViewId, playing: bool },
    /// A login form was submitted (the page's own script reported what was typed). The site is
    /// NOT part of the event: the browser uses its own record of the tab's URL.
    LoginSubmitted {
        view: ViewId,
        username: String,
        password: String,
    },
    /// The user clicked (a real, trusted click) a login field: time to offer or do autofill.
    LoginFieldClicked { view: ViewId },
    /// A page has text typed into a form field that hasn't been submitted (blocks tab sleeping).
    FormInputChanged { view: ViewId, has_input: bool },
    /// The page wants a new window or tab. The engine did *not* open it: the browser decides
    /// (and keeps `opener` to draw the Trail's branch). `user_gesture` is false for pop-ups the
    /// user didn't ask for (EDGE_CASES §2).
    NewWindowRequested {
        opener: ViewId,
        url: String,
        kind: NewWindowKind,
        user_gesture: bool,
    },
    /// A download started or changed.
    Download(DownloadInfo),
    /// A page asks for a permission. Answer with [`Engine::answer_permission`].
    PermissionRequested {
        view: ViewId,
        request: u64,
        origin: String,
        kind: PermissionKind,
    },
    /// A certificate error on a navigation. Answer with [`Engine::answer_certificate_error`].
    CertificateError {
        view: ViewId,
        request: u64,
        url: String,
        error: String,
    },
    /// The page's renderer process crashed or was killed.
    Crashed { view: ViewId, reason: String },
    /// The view finished closing; its id is now invalid.
    Closed { view: ViewId },
    /// Count of ads and trackers blocked for this view changed (P5-01).
    ShieldsCountChanged { view: ViewId, count: u64 },
    /// A keyboard shortcut the user pressed in a page window: the id of the command it is bound
    /// to (see the shell's `shortcuts.rs`). The bindings come from `set_key_bindings`.
    Command { view: ViewId, id: String },
    /// A page answered a script the assistant ran in it (see the shell's `agent`): the request id
    /// and the JSON text it produced. Page content is untrusted; the id only routes the answer.
    AgentResult { view: ViewId, id: u64, json: String },
    /// Alt+F4: close the whole browser (not rebindable).
    ShortcutQuit,
    /// The browser is closing (every route: the close button, Alt+F4, the taskbar). Sent once,
    /// before the pages close, so the shell can save the session first.
    AppClosing,
}

/// The web engine, as the browser sees it. All methods are called on the UI thread.
/// Calls with an unknown or closed [`ViewId`] are ignored (never a crash).
pub trait Engine {
    /// Create a hidden page view loading `url`. Show it with [`Engine::set_visible`].
    fn create_view(&mut self, url: &str) -> ViewId;
    /// Close a page view (runs the page's `beforeunload`; [`Event::Closed`] follows).
    fn close_view(&mut self, view: ViewId);
    /// Place a page view in the main window's client area.
    fn set_bounds(&mut self, view: ViewId, bounds: Rect);
    /// Show or hide a page view. Hidden views keep running unless put to sleep.
    fn set_visible(&mut self, view: ViewId, visible: bool);

    /// Load a URL (already decided by the browser: URL vs search, blocked schemes, EDGE §3).
    fn navigate(&mut self, view: ViewId, url: &str);
    fn go_back(&mut self, view: ViewId);
    fn go_forward(&mut self, view: ViewId);
    fn reload(&mut self, view: ViewId);
    fn stop(&mut self, view: ViewId);

    /// Zoom level as a factor (1.0 = 100%).
    fn set_zoom(&mut self, view: ViewId, factor: f64);
    /// Find text in the page; `forward` picks the direction, `next` continues a search.
    fn find(&mut self, view: ViewId, text: &str, forward: bool, match_case: bool, next: bool);
    fn stop_finding(&mut self, view: ViewId, clear_selection: bool);

    /// Run a script in **our UI** only. There is deliberately no way to run a script in a
    /// web page from here.
    fn run_ui_script(&mut self, script: &str);

    /// Put a background tab to sleep (free its renderer) and wake it again at the same URL.
    fn sleep(&mut self, view: ViewId);
    fn wake(&mut self, view: ViewId);

    fn open_devtools(&mut self, view: ViewId);
    fn print(&mut self, view: ViewId);

    /// Answer a [`Event::PermissionRequested`].
    fn answer_permission(&mut self, request: u64, allow: bool);
    /// Answer a [`Event::CertificateError`] (proceed only after the user chose to).
    fn answer_certificate_error(&mut self, request: u64, proceed: bool);

    /// Pause, resume or cancel a download running under the engine (`action`: "pause", "resume", "cancel").
    fn download_control(&mut self, id: u32, action: &str);

    /// Minimize the main browser window.
    fn minimize_window(&mut self);
    /// Toggle maximize / restore of the main browser window.
    fn toggle_maximize_window(&mut self);
    /// Close the main browser window (terminates the application).
    fn close_window(&mut self);

    /// Toggle ad/tracker blocking exception for `host` (P5-01). Returns true if shields are now active.
    fn toggle_shields_exception(&mut self, _host: &str) -> bool {
        true
    }

    /// Whether shields are active for `host` (P5-01).
    fn is_shields_active_for(&self, _host: &str) -> bool {
        true
    }

    /// Number of items blocked for `view` (P5-01).
    fn shields_count(&self, _view: ViewId) -> u64 {
        0
    }

    /// Return whether a tab is currently sleeping (P5-02).
    fn is_sleeping(&self, _view: ViewId) -> bool {
        false
    }
}

/// Where our own browser UI (the HTML in `ui/`) lives on disk.
///
/// Looked up next to the executable first (installed builds), then in the source tree
/// (development builds).
pub fn find_ui_dir(exe_dir: &Path, source_ui: &Path) -> Option<PathBuf> {
    [exe_dir.join("ui"), source_ui.to_path_buf()]
        .into_iter()
        .find(|d| d.join("index.html").is_file())
}

/// A `file://` URL for a local path. Every byte outside a small safe set is percent-encoded,
/// so paths with spaces and non-ASCII user names work (EDGE_CASES §1).
pub fn file_url(path: &Path) -> String {
    let s = path.to_string_lossy().replace('\\', "/");
    let s = s.trim_start_matches("//?/");
    let mut out = String::from(if s.starts_with('/') {
        "file://"
    } else {
        "file:///"
    });
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"/:-_.~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn file_urls_encode_spaces_and_unicode() {
        assert_eq!(
            file_url(Path::new("C:\\Users\\A B\\ui\\index.html")),
            "file:///C:/Users/A%20B/ui/index.html"
        );
        assert_eq!(
            file_url(Path::new("/home/zoë/ui/index.html")),
            "file:///home/zo%C3%AB/ui/index.html"
        );
        assert_eq!(
            file_url(Path::new("\\\\?\\C:\\x\\i.html")),
            "file:///C:/x/i.html"
        );
    }

    #[test]
    fn ui_dir_prefers_installed_copy_and_handles_missing() {
        let n = TEST_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        let tmp =
            std::env::temp_dir().join(format!("engine-api-test-{}-{}", std::process::id(), n));
        let installed = tmp.join("bin");
        let source = tmp.join("src-ui");
        std::fs::create_dir_all(installed.join("ui")).unwrap();
        std::fs::create_dir_all(&source).unwrap();
        assert_eq!(find_ui_dir(&installed, &source), None);
        std::fs::write(source.join("index.html"), "x").unwrap();
        assert_eq!(find_ui_dir(&installed, &source), Some(source.clone()));
        std::fs::write(installed.join("ui").join("index.html"), "x").unwrap();
        assert_eq!(find_ui_dir(&installed, &source), Some(installed.join("ui")));
        std::fs::remove_dir_all(&tmp).ok();
    }

    /// The trait must stay object-safe so the shell can hold `Box<dyn Engine>` (D-005).
    #[test]
    fn engine_is_object_safe() {
        fn takes(_: Option<Box<dyn Engine>>) {}
        takes(None);
    }
}
