//! Updates (AUDIT-23): find a newer version, download it, check it, and install it.
//!
//! Nothing is installed that was not signed by us. The update manifest (`latest.json`) lists, per
//! platform, the file's address, its SHA-256 and size and an Ed25519 signature over
//! `constellations-update/v1\n<version>\n<platform>\n<sha256>\n<size>`. The matching public key is
//! built into the program, so a hacked server or a swapped file cannot make us run anything. The
//! download is checked (size, SHA-256, signature) before the installer is started, and a failed
//! check deletes it. Only `https` addresses are used (a local test server is allowed in debug builds).
//! No identifier or history is sent: it is one plain GET of the manifest, then one of the file.

use base64::Engine as _;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Where the manifest is read from when the user has not set another address.
pub const DEFAULT_MANIFEST_URL: &str =
    "https://raw.githubusercontent.com/13LA7E/Odys-Brows/Constellations/releases/latest.json";

/// The public half of the signing key (the private half is kept by the owner, never in the repo).
const PUBLIC_KEY_B64: &str = "9up2e0vwR92mqP71NyKkB0DUOTSt/dKaGo88oDlAIBs=";

/// Largest download accepted.
pub const MAX_DOWNLOAD: u64 = 700 * 1024 * 1024;

pub const PLATFORM: &str = if cfg!(windows) {
    "windows-x86_64"
} else {
    "linux-x86_64"
};

#[derive(Clone, Debug, Deserialize)]
pub struct PlatformEntry {
    pub url: String,
    pub sha256: String,
    pub size: u64,
    /// Base64 of the Ed25519 signature.
    pub signature: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Manifest {
    pub version: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub platforms: std::collections::HashMap<String, PlatformEntry>,
}

/// A newer version that has been checked and is ready to download.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Available {
    pub version: String,
    pub notes: String,
    pub url: String,
    pub sha256: String,
    pub size: u64,
}

/// The version of this program (debug builds may pretend to be another, to test updating).
pub fn current_version() -> String {
    #[cfg(debug_assertions)]
    if let Ok(v) = std::env::var("CONSTELLATIONS_FAKE_VERSION") {
        return v;
    }
    env!("CARGO_PKG_VERSION").to_string()
}

/// The public key to check with. Debug builds may be pointed at a test key.
fn public_key() -> Result<VerifyingKey, String> {
    #[cfg(debug_assertions)]
    let b64 = std::env::var("CONSTELLATIONS_UPDATE_PUBKEY")
        .unwrap_or_else(|_| PUBLIC_KEY_B64.to_string());
    #[cfg(not(debug_assertions))]
    let b64 = PUBLIC_KEY_B64.to_string();
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64.trim())
        .map_err(|_| "The built-in update key is damaged.".to_string())?;
    let arr: [u8; 32] = bytes
        .try_into()
        .map_err(|_| "The built-in update key is damaged.".to_string())?;
    VerifyingKey::from_bytes(&arr).map_err(|_| "The built-in update key is damaged.".to_string())
}

/// The text that is signed for one platform's file.
pub fn signed_message(version: &str, platform: &str, sha256: &str, size: u64) -> String {
    format!("constellations-update/v1\n{version}\n{platform}\n{sha256}\n{size}")
}

fn verify_signature(key: &VerifyingKey, msg: &str, signature_b64: &str) -> bool {
    let Ok(sig_bytes) = base64::engine::general_purpose::STANDARD.decode(signature_b64.trim())
    else {
        return false;
    };
    let Ok(arr) = <[u8; 64]>::try_from(sig_bytes.as_slice()) else {
        return false;
    };
    key.verify(msg.as_bytes(), &Signature::from_bytes(&arr))
        .is_ok()
}

/// "1.2.3" or "1.2.3-beta" as numbers; a version with a pre-release tag is older than the same
/// number without it. `None` when it is not a version at all.
pub fn parse_version(v: &str) -> Option<(u64, u64, u64, bool)> {
    let v = v.trim().trim_start_matches('v');
    let (core, pre) = match v.split_once(['-', '+']) {
        Some((c, _)) => (c, true),
        None => (v, false),
    };
    let mut it = core.split('.');
    let a = it.next()?.parse().ok()?;
    let b = it.next().unwrap_or("0").parse().ok()?;
    let c = it.next().unwrap_or("0").parse().ok()?;
    if it.next().is_some() {
        return None;
    }
    Some((a, b, c, !pre))
}

