//! The new-tab wallpaper (AUDIT-37): one image the user picked, kept in the profile folder.
//!
//! The UI cannot write files, so it sends the picked image as a `data:` URL. Here the text is
//! checked (type, size, and that the bytes really start like a PNG, JPEG or WebP), then written
//! atomically as `wallpaper.png|jpg|webp`. Anything else is refused with a sentence for the user.
//! Never executed or parsed beyond the first bytes; the browser engine draws it as a plain image.

use base64::Engine as _;
use std::path::{Path, PathBuf};

/// Largest accepted image (after decoding).
pub const MAX_BYTES: usize = 4 * 1024 * 1024;

const KINDS: &[(&str, &str)] = &[
    ("image/png", "png"),
    ("image/jpeg", "jpg"),
    ("image/webp", "webp"),
];

fn magic_ok(mime: &str, b: &[u8]) -> bool {
    match mime {
        "image/png" => b.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]),
        "image/jpeg" => b.starts_with(&[0xFF, 0xD8, 0xFF]),
        "image/webp" => b.len() > 12 && &b[0..4] == b"RIFF" && &b[8..12] == b"WEBP",
        _ => false,
    }
}

fn existing(dir: &Path) -> Option<(PathBuf, &'static str)> {
    for (mime, ext) in KINDS {
        let p = dir.join(format!("wallpaper.{ext}"));
        if p.is_file() {
            return Some((p, mime));
        }
    }
    None
}

/// Checks a `data:` URL and returns the mime type and the image bytes.
pub fn decode(data_url: &str) -> Result<(&'static str, Vec<u8>), String> {
    // 4 MiB of bytes is about 5.6 MB of text; refuse anything much bigger before decoding.
    if data_url.len() > MAX_BYTES * 4 / 3 + 200 {
        return Err("That image is too big (4 MB at most).".into());
    }
    let rest = data_url
        .strip_prefix("data:")
        .ok_or("That is not an image file.")?;
    let (head, body) = rest.split_once(',').ok_or("That is not an image file.")?;
    let mime = head
        .strip_suffix(";base64")
        .ok_or("That is not an image file.")?;
    let mime = KINDS
        .iter()
        .map(|(m, _)| *m)
        .find(|m| *m == mime)
        .ok_or("Use a PNG, JPEG or WebP image.")?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(body.trim())
        .map_err(|_| "That is not an image file.".to_string())?;
    if bytes.len() > MAX_BYTES {
        return Err("That image is too big (4 MB at most).".into());
    }
    if !magic_ok(mime, &bytes) {
        return Err("That file is not really a PNG, JPEG or WebP image.".into());
    }
    Ok((mime, bytes))
}

/// Saves the wallpaper (replacing any earlier one).
pub fn save(dir: &Path, data_url: &str) -> Result<(), String> {
    let (mime, bytes) = decode(data_url)?;
    let ext = KINDS
        .iter()
        .find(|(m, _)| *m == mime)
        .map(|(_, e)| *e)
        .unwrap_or("png");
    std::fs::create_dir_all(dir).map_err(|e| format!("Couldn't save: {e}"))?;
    let tmp = dir.join("wallpaper.tmp");
    std::fs::write(&tmp, &bytes).map_err(|e| format!("Couldn't save: {e}"))?;
    clear(dir);
    std::fs::rename(&tmp, dir.join(format!("wallpaper.{ext}")))
        .map_err(|e| format!("Couldn't save: {e}"))
}

/// Removes the wallpaper (nothing to do when there is none).
pub fn clear(dir: &Path) {
    for (_, ext) in KINDS {
        let _ = std::fs::remove_file(dir.join(format!("wallpaper.{ext}")));
    }
}

/// The saved wallpaper as a `data:` URL, or `None` (none saved, unreadable or no longer valid).
pub fn load(dir: &Path) -> Option<String> {
    let (path, mime) = existing(dir)?;
    let bytes = std::fs::read(path).ok()?;
    if bytes.len() > MAX_BYTES || !magic_ok(mime, &bytes) {
        return None;
    }
    Some(format!(
        "data:{mime};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(mime: &str, bytes: &[u8]) -> String {
        format!(
            "data:{mime};base64,{}",
            base64::engine::general_purpose::STANDARD.encode(bytes)
        )
    }

    const PNG: &[u8] = &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 0];

    #[test]
    fn a_real_image_is_saved_loaded_and_removed() {
        let d = std::env::temp_dir().join(format!("wp-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        assert!(load(&d).is_none());
        save(&d, &url("image/png", PNG)).unwrap();
        assert_eq!(load(&d).unwrap(), url("image/png", PNG));
        // a new one replaces the old, whatever its type
        let mut jpg = vec![0xFF, 0xD8, 0xFF, 0xE0];
        jpg.extend([0u8; 8]);
        save(&d, &url("image/jpeg", &jpg)).unwrap();
        assert_eq!(load(&d).unwrap(), url("image/jpeg", &jpg));
        assert!(!d.join("wallpaper.png").exists());
        clear(&d);
        assert!(load(&d).is_none());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn anything_else_is_refused_with_a_sentence() {
        for (bad, want) in [
            ("hello".to_string(), "not an image"),
            (
                "data:text/html;base64,PGI+".to_string(),
                "PNG, JPEG or WebP",
            ),
            (
                "data:image/svg+xml;base64,PHN2Zz4=".to_string(),
                "PNG, JPEG or WebP",
            ),
            ("data:image/png;base64,@@@".to_string(), "not an image"),
            ("data:image/png,abc".to_string(), "not an image"),
            (url("image/png", b"<html>not a png</html>"), "not really"),
            (url("image/jpeg", PNG), "not really"),
            (url("image/png", &vec![0u8; MAX_BYTES + 1]), "too big"),
        ] {
            let e = decode(&bad).unwrap_err();
            assert!(e.contains(want), "{bad:.40}: {e}");
        }
        // a huge string is refused before it is decoded
        let huge = format!("data:image/png;base64,{}", "A".repeat(MAX_BYTES * 2));
        assert!(decode(&huge).unwrap_err().contains("too big"));
    }

    #[test]
    fn a_damaged_saved_file_is_ignored() {
        let d = std::env::temp_dir().join(format!("wp-bad-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&d);
        std::fs::write(d.join("wallpaper.png"), b"not a png").unwrap();
        assert!(load(&d).is_none());
        let _ = std::fs::remove_dir_all(&d);
    }
}
