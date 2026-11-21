//! Password vault (P8-02, EDGE_CASES §1, §7).
//!
//! Windows: each password is encrypted with DPAPI (tied to the Windows login).
//! Linux: each password lives in the Secret Service keyring; `vault.json` only holds a marker.
//! If no secure store is available the operation fails with a clear error. Nothing is ever
//! stored in a reversible form on disk.

use base64::{Engine, engine::general_purpose::STANDARD as B64};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[cfg(windows)]
use windows_sys::Win32::Foundation::LocalFree;
#[cfg(windows)]
use windows_sys::Win32::Security::Cryptography::{
    CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptProtectData, CryptUnprotectData,
};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct VaultEntry {
    pub id: String,
    pub origin: String,
    pub username: String,
    /// DPAPI-protected password bytes, base64-encoded.
    pub secret: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct VaultEntrySummary {
    pub id: String,
    pub origin: String,
    pub username: String,
    pub updated_at: String,
}

impl From<&VaultEntry> for VaultEntrySummary {
    fn from(e: &VaultEntry) -> Self {
        Self {
            id: e.id.clone(),
            origin: e.origin.clone(),
            username: e.username.clone(),
            updated_at: e.updated_at.clone(),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct VaultStore {
    pub entries: Vec<VaultEntry>,
    /// Sites the user chose "Never" for when asked to save a password (host keys).
    #[serde(default)]
    pub never: Vec<String>,
    /// Set when the file on disk was damaged; show it to the user.
    #[serde(skip)]
    pub load_notice: Option<String>,
    #[serde(skip)]
    read_only: bool,
}

/// What a saved login is matched against: the site's host (with the port when there is one),
/// lowercase, without `www.`. `https://www.Example.com/login?x=1` and `example.com` match.
pub fn host_key(origin: &str) -> String {
    let t = origin.trim().to_lowercase();
    let t = t
        .strip_prefix("https://")
        .or_else(|| t.strip_prefix("http://"))
        .unwrap_or(&t);
    let t = t.split(['/', '?', '#']).next().unwrap_or("");
    let t = t.rsplit('@').next().unwrap_or(t);
    t.strip_prefix("www.").unwrap_or(t).to_string()
}

/// A saved login with its password decrypted. Never leaves Rust except into the page that the
/// user clicked a login field on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Credential {
    pub id: String,
    pub username: String,
    pub password: String,
}

/// Result of an import.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct ImportSummary {
    pub added: usize,
    pub skipped: usize,
}

/// Parses CSV (quotes, doubled quotes, newlines inside quotes, CRLF, BOM) into rows.
fn parse_csv(text: &str) -> Vec<Vec<String>> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if quoted {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    field.push('"');
                    chars.next();
                } else {
                    quoted = false;
                }
            } else {
                field.push(c);
            }
        } else {
            match c {
                '"' if field.is_empty() => quoted = true,
                ',' => row.push(std::mem::take(&mut field)),
                '\r' => {}
                '\n' => {
                    row.push(std::mem::take(&mut field));
                    rows.push(std::mem::take(&mut row));
                }
                _ => field.push(c),
            }
        }
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }
    rows.retain(|r| !(r.len() == 1 && r[0].is_empty()));
    rows
}

fn csv_field(s: &str) -> String {
    if s.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn now_ms() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis().to_string())
        .unwrap_or_default()
}

fn new_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{nanos:x}")
}

#[cfg(windows)]
fn dpapi_protect(plaintext: &[u8]) -> Result<Vec<u8>, String> {
    let mut input_buf = plaintext.to_vec();
    let input = CRYPT_INTEGER_BLOB {
        cbData: input_buf.len() as u32,
        pbData: input_buf.as_mut_ptr(),
    };
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };
    let ok = unsafe {
        CryptProtectData(
            &input,
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
    };
    if ok == 0 {
        return Err("Windows DPAPI failed to protect password.".to_string());
    }
    let res = unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize) }.to_vec();
    unsafe {
        LocalFree(output.pbData as _);
    }
    Ok(res)
}