pub fn is_newer(candidate: &str, current: &str) -> bool {
    match (parse_version(candidate), parse_version(current)) {
        (Some(a), Some(b)) => a > b,
        _ => false,
    }
}

/// Only https (and, in debug builds, a local test server).
pub fn allowed_url(url: &str) -> bool {
    if url.len() > 600 || url.chars().any(|c| c.is_control() || c == ' ') {
        return false;
    }
    if url.starts_with("https://") {
        return true;
    }
    cfg!(debug_assertions)
        && (url.starts_with("http://127.0.0.1") || url.starts_with("http://localhost"))
}

/// Looks at a manifest's text: is there a newer, properly signed version for this platform?
/// `Ok(None)` = up to date. Anything wrong (bad signature, wrong platform, odd address) is an error.
pub fn evaluate(manifest_text: &str, current: &str) -> Result<Option<Available>, String> {
    let m: Manifest = serde_json::from_str(manifest_text)
        .map_err(|_| "The update information could not be read.".to_string())?;
    if parse_version(&m.version).is_none() {
        return Err("The update information has an odd version number.".into());
    }
    if !is_newer(&m.version, current) {
        return Ok(None);
    }
    let entry = m
        .platforms
        .get(PLATFORM)
        .ok_or("There is no update for this kind of computer yet.")?;
    let sha = entry.sha256.trim().to_lowercase();
    if sha.len() != 64 || !sha.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("The update information is damaged.".into());
    }
    if entry.size == 0 || entry.size > MAX_DOWNLOAD {
        return Err("The update file has an odd size.".into());
    }
    if !allowed_url(&entry.url) {
        return Err("The update address is not allowed.".into());
    }
    let key = public_key()?;
    let msg = signed_message(&m.version, PLATFORM, &sha, entry.size);
    if !verify_signature(&key, &msg, &entry.signature) {
        return Err("The update is not signed by us, so it was not used.".into());
    }
    Ok(Some(Available {
        version: m.version,
        notes: m.notes.chars().take(2000).collect(),
        url: entry.url.clone(),
        sha256: sha,
        size: entry.size,
    }))
}

/// Reads the manifest from `url` and evaluates it.
pub async fn check(url: &str, current: &str) -> Result<Option<Available>, String> {
    if !allowed_url(url) {
        return Err("The update address must start with https://.".into());
    }
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("Constellations-updater")
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Could not reach the update server: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("The update server answered {}.", resp.status()));
    }
    if resp.content_length().is_some_and(|n| n > 256 * 1024) {
        return Err("The update information is too big.".into());
    }
    let text = resp.text().await.map_err(|e| e.to_string())?;
    if text.len() > 256 * 1024 {
        return Err("The update information is too big.".into());
    }
    evaluate(&text, current)
}

