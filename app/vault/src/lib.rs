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
    /// Set when the file on disk was damaged; show it to the user.
    #[serde(skip)]
    pub load_notice: Option<String>,
    #[serde(skip)]
    read_only: bool,
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
