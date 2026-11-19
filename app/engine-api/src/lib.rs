//! The engine interface (D-005). All browser code talks to the web engine through this crate,
//! never to CEF directly. P1-02 defines the trait; P1-01 only sets up the crate.

/// Where our own browser UI (the HTML in `ui/`) lives on disk.
///
/// Looked up next to the executable first (installed builds), then in the source tree
/// (development builds).
pub fn find_ui_dir(
    exe_dir: &std::path::Path,
    source_ui: &std::path::Path,
) -> Option<std::path::PathBuf> {
    [exe_dir.join("ui"), source_ui.to_path_buf()]
        .into_iter()
        .find(|d| d.join("index.html").is_file())
}

/// A `file://` URL for a local path. Every byte outside a small safe set is percent-encoded,
/// so paths with spaces and non-ASCII user names work (EDGE_CASES §1).
pub fn file_url(path: &std::path::Path) -> String {
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
    use std::path::Path;

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
        let tmp = std::env::temp_dir().join(format!("engine-api-test-{}", std::process::id()));
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
}