/// Downloads the file, checks its size and SHA-256 against the signed values and returns its path.
/// A file that does not match is deleted. `progress(received, total)` is called now and then.
pub async fn download(
    a: &Available,
    dir: &Path,
    mut progress: impl FnMut(u64, u64),
) -> Result<PathBuf, String> {
    use std::io::Write;
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let ext = if cfg!(windows) {
        if cfg!(debug_assertions) && a.url.ends_with(".bat") {
            "bat"
        } else {
            "exe"
        }
    } else {
        "AppImage"
    };
    let final_path = dir.join(format!(
        "Constellations-Update-{}.{ext}",
        a.version.replace(['/', '\\', ' '], "_")
    ));
    let part = final_path.with_extension("part");
    let _ = std::fs::remove_file(&part);
    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(15))
        .user_agent("Constellations-updater")
        .build()
        .map_err(|e| e.to_string())?;
    let mut resp = client
        .get(&a.url)
        .send()
        .await
        .map_err(|e| format!("Could not reach the update server: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("The update server answered {}.", resp.status()));
    }
    // a redirect must not lead somewhere we would not have gone
    if !allowed_url(resp.url().as_str()) {
        return Err("The update address is not allowed.".into());
    }
    let mut file = std::fs::File::create(&part).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    let mut got: u64 = 0;
    let fail = |msg: String, part: &Path| -> Result<PathBuf, String> {
        let _ = std::fs::remove_file(part);
        Err(msg)
    };
    loop {
        match resp.chunk().await {
            Ok(Some(chunk)) => {
                got += chunk.len() as u64;
                if got > a.size || got > MAX_DOWNLOAD {
                    drop(file);
                    return fail("The update file is bigger than it should be.".into(), &part);
                }
                hasher.update(&chunk);
                if file.write_all(&chunk).is_err() {
                    drop(file);
                    return fail(
                        "The update could not be saved (is the disk full?).".into(),
                        &part,
                    );
                }
                progress(got, a.size);
            }
            Ok(None) => break,
            Err(e) => {
                drop(file);
                return fail(format!("The download was interrupted: {e}"), &part);
            }
        }
    }
    drop(file);
    if got != a.size {
        return fail("The update file is incomplete.".into(), &part);
    }
    let digest: String = hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    if digest != a.sha256 {
        return fail(
            "The update file does not match what was signed, so it was deleted.".into(),
            &part,
        );
    }
    std::fs::rename(&part, &final_path).map_err(|e| e.to_string())?;
    Ok(final_path)
}

