//! Serves our UI from `odyssey-ui://app/...` instead of `file://` (P1-03).
//!
//! Why: Chromium won't run ES module scripts from `file://` (opaque origin), and a custom
//! scheme gives our UI its own origin. The scheme is registered *display isolated*, so web
//! pages can't link to it or frame it. Files are read only from the UI folder; any path that
//! tries to leave it is refused.

use cef::*;
use std::cell::RefCell;
use std::path::{Component, Path, PathBuf};
use std::rc::Rc;

pub const SCHEME: &str = "odyssey-ui";
pub const HOST: &str = "app";

/// `odyssey-ui://app/`: every UI URL starts with this.
pub fn prefix() -> String {
    format!("{SCHEME}://{HOST}/")
}

/// Scheme options: standard URL rules, treated like https, only displayable from itself,
/// may use fetch().
pub fn options() -> i32 {
    use sys::cef_scheme_options_t as O;
    (O::CEF_SCHEME_OPTION_STANDARD as i32)
        | (O::CEF_SCHEME_OPTION_SECURE as i32)
        | (O::CEF_SCHEME_OPTION_DISPLAY_ISOLATED as i32)
        | (O::CEF_SCHEME_OPTION_FETCH_ENABLED as i32)
}

fn percent_decode(s: &str) -> Option<String> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            let hex = s.get(i + 1..i + 3)?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// Map a UI URL to a file inside `root`. `None` for anything outside the UI folder.
pub fn resolve(root: &Path, url: &str) -> Option<PathBuf> {
    let rest = url.strip_prefix(&prefix())?;
    let rest = rest.split(['?', '#']).next().unwrap_or("");
    let rest = if rest.is_empty() { "index.html" } else { rest };
    let rel = percent_decode(rest)?;
    if rel.contains('\\') || rel.contains('\0') || rel.contains(':') {
        return None;
    }
    let rel = Path::new(&rel);
    if !rel.components().all(|c| matches!(c, Component::Normal(_))) {
        return None;
    }
    let path = root.join(rel);
    // Resolve links too: the final file must still be inside the UI folder.
    let real = std::fs::canonicalize(&path).ok()?;
    let real_root = std::fs::canonicalize(root).ok()?;
    (real.starts_with(&real_root) && real.is_file()).then_some(real)
}

pub fn mime_for(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "html" => "text/html",
        "css" => "text/css",
        "js" | "mjs" => "text/javascript",
        "json" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "woff2" => "font/woff2",
        "txt" => "text/plain",
        _ => "application/octet-stream",
    }
}

struct Body {
    status: i32,
    mime: &'static str,
    data: Vec<u8>,
    offset: usize,
}

wrap_resource_handler! {
    struct UiResource { body: Rc<RefCell<Body>> }

    impl ResourceHandler {
        fn open(&self, _request: Option<&mut Request>, handle_request: Option<&mut i32>, _callback: Option<&mut Callback>) -> i32 {
            if let Some(h) = handle_request {
                *h = 1; // answered right here, synchronously
            }
            1
        }

        fn response_headers(&self, response: Option<&mut Response>, response_length: Option<&mut i64>, _redirect_url: Option<&mut CefString>) {
            let body = self.body.borrow();
            if let Some(r) = response {
                r.set_status(body.status);
                r.set_mime_type(Some(&CefString::from(body.mime)));
                if body.mime.starts_with("text/") || body.mime == "application/json" {
                    r.set_charset(Some(&CefString::from("utf-8")));
                }
                // Our UI must not be cached between versions, and it never runs other origins' code.
                r.set_header_by_name(Some(&CefString::from("Cache-Control")), Some(&CefString::from("no-store")), 1);
            }
            if let Some(len) = response_length {
                *len = body.data.len() as i64;
            }
        }

        fn read(&self, data_out: *mut u8, bytes_to_read: i32, bytes_read: Option<&mut i32>, _callback: Option<&mut ResourceReadCallback>) -> i32 {
            let mut body = self.body.borrow_mut();
            let left = body.data.len().saturating_sub(body.offset);
            let n = left.min(bytes_to_read.max(0) as usize);
            if n > 0 && !data_out.is_null() {
                // SAFETY: CEF gives us a buffer of `bytes_to_read` bytes; we write at most `n` ≤ that.
                unsafe { std::ptr::copy_nonoverlapping(body.data[body.offset..].as_ptr(), data_out, n) };
                body.offset += n;
            }
            if let Some(r) = bytes_read {
                *r = n as i32;
            }
            (n > 0) as i32
        }
    }
}

wrap_scheme_handler_factory! {
    pub struct UiSchemeFactory { root: PathBuf }

    impl SchemeHandlerFactory {
        fn create(&self, _browser: Option<&mut Browser>, _frame: Option<&mut Frame>, _scheme_name: Option<&CefString>, request: Option<&mut Request>) -> Option<ResourceHandler> {
            let url = request.map(|r| CefString::from(&r.url()).to_string()).unwrap_or_default();
            let body = match resolve(&self.root, &url).and_then(|p| std::fs::read(&p).ok().map(|d| (p, d))) {
                Some((path, data)) => Body { status: 200, mime: mime_for(&path), data, offset: 0 },
                None => {
                    eprintln!("ui scheme: not found or refused: {url}");
                    Body { status: 404, mime: "text/plain", data: b"Not found".to_vec(), offset: 0 }
                }
            };
            Some(UiResource::new(Rc::new(RefCell::new(body))))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn tree() -> PathBuf {
        let n = TEST_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        let root =
            std::env::temp_dir().join(format!("ui-scheme-test-{}-{}", std::process::id(), n));
        std::fs::create_dir_all(root.join("ui").join("fonts")).unwrap();
        std::fs::write(root.join("ui").join("index.html"), "<p>").unwrap();
        std::fs::write(root.join("ui").join("fonts").join("a b.woff2"), "x").unwrap();
        std::fs::write(root.join("secret.txt"), "no").unwrap();
        root
    }

    #[test]
    fn serves_files_inside_the_ui_folder() {
        let root = tree();
        let ui = root.join("ui");
        assert!(
            resolve(&ui, "odyssey-ui://app/")
                .unwrap()
                .ends_with("index.html")
        );
        assert!(
            resolve(&ui, "odyssey-ui://app/index.html?fixture=Main#x")
                .unwrap()
                .ends_with("index.html")
        );
        assert!(
            resolve(&ui, "odyssey-ui://app/fonts/a%20b.woff2")
                .unwrap()
                .ends_with("a b.woff2")
        );
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn refuses_anything_outside_or_odd() {
        let root = tree();
        let ui = root.join("ui");
        for url in [
            "odyssey-ui://app/../secret.txt",
            "odyssey-ui://app/%2e%2e/secret.txt",
            "odyssey-ui://app/fonts/..%2f..%2fsecret.txt",
            "odyssey-ui://app/..\\secret.txt",
            "odyssey-ui://app/C:/Windows/win.ini",
            "odyssey-ui://app//etc/passwd",
            "odyssey-ui://app/missing.js",
            "odyssey-ui://app/fonts",
            "odyssey-ui://app/%zz",
            "odyssey-ui://other/index.html",
            "https://app/index.html",
        ] {
            assert_eq!(resolve(&ui, url), None, "{url}");
        }
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn mime_types() {
        assert_eq!(mime_for(Path::new("a.JS")), "text/javascript");
        assert_eq!(mime_for(Path::new("f.woff2")), "font/woff2");
        assert_eq!(mime_for(Path::new("noext")), "application/octet-stream");
    }
}