#[cfg(windows)]
fn dpapi_unprotect(ciphertext: &[u8]) -> Result<Vec<u8>, String> {
    let mut input_buf = ciphertext.to_vec();
    let input = CRYPT_INTEGER_BLOB {
        cbData: input_buf.len() as u32,
        pbData: input_buf.as_mut_ptr(),
    };
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };
    let ok = unsafe {
        CryptUnprotectData(
            &input,
            std::ptr::null_mut(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
    };
    if ok == 0 {
        return Err(
            "Windows DPAPI failed to unprotect password (may belong to another user).".to_string(),
        );
    }
    let res = unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize) }.to_vec();
    unsafe {
        LocalFree(output.pbData as _);
    }
    Ok(res)
}

#[cfg(windows)]
fn protect(_id: &str, plain: &[u8]) -> Result<Vec<u8>, String> {
    dpapi_protect(plain)
}
#[cfg(windows)]
fn unprotect(_id: &str, data: &[u8]) -> Result<Vec<u8>, String> {
    dpapi_unprotect(data)
}
#[cfg(not(windows))]
fn protect(id: &str, plain: &[u8]) -> Result<Vec<u8>, String> {
    let text = std::str::from_utf8(plain).map_err(|_| "Password is not valid text.".to_string())?;
    secrets::set(&format!("vault:{id}"), text)?;
    Ok(b"keyring".to_vec())
}
#[cfg(not(windows))]
fn unprotect(id: &str, _marker: &[u8]) -> Result<Vec<u8>, String> {
    secrets::get(&format!("vault:{id}"))?
        .map(String::into_bytes)
        .ok_or_else(|| "The password is missing from the system keyring.".to_string())
}

/// Named secrets in the OS credential store (Windows Credential Manager, Linux Secret
/// Service). Used for the vault on Linux and for AI keys everywhere (D-014).
pub mod secrets {
    const SERVICE: &str = "constellations-browser";

    fn entry(name: &str) -> Result<keyring::Entry, String> {
        keyring::Entry::new(SERVICE, name).map_err(|e| format!("Secure store unavailable: {e}"))
    }

    pub fn set(name: &str, value: &str) -> Result<(), String> {
        entry(name)?
            .set_password(value)
            .map_err(|e| format!("Couldn't save to the secure store: {e}"))
    }

    pub fn get(name: &str) -> Result<Option<String>, String> {
        match entry(name)?.get_password() {
            Ok(v) => Ok(Some(v)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(format!("Couldn't read the secure store: {e}")),
        }
    }

    pub fn delete(name: &str) -> Result<(), String> {
        match entry(name)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(format!("Couldn't update the secure store: {e}")),
        }
    }
}

impl VaultStore {
    /// A missing file gives an empty vault. A file that can't be parsed is moved aside to
    /// `vault.json.corrupt-<time>` (never overwritten) and reported in `load_notice`.
    pub fn load(path: &Path) -> Self {
        let data = match std::fs::read_to_string(path) {
            Ok(d) => d,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Self::default(),
            Err(e) => {
                return Self {
                    load_notice: Some(format!("Couldn't read saved logins: {e}")),
                    read_only: true,
                    ..Self::default()
                };
            }
        };
        match serde_json::from_str::<Self>(&data) {
            Ok(store) => store,
            Err(_) => {
                let mut backup = path.as_os_str().to_owned();
                backup.push(format!(".corrupt-{}", now_ms()));
                let moved = std::fs::rename(path, &backup).is_ok();
                Self {
                    load_notice: Some(if moved {
                        "Saved logins couldn't be read. The damaged file was kept as vault.json.corrupt-*.".to_string()
                    } else {
                        "Saved logins couldn't be read and the damaged file couldn't be moved aside.".to_string()
                    }),
                    read_only: !moved,
                    ..Self::default()
                }
            }
        }
    }

