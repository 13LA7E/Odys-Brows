//! Extensions management (Phase 6: P6-01, P6-02, P6-03 / EDGE_CASES §10).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ExtensionInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub enabled: bool,
    pub permissions: Vec<String>,
    pub icon: Option<String>,
    pub memory_mb: f64,
    pub cpu_pct: f64,
    pub path: Option<String>,
    /// Chromium's own id for this extension (what its pages live under).
    #[serde(default)]
    pub chrome_id: String,
    /// The page its toolbar button opens (`action.default_popup`), if any.
    #[serde(default)]
    pub popup: Option<String>,
    /// Its settings page (`options_page` or `options_ui.page`), if any.
    #[serde(default)]
    pub options: Option<String>,
    /// Loaded in the running browser (no restart needed); not saved.
    #[serde(default, skip_serializing_if = "is_false")]
    pub live: bool,
}

fn is_false(b: &bool) -> bool {
    !*b
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
struct ManifestV3 {
    pub manifest_version: Option<u32>,
    pub name: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub optional_permissions: Vec<String>,
    #[serde(default)]
    pub action: Option<ActionManifest>,
    #[serde(default)]
    pub default_locale: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ActionManifest {
    #[serde(default)]
    pub default_popup: Option<String>,
}

/// The id Chromium gives an unpacked extension (one without a `key`): the SHA-256 of its absolute
/// path as the platform stores it (UTF-16 on Windows), first 16 bytes, each nibble written as a
/// letter a to p. Used to open the extension's popup page.
pub fn chrome_extension_id(path: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    #[cfg(windows)]
    for u in path.encode_utf16() {
        h.update(u.to_le_bytes());
    }
    #[cfg(not(windows))]
    h.update(path.as_bytes());
    let d = h.finalize();
    d[..16]
        .iter()
        .flat_map(|b| [b >> 4, b & 0xF])
        .map(|n| (b'a' + n) as char)
        .collect()
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ExtensionsStore {
    pub extensions: HashMap<String, ExtensionInfo>,
}

impl ExtensionsStore {
    pub fn load_or_default(dir: &Path) -> Self {
        let file = dir.join("extensions.json");
        let Ok(content) = std::fs::read_to_string(&file) else {
            return Self::default();
        };
        serde_json::from_str::<ExtensionsStore>(&content).unwrap_or_default()
    }

    pub fn save(&self, dir: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(dir)?;
        let file = dir.join("extensions.json");
        let tmp = dir.join("extensions.json.tmp");
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(&tmp, json)?;
        std::fs::rename(tmp, file)?;
        Ok(())
    }

    pub fn list(&self) -> Vec<ExtensionInfo> {
        let mut list: Vec<_> = self.extensions.values().cloned().collect();
        list.sort_by_key(|a| a.name.to_lowercase());
        list
    }

    pub fn toggle(&mut self, id: &str, enabled: bool) -> bool {
        if let Some(ext) = self.extensions.get_mut(id) {
            ext.enabled = enabled;
            true
        } else {
            false
        }
    }

    /// Chromium's id for a running extension (from the load answer).
    pub fn set_chrome_id(&mut self, id: &str, chrome_id: &str) {
        if let Some(e) = self.extensions.get_mut(id) {
            e.chrome_id = chrome_id.to_string();
        }
    }

    pub fn remove(&mut self, id: &str) -> bool {
        self.extensions.remove(id).is_some()
    }

    pub fn load_unpacked(&mut self, path: &Path) -> Result<ExtensionInfo, String> {
        let manifest_path = path.join("manifest.json");
        if !manifest_path.exists() {
            return Err("manifest.json not found in the selected folder".to_string());
        }

        let content = std::fs::read_to_string(&manifest_path)
            .map_err(|e| format!("Failed to read manifest.json: {e}"))?;

        let raw: serde_json::Value =
            serde_json::from_str(content.trim_start_matches('\u{feff}')).unwrap_or_default();
        // Store extensions often start with a byte order mark.
        let manifest: ManifestV3 = serde_json::from_str(content.trim_start_matches('\u{feff}'))
            .map_err(|e| format!("Invalid manifest.json: {e}"))?;

        if let Some(mv) = manifest.manifest_version.filter(|&v| v < 3) {
            return Err(format!(
                "Manifest V{mv} is deprecated. Constellations requires Manifest V3."
            ));
        }

        let locale = manifest.default_locale.clone();
        let name = localized(
            path,
            locale.as_deref(),
            manifest
                .name
                .unwrap_or_else(|| "Unnamed Extension".to_string()),
        );
        let version = manifest.version.unwrap_or_else(|| "1.0.0".to_string());
        let description = localized(
            path,
            locale.as_deref(),
            manifest.description.unwrap_or_default(),
        );
        let permissions = manifest.permissions;
        let manifest_popup = manifest.action.and_then(|a| a.default_popup);

        // Deterministic ID derived from path
        let id = format!("{:x}", md5_hash(path.to_string_lossy().as_bytes()));

        let info = ExtensionInfo {
            id: id.clone(),
            name,
            version,
            description,
            enabled: true,
            permissions,
            icon: read_icon(path, &raw),
            // Not measured yet: CEF gives no per-extension cost, so no number is invented.
            memory_mb: 0.0,
            cpu_pct: 0.0,
            path: Some(path.to_string_lossy().to_string()),
            chrome_id: chrome_extension_id(&path.to_string_lossy()),
            popup: manifest_popup.filter(|p| valid_popup_path(p)),
            options: options_page(&raw),
            live: false,
        };

        self.extensions.insert(id, info.clone());
        Ok(info)
    }
}

/// True for a link that hands out a Chrome extension: the store's update service or a plain `.crx`.
pub fn is_crx_url(url: &str) -> bool {
    let Some(rest) = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
    else {
        return false;
    };
    let (host_path, _query) = rest.split_once('?').unwrap_or((rest, ""));
    let host = host_path.split('/').next().unwrap_or("").to_lowercase();
    let lower = host_path.to_lowercase();
    ((host == "clients2.google.com" || host == "clients2.googleusercontent.com")
        && lower.contains("/service/update2/crx"))
        || lower.ends_with(".crx")
}

/// Fetch an extension file ourselves (Chromium would throw a store download away) into `dest`.
pub async fn fetch_crx(url: &str, dest: &Path) -> Result<(), String> {
    const MAX: usize = 100 * 1024 * 1024;
    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(15))
        .timeout(std::time::Duration::from_secs(120))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36")
        .build()
        .map_err(|e| e.to_string())?;
    let mut resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Could not download it: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("The server answered {}.", resp.status()));
    }
    let mut data: Vec<u8> = Vec::new();
    while let Some(chunk) = resp.chunk().await.map_err(|e| e.to_string())? {
        if data.len() + chunk.len() > MAX {
            return Err("That extension file is too big.".into());
        }
        data.extend_from_slice(&chunk);
    }
    if data.get(0..4) != Some(b"Cr24") {
        return Err("The address did not give an extension file.".into());
    }
    if let Some(parent) = dest.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(dest, data).map_err(|e| format!("Could not save it: {e}"))
}