/// Starts the installer after this program has had a moment to close, then the caller quits.
/// Windows: the NSIS setup with `/S` (silent). Linux: the AppImage is made executable and run.
pub fn start_installer(path: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // `ping` is a delay that needs no extra program: the installer starts after we have exited.
        let p = path.to_string_lossy().to_string();
        std::process::Command::new("cmd")
            .raw_arg(format!("/C ping -n 4 127.0.0.1 >nul & \"{p}\" /S"))
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("Could not start the installer: {e}"))
    }
    #[cfg(not(windows))]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755));
        std::process::Command::new("sh")
            .arg("-c")
            .arg(format!("sleep 3; \"{}\"", path.display()))
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("Could not start the installer: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    fn test_key() -> SigningKey {
        SigningKey::from_bytes(&[7u8; 32])
    }

    fn with_test_key<T>(f: impl FnOnce() -> T) -> T {
        let pk =
            base64::engine::general_purpose::STANDARD.encode(test_key().verifying_key().to_bytes());
        // tests share one process: set the key for the duration of one call
        static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        // SAFETY: serialised by LOCK; nothing else reads this variable concurrently in tests.
        unsafe { std::env::set_var("CONSTELLATIONS_UPDATE_PUBKEY", pk) };
        let r = f();
        unsafe { std::env::remove_var("CONSTELLATIONS_UPDATE_PUBKEY") };
        r
    }

    fn manifest(version: &str, sha: &str, size: u64, sign: bool, url: &str) -> String {
        let msg = signed_message(version, PLATFORM, sha, size);
        let sig = if sign {
            base64::engine::general_purpose::STANDARD
                .encode(test_key().sign(msg.as_bytes()).to_bytes())
        } else {
            base64::engine::general_purpose::STANDARD.encode([0u8; 64])
        };
        serde_json::json!({
            "version": version, "notes": "Better things.",
            "platforms": { PLATFORM: { "url": url, "sha256": sha, "size": size, "signature": sig } }
        })
        .to_string()
    }

    const SHA: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    #[test]
    fn versions_compare_like_people_expect() {
        assert!(is_newer("0.2.0", "0.1.9"));
        assert!(is_newer("1.0.0", "0.99.99"));
        assert!(is_newer("v0.1.1", "0.1.0"));
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(!is_newer("0.0.9", "0.1.0"));
        assert!(
            is_newer("0.1.0", "0.1.0-beta"),
            "a release is newer than its beta"
        );
        assert!(!is_newer("0.1.0-beta", "0.1.0"));
        assert!(!is_newer("garbage", "0.1.0") && !is_newer("0.2.0", "garbage"));
        assert_eq!(parse_version("1.2"), Some((1, 2, 0, true)));
        assert_eq!(parse_version("1.2.3.4"), None);
    }

    #[test]
    fn only_https_addresses_are_used() {
        assert!(allowed_url("https://example.com/a.exe"));
        assert!(!allowed_url("http://example.com/a.exe"));
        assert!(!allowed_url("file:///c:/x.exe"));
        assert!(!allowed_url("ftp://example.com/x"));
        assert!(!allowed_url("https://exa mple.com/x"));
        assert!(!allowed_url(&format!("https://e.com/{}", "a".repeat(700))));
        // a local test server is allowed in debug builds (which these tests are)
        assert!(allowed_url("http://127.0.0.1:8080/latest.json"));
        assert!(!allowed_url("http://evil.test/latest.json"));
    }

    #[test]
    fn a_signed_newer_version_is_offered_and_everything_else_is_not() {
        with_test_key(|| {
            let ok = evaluate(
                &manifest("9.9.9", SHA, 1000, true, "https://example.com/x.exe"),
                "0.1.0",
            )
            .unwrap()
            .unwrap();
            assert_eq!((ok.version.as_str(), ok.size), ("9.9.9", 1000));
            // up to date
            assert_eq!(
                evaluate(
                    &manifest("0.1.0", SHA, 1000, true, "https://example.com/x.exe"),
                    "0.1.0"
                )
                .unwrap(),
                None
            );
            // forged signature
            assert!(
                evaluate(
                    &manifest("9.9.9", SHA, 1000, false, "https://example.com/x.exe"),
                    "0.1.0"
                )
                .unwrap_err()
                .contains("not signed")
            );
            // a signature for other values does not carry over (the file, size or version changed)
            let m = manifest("9.9.9", SHA, 1000, true, "https://example.com/x.exe")
                .replace("1000", "2000");
            assert!(evaluate(&m, "0.1.0").unwrap_err().contains("not signed"));
            let m = manifest("9.9.9", SHA, 1000, true, "https://example.com/x.exe")
                .replace("9.9.9", "9.9.8");
            assert!(evaluate(&m, "0.1.0").unwrap_err().contains("not signed"));
            // odd input
            assert!(evaluate("not json", "0.1.0").is_err());
            assert!(
                evaluate(
                    &manifest("9.9.9", "zz", 1000, true, "https://example.com/x.exe"),
                    "0.1.0"
                )
                .is_err()
            );
            assert!(
                evaluate(
                    &manifest("9.9.9", SHA, 0, true, "https://example.com/x.exe"),
                    "0.1.0"
                )
                .is_err()
            );
            assert!(
                evaluate(
                    &manifest(
                        "9.9.9",
                        SHA,
                        MAX_DOWNLOAD + 1,
                        true,
                        "https://example.com/x.exe"
                    ),
                    "0.1.0"
                )
                .is_err()
            );
            assert!(
                evaluate(
                    &manifest("9.9.9", SHA, 1000, true, "http://example.com/x.exe"),
                    "0.1.0"
                )
                .unwrap_err()
                .contains("not allowed")
            );
            assert!(
                evaluate(r#"{"version":"9.9.9","platforms":{}}"#, "0.1.0")
                    .unwrap_err()
                    .contains("no update")
            );
        });
    }

    #[test]
    fn the_built_in_key_is_a_valid_public_key() {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(PUBLIC_KEY_B64)
            .unwrap();
        assert_eq!(bytes.len(), 32);
        assert!(VerifyingKey::from_bytes(&bytes.try_into().unwrap()).is_ok());
        // and a signature made with another key is not accepted by it
        let key = VerifyingKey::from_bytes(
            &base64::engine::general_purpose::STANDARD
                .decode(PUBLIC_KEY_B64)
                .unwrap()
                .try_into()
                .unwrap(),
        )
        .unwrap();
        let sig =
            base64::engine::general_purpose::STANDARD.encode(test_key().sign(b"hello").to_bytes());
        assert!(!verify_signature(&key, "hello", &sig));
    }
}