    /// Writes a temp file then renames it, so a crash never leaves a half-written vault.
    pub fn save(&self, path: &Path) -> Result<(), String> {
        if self.read_only {
            return Err("Saved logins are locked because the file on disk is damaged.".to_string());
        }
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let data = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        let mut tmp = path.as_os_str().to_owned();
        tmp.push(".tmp");
        let tmp = std::path::PathBuf::from(tmp);
        std::fs::write(&tmp, data).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, path).map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            e.to_string()
        })
    }

    /// Saved logins for the page at `host` (a host key or any URL), passwords decrypted.
    pub fn credentials_for_host(&self, host: &str) -> Result<Vec<Credential>, String> {
        let key = host_key(host);
        let mut out = Vec::new();
        for e in self.entries.iter().filter(|e| host_key(&e.origin) == key) {
            let encrypted = B64.decode(&e.secret).map_err(|e| e.to_string())?;
            let plain = unprotect(&e.id, &encrypted)?;
            out.push(Credential {
                id: e.id.clone(),
                username: e.username.clone(),
                password: String::from_utf8(plain)
                    .map_err(|_| "Decrypted password is not valid text.".to_string())?,
            });
        }
        out.sort_by_key(|c| c.username.to_lowercase());
        Ok(out)
    }

    /// The saved login for this site and username, if there is one.
    pub fn find_login(&self, host: &str, username: &str) -> Option<Credential> {
        self.credentials_for_host(host)
            .ok()?
            .into_iter()
            .find(|c| c.username == username)
    }

    pub fn is_never(&self, host: &str) -> bool {
        let key = host_key(host);
        self.never.contains(&key)
    }

    pub fn add_never(&mut self, host: &str) {
        let key = host_key(host);
        if !key.is_empty() && !self.never.contains(&key) {
            self.never.push(key);
            self.never.sort();
        }
    }

    pub fn remove_never(&mut self, host: &str) {
        let key = host_key(host);
        self.never.retain(|n| *n != key);
    }

    /// All logins as CSV in the layout Chrome and Brave use: name,url,username,password,note.
    /// The file holds passwords in plain text.
    pub fn export_csv(&self) -> Result<String, String> {
        let mut out = String::from("name,url,username,password,note\r\n");
        let mut entries: Vec<&VaultEntry> = self.entries.iter().collect();
        entries.sort_by_key(|e| host_key(&e.origin));
        for e in entries {
            let encrypted = B64.decode(&e.secret).map_err(|e| e.to_string())?;
            let plain = String::from_utf8(unprotect(&e.id, &encrypted)?)
                .map_err(|_| "A saved password is not valid text.".to_string())?;
            let url = if e.origin.contains("://") {
                e.origin.clone()
            } else {
                format!("https://{}", e.origin)
            };
            out.push_str(&format!(
                "{},{},{},{},\r\n",
                csv_field(&host_key(&e.origin)),
                csv_field(&url),
                csv_field(&e.username),
                csv_field(&plain)
            ));
        }
        Ok(out)
    }

    /// Adds logins from CSV (Chrome, Brave, Edge and Firefox exports all work: columns are found
    /// by name). Rows without a site or password, and logins that already exist for the same site
    /// and username, are skipped.
    pub fn import_csv(&mut self, csv: &str) -> Result<ImportSummary, String> {
        let rows = parse_csv(csv);
        let Some(header) = rows.first() else {
            return Err("The file is empty.".to_string());
        };
        let find = |names: &[&str]| {
            header
                .iter()
                .position(|h| names.contains(&h.trim().to_lowercase().as_str()))
        };
        let (Some(url_i), Some(pass_i)) = (
            find(&["url", "origin", "website", "site", "hostname", "name"]),
            find(&["password"]),
        ) else {
            return Err(
                "This doesn't look like a passwords file (no url and password columns)."
                    .to_string(),
            );
        };
        let user_i = find(&["username", "login", "email", "user", "login_username"]);
        if rows.len() > 5001 {
            return Err("Too many rows (limit 5000).".to_string());
        }
        let mut summary = ImportSummary::default();
        for r in rows.iter().skip(1) {
            let site = r.get(url_i).map(|s| s.trim()).unwrap_or("");
            let password = r.get(pass_i).map(String::as_str).unwrap_or("");
            let username = user_i
                .and_then(|i| r.get(i))
                .map(String::as_str)
                .unwrap_or("");
            if site.is_empty() || password.is_empty() || host_key(site).is_empty() {
                summary.skipped += 1;
                continue;
            }
            if self.find_login(site, username).is_some() {
                summary.skipped += 1;
                continue;
            }
            self.add(site, username, password)?;
            summary.added += 1;
        }
        Ok(summary)
    }

    pub fn list(&self) -> Vec<VaultEntrySummary> {
        let mut out: Vec<VaultEntrySummary> =
            self.entries.iter().map(VaultEntrySummary::from).collect();
        out.sort_by_key(|a| a.origin.to_lowercase());
        out
    }

    pub fn list_for_origin(&self, origin: &str) -> Vec<VaultEntrySummary> {
        let needle = origin.trim().to_lowercase();
        self.entries
            .iter()
            .filter(|e| e.origin.trim().to_lowercase() == needle)
            .map(VaultEntrySummary::from)
            .collect()
    }

    pub fn add(
        &mut self,
        origin: &str,
        username: &str,
        password: &str,
    ) -> Result<VaultEntrySummary, String> {
        let origin = origin.trim().to_string();
        if origin.is_empty() {
            return Err("Site is required.".to_string());
        }
        let id = new_id();
        let encrypted = protect(&id, password.as_bytes())?;
        let ts = now_ms();
        let entry = VaultEntry {
            id,
            origin,
            username: username.to_string(),
            secret: B64.encode(encrypted),
            created_at: ts.clone(),
            updated_at: ts,
        };
        let summary = VaultEntrySummary::from(&entry);
        self.entries.push(entry);
        Ok(summary)
    }

    pub fn update(
        &mut self,
        id: &str,
        origin: Option<String>,
        username: Option<String>,
        password: Option<String>,
    ) -> Result<(), String> {
        let new_secret = if let Some(p) = password {
            Some(B64.encode(protect(id, p.as_bytes())?))
        } else {
            None
        };
        let entry = self
            .entries
            .iter_mut()
            .find(|e| e.id == id)
            .ok_or_else(|| "Saved login not found.".to_string())?;
        if let Some(o) = origin {
            entry.origin = o.trim().to_string();
        }
        if let Some(u) = username {
            entry.username = u;
        }
        if let Some(s) = new_secret {
            entry.secret = s;
        }
        entry.updated_at = now_ms();
        Ok(())
    }

    pub fn delete(&mut self, id: &str) -> Result<(), String> {
        let before = self.entries.len();
        self.entries.retain(|e| e.id != id);
        if self.entries.len() == before {
            return Err("Saved login not found.".to_string());
        }
        #[cfg(not(windows))]
        let _ = secrets::delete(&format!("vault:{id}"));
        Ok(())
    }

    pub fn reveal(&self, id: &str) -> Result<String, String> {
        let entry = self
            .entries
            .iter()
            .find(|e| e.id == id)
            .ok_or_else(|| "Saved login not found.".to_string())?;
        let encrypted = B64.decode(&entry.secret).map_err(|e| e.to_string())?;
        let plain = unprotect(&entry.id, &encrypted)?;
        String::from_utf8(plain).map_err(|_| "Decrypted password is not valid text.".to_string())
    }

    pub fn get_credentials(&self, id: &str) -> Result<(String, String), String> {
        let entry = self
            .entries
            .iter()
            .find(|e| e.id == id)
            .ok_or_else(|| "Saved login not found.".to_string())?;
        let encrypted = B64.decode(&entry.secret).map_err(|e| e.to_string())?;
        let plain = unprotect(&entry.id, &encrypted)?;
        let pass = String::from_utf8(plain)
            .map_err(|_| "Decrypted password is not valid text.".to_string())?;
        Ok((entry.username.clone(), pass))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("vault-test-{name}-{}", new_id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("vault.json")
    }

    #[test]
    fn host_keys_ignore_scheme_path_www_and_case() {
        assert_eq!(host_key("https://www.Example.com/login?x=1"), "example.com");
        assert_eq!(host_key("example.com"), "example.com");
        assert_eq!(host_key("http://127.0.0.1:8765/a.html"), "127.0.0.1:8765");
        assert_eq!(host_key("user@git.example.com:22"), "git.example.com:22");
    }

    #[test]
    fn credentials_match_by_site_and_never_list_works() {
        let mut s = VaultStore::default();
        s.add("https://www.github.com/login", "octocat", "pw1")
            .unwrap();
        s.add("github.com", "hubot", "pw2").unwrap();
        s.add("example.org", "x", "y").unwrap();
        let c = s.credentials_for_host("https://github.com/").unwrap();
        assert_eq!(c.len(), 2);
        assert_eq!(c[0].username, "hubot");
        assert_eq!(
            s.find_login("github.com", "octocat").unwrap().password,
            "pw1"
        );
        assert!(s.find_login("github.com", "nobody").is_none());
        assert!(!s.is_never("github.com"));
        s.add_never("https://www.github.com/");
        assert!(s.is_never("github.com"));
        s.remove_never("github.com");
        assert!(!s.is_never("github.com"));
    }

    #[test]
    fn csv_round_trip_with_awkward_characters() {
        let mut a = VaultStore::default();
        a.add("example.com", "me@example.com", "pa\"ss,word\nline2")
            .unwrap();
        a.add("other.net", "user", "plain").unwrap();
        let csv = a.export_csv().unwrap();
        assert!(csv.starts_with("name,url,username,password,note"));
        let mut b = VaultStore::default();
        let r = b.import_csv(&csv).unwrap();
        assert_eq!(
            r,
            ImportSummary {
                added: 2,
                skipped: 0
            }
        );
        assert_eq!(
            b.find_login("example.com", "me@example.com")
                .unwrap()
                .password,
            "pa\"ss,word\nline2"
        );
        // importing again adds nothing
        assert_eq!(
            b.import_csv(&csv).unwrap(),
            ImportSummary {
                added: 0,
                skipped: 2
            }
        );
    }

    #[test]
    fn import_reads_chrome_and_firefox_layouts_and_rejects_other_files() {
        let mut s = VaultStore::default();
        let chrome =
            "\u{feff}name,url,username,password,note\r\nsite,https://a.example/,u1,p1,\r\n";
        assert_eq!(s.import_csv(chrome).unwrap().added, 1);
        let firefox =
            "\"url\",\"username\",\"password\"\r\n\"https://b.example\",\"u2\",\"p2\"\r\n";
        assert_eq!(s.import_csv(firefox).unwrap().added, 1);
        assert!(s.import_csv("a,b\n1,2\n").is_err());
        assert!(s.import_csv("").is_err());
        let blank = "url,username,password\nhttps://c.example,u3,\n,u4,p4\n";
        assert_eq!(
            s.import_csv(blank).unwrap(),
            ImportSummary {
                added: 0,
                skipped: 2
            }
        );
    }

    #[test]
    fn corrupt_file_is_kept_and_not_overwritten() {
        let path = temp_path("corrupt");
        std::fs::write(&path, "{ not json").unwrap();
        let store = VaultStore::load(&path);
        assert!(store.load_notice.is_some());
        assert!(store.entries.is_empty());
        let moved: Vec<_> = std::fs::read_dir(path.parent().unwrap())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().contains(".corrupt-"))
            .collect();
        assert_eq!(moved.len(), 1);
        assert_eq!(
            std::fs::read_to_string(moved[0].path()).unwrap(),
            "{ not json"
        );
    }

    #[test]
    fn interrupted_write_keeps_old_file() {
        let path = temp_path("atomic");
        std::fs::write(&path, r#"{"entries":[]}"#).unwrap();
        let mut tmp = path.as_os_str().to_owned();
        tmp.push(".tmp");
        std::fs::write(&tmp, "{ half").unwrap();
        let store = VaultStore::load(&path);
        assert!(store.load_notice.is_none());
        store.save(&path).unwrap();
        assert!(VaultStore::load(&path).load_notice.is_none());
    }

    #[test]
    fn test_vault_protect_and_reveal() {
        let mut store = VaultStore::default();
        let summary = store
            .add("github.com", "octocat", "super-secret-password-123")
            .expect("add succeeds");

        assert_eq!(summary.origin, "github.com");
        assert_eq!(summary.username, "octocat");

        let list = store.list();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, summary.id);

        let revealed = store.reveal(&summary.id).expect("reveal succeeds");
        assert_eq!(revealed, "super-secret-password-123");

        let for_origin = store.list_for_origin("GITHUB.COM");
        assert_eq!(for_origin.len(), 1);

        store.delete(&summary.id).expect("delete succeeds");
        assert_eq!(store.list().len(), 0);
    }
}