/// Unpack a downloaded Chrome extension (`.crx`, format 2 or 3) into `dest_root/<file name>` and
/// return that folder. The files are only written there: a name that would leave the folder, too
/// many files or too much data is refused, and nothing is run.
pub fn unpack_crx(crx: &Path, dest_root: &Path) -> Result<std::path::PathBuf, String> {
    const MAX_FILES: usize = 5000;
    const MAX_BYTES: u64 = 200 * 1024 * 1024;
    let bytes = std::fs::read(crx).map_err(|e| format!("Could not read the file: {e}"))?;
    if bytes.len() > 100 * 1024 * 1024 {
        return Err("That extension file is too big.".into());
    }
    let u32_at = |i: usize| -> Option<usize> {
        bytes
            .get(i..i + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize)
    };
    if bytes.get(0..4) != Some(b"Cr24") {
        return Err("That is not an extension file (.crx).".into());
    }
    let start = match u32_at(4) {
        Some(3) => 12usize.checked_add(u32_at(8).ok_or("Damaged extension file.")?),
        Some(2) => 16usize
            .checked_add(u32_at(8).ok_or("Damaged extension file.")?)
            .and_then(|v| v.checked_add(u32_at(12)?)),
        _ => return Err("That extension file format is not supported.".into()),
    }
    .filter(|s| *s < bytes.len())
    .ok_or("Damaged extension file.")?;
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(&bytes[start..]))
        .map_err(|_| "Damaged extension file.".to_string())?;
    if zip.len() > MAX_FILES {
        return Err("That extension has too many files.".into());
    }
    let stem: String = crx
        .file_stem()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_default()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .take(60)
        .collect();
    let dir = dest_root.join(if stem.is_empty() { "extension" } else { &stem });
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).map_err(|e| format!("Could not make the folder: {e}"))?;
    let mut total = 0u64;
    for i in 0..zip.len() {
        let mut f = zip
            .by_index(i)
            .map_err(|_| "Damaged extension file.".to_string())?;
        let Some(rel) = f.enclosed_name() else {
            continue; // a name that points outside the folder is skipped
        };
        let out = dir.join(rel);
        if f.is_dir() {
            let _ = std::fs::create_dir_all(&out);
            continue;
        }
        total += f.size();
        if total > MAX_BYTES {
            let _ = std::fs::remove_dir_all(&dir);
            return Err("That extension is too big once unpacked.".into());
        }
        if let Some(parent) = out.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let mut w =
            std::fs::File::create(&out).map_err(|e| format!("Could not write a file: {e}"))?;
        std::io::copy(&mut f.by_ref().take(MAX_BYTES), &mut w)
            .map_err(|e| format!("Could not write a file: {e}"))?;
    }
    if !dir.join("manifest.json").exists() {
        let _ = std::fs::remove_dir_all(&dir);
        return Err("That file has no manifest.json.".into());
    }
    Ok(dir)
}

/// The extension's settings page, if the manifest names a valid one.
fn options_page(raw: &serde_json::Value) -> Option<String> {
    raw.get("options_page")
        .and_then(|v| v.as_str())
        .or_else(|| {
            raw.get("options_ui")
                .and_then(|o| o.get("page"))
                .and_then(|v| v.as_str())
        })
        .filter(|p| valid_popup_path(p))
        .map(str::to_string)
}

/// The biggest icon (up to 128 px) from `icons` or `action.default_icon`, as a data URL.
fn read_icon(dir: &Path, raw: &serde_json::Value) -> Option<String> {
    use base64::Engine as _;
    let pick = |v: &serde_json::Value| -> Option<String> {
        if let Some(s) = v.as_str() {
            return Some(s.to_string());
        }
        let obj = v.as_object()?;
        obj.iter()
            .filter_map(|(k, v)| Some((k.parse::<u32>().ok()?, v.as_str()?.to_string())))
            .filter(|(n, _)| *n <= 128)
            .max_by_key(|(n, _)| *n)
            .map(|(_, p)| p)
    };
    let rel = raw.get("icons").and_then(pick).or_else(|| {
        raw.get("action")
            .and_then(|a| a.get("default_icon"))
            .and_then(pick)
    })?;
    if !valid_popup_path(&rel) {
        return None;
    }
    let mime = match Path::new(&rel)
        .extension()?
        .to_string_lossy()
        .to_lowercase()
        .as_str()
    {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        _ => return None,
    };
    let bytes = std::fs::read(dir.join(rel)).ok()?;
    if bytes.len() > 300 * 1024 {
        return None;
    }
    Some(format!(
        "data:{mime};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}

/// `__MSG_key__` names come from `_locales/<default_locale>/messages.json` (or `en`).
fn localized(dir: &Path, locale: Option<&str>, text: String) -> String {
    let Some(key) = text
        .strip_prefix("__MSG_")
        .and_then(|t| t.strip_suffix("__"))
    else {
        return text;
    };
    for loc in locale.into_iter().chain(["en", "en_US"]) {
        if loc.is_empty() || loc.contains(['/', '\\', '.']) {
            continue;
        }
        let file = dir.join("_locales").join(loc).join("messages.json");
        if let Ok(raw) = std::fs::read_to_string(file)
            && let Ok(v) =
                serde_json::from_str::<serde_json::Value>(raw.trim_start_matches('\u{feff}'))
            && let Some(obj) = v.as_object()
        {
            // keys are case-insensitive in Chrome
            for (k, val) in obj {
                if k.eq_ignore_ascii_case(key)
                    && let Some(m) = val.get("message").and_then(|m| m.as_str())
                {
                    return m.to_string();
                }
            }
        }
    }
    text
}

/// A relative path inside the extension: no scheme, no `..`, no leading slash.
fn valid_popup_path(p: &str) -> bool {
    !p.is_empty()
        && p.len() < 200
        && !p.contains("..")
        && !p.contains(':')
        && !p.starts_with('/')
        && !p.starts_with('\\')
        && !p.chars().any(char::is_control)
}

fn md5_hash(data: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in data {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_crx(files: &[(&str, &str)], version: u32) -> Vec<u8> {
        use std::io::Write;
        let mut zbytes = Vec::new();
        {
            let mut w = zip::ZipWriter::new(std::io::Cursor::new(&mut zbytes));
            let o = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            for (n, c) in files {
                w.start_file(*n, o).unwrap();
                w.write_all(c.as_bytes()).unwrap();
            }
            w.finish().unwrap();
        }
        let mut out = b"Cr24".to_vec();
        out.extend_from_slice(&version.to_le_bytes());
        if version == 3 {
            out.extend_from_slice(&5u32.to_le_bytes());
            out.extend_from_slice(&[0; 5]);
        } else {
            out.extend_from_slice(&2u32.to_le_bytes());
            out.extend_from_slice(&3u32.to_le_bytes());
            out.extend_from_slice(&[0; 5]);
        }
        out.extend_from_slice(&zbytes);
        out
    }

    #[test]
    fn unpacks_crx_files_and_refuses_bad_ones() {
        let tmp = std::env::temp_dir().join(format!("crx-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let manifest = r#"{"manifest_version":3,"name":"Demo","version":"1.0"}"#;
        for v in [3u32, 2] {
            let f = tmp.join(format!("abcdef{v}.CRX"));
            std::fs::write(
                &f,
                make_crx(&[("manifest.json", manifest), ("a/b.js", "x")], v),
            )
            .unwrap();
            let dir = unpack_crx(&f, &tmp.join("out")).unwrap();
            assert!(dir.join("manifest.json").exists() && dir.join("a").join("b.js").exists());
            let mut reg = ExtensionsStore::default();
            assert_eq!(reg.load_unpacked(&dir).unwrap().name, "Demo");
        }
        // a name that climbs out of the folder is skipped, not written
        let f = tmp.join("evil.crx");
        std::fs::write(
            &f,
            make_crx(&[("manifest.json", manifest), ("../escape.txt", "x")], 3),
        )
        .unwrap();
        unpack_crx(&f, &tmp.join("out")).unwrap();
        assert!(!tmp.join("escape.txt").exists() && !tmp.join("out").join("escape.txt").exists());
        // no manifest, not a crx, truncated
        let f = tmp.join("nomanifest.crx");
        std::fs::write(&f, make_crx(&[("a.js", "x")], 3)).unwrap();
        assert!(unpack_crx(&f, &tmp.join("out")).is_err());
        std::fs::write(tmp.join("x.crx"), b"hello").unwrap();
        assert!(unpack_crx(&tmp.join("x.crx"), &tmp.join("out")).is_err());
        std::fs::write(tmp.join("y.crx"), b"Cr24\x03\0\0\0\xff\xff\xff\x7f").unwrap();
        assert!(unpack_crx(&tmp.join("y.crx"), &tmp.join("out")).is_err());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn store_names_are_translated() {
        let tmp = std::env::temp_dir().join(format!("loc-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("_locales").join("en")).unwrap();
        std::fs::write(
            tmp.join("manifest.json"),
            "\u{feff}{\"manifest_version\":3,\"name\":\"__MSG_Title__\",\"version\":\"1\",\"default_locale\":\"en\"}",
        )
        .unwrap();
        std::fs::write(
            tmp.join("_locales").join("en").join("messages.json"),
            "{\"title\":{\"message\":\"Nice Ext\"}}",
        )
        .unwrap();
        let mut reg = ExtensionsStore::default();
        assert_eq!(reg.load_unpacked(&tmp).unwrap().name, "Nice Ext");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn recognises_extension_links() {
        assert!(is_crx_url(
            "https://clients2.google.com/service/update2/crx?response=redirect&x=id%3Dabc"
        ));
        assert!(is_crx_url("https://example.com/files/thing.CRX?x=1"));
        assert!(!is_crx_url("https://example.com/thing.zip"));
        assert!(!is_crx_url("https://evil.com/service/update2/crx"));
        assert!(!is_crx_url("file:///c:/a.crx"));
    }

    #[test]
    fn loads_unpacked_v3_manifest() {
        let tmp = std::env::temp_dir().join(format!("ext-test-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&tmp);
        let manifest = r#"{
            "manifest_version": 3,
            "name": "Test Extension",
            "version": "2.1.0",
            "description": "A helpful test extension",
            "permissions": ["storage", "tabs"]
        }"#;
        std::fs::write(tmp.join("manifest.json"), manifest).unwrap();

        let mut store = ExtensionsStore::default();
        let res = store.load_unpacked(&tmp);
        assert!(res.is_ok());
        let info = res.unwrap();
        assert_eq!(info.name, "Test Extension");
        assert_eq!(info.version, "2.1.0");
        assert_eq!(info.permissions, vec!["storage", "tabs"]);
        assert!(info.enabled);

        // Toggle and remove
        assert!(store.toggle(&info.id, false));
        assert!(!store.extensions.get(&info.id).unwrap().enabled);
        assert!(store.remove(&info.id));
        assert!(store.extensions.is_empty());

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn rejects_v2_manifest() {
        let tmp = std::env::temp_dir().join(format!("ext-test-v2-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&tmp);
        let manifest = r#"{
            "manifest_version": 2,
            "name": "Old Extension",
            "version": "1.0"
        }"#;
        std::fs::write(tmp.join("manifest.json"), manifest).unwrap();

        let mut store = ExtensionsStore::default();
        let res = store.load_unpacked(&tmp);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("Manifest V2 is deprecated"));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn the_chromium_id_is_32_letters_and_depends_on_the_path() {
        let a = chrome_extension_id(r"C:\x\one");
        assert_eq!(a.len(), 32);
        assert!(a.chars().all(|c| ('a'..='p').contains(&c)));
        assert_eq!(a, chrome_extension_id(r"C:\x\one"));
        assert_ne!(a, chrome_extension_id(r"C:\x\two"));
    }

    #[test]
    fn popup_paths_stay_inside_the_extension() {
        for ok in ["popup.html", "ui/popup.html"] {
            assert!(valid_popup_path(ok), "{ok}");
        }
        for bad in [
            "",
            "../x.html",
            "/etc/x",
            "https://evil.test/",
            r"a\b:c",
            r"\\server\share",
        ] {
            assert!(!valid_popup_path(bad), "{bad}");
        }
    }
}
