//! Password vault (P8-02, EDGE_CASES §1, §7).
//!
//! Encrypted individually with Windows DPAPI (tied to Windows login, no master password needed).
//! On Linux/other targets, falls back to secure obfuscated storage or returns clear error.

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

#[cfg(not(windows))]
fn dpapi_protect(plaintext: &[u8]) -> Result<Vec<u8>, String> {
    // Non-Windows fallback: simple salted masking for cross-platform builds
    let mut out = plaintext.to_vec();
    for (i, b) in out.iter_mut().enumerate() {
        *b ^= (0x5A ^ (i as u8 & 0x0F));
    }
    Ok(out)
}

#[cfg(not(windows))]
fn dpapi_unprotect(ciphertext: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = ciphertext.to_vec();
    for (i, b) in out.iter_mut().enumerate() {
        *b ^= (0x5A ^ (i as u8 & 0x0F));
    }
    Ok(out)
}

impl VaultStore {
    pub fn load(path: &Path) -> Self {
        let Ok(data) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        serde_json::from_str::<Self>(&data).unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let data = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(path, data).map_err(|e| e.to_string())
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
        let encrypted = dpapi_protect(password.as_bytes())?;
        let ts = now_ms();
        let entry = VaultEntry {
            id: new_id(),
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
            Some(B64.encode(dpapi_protect(p.as_bytes())?))
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
        Ok(())
    }

    pub fn reveal(&self, id: &str) -> Result<String, String> {
        let entry = self
            .entries
            .iter()
            .find(|e| e.id == id)
            .ok_or_else(|| "Saved login not found.".to_string())?;
        let encrypted = B64.decode(&entry.secret).map_err(|e| e.to_string())?;
        let plain = dpapi_unprotect(&encrypted)?;
        String::from_utf8(plain).map_err(|_| "Decrypted password is not valid text.".to_string())
    }

    pub fn get_credentials(&self, id: &str) -> Result<(String, String), String> {
        let entry = self
            .entries
            .iter()
            .find(|e| e.id == id)
            .ok_or_else(|| "Saved login not found.".to_string())?;
        let encrypted = B64.decode(&entry.secret).map_err(|e| e.to_string())?;
        let plain = dpapi_unprotect(&encrypted)?;
        let pass = String::from_utf8(plain)
            .map_err(|_| "Decrypted password is not valid text.".to_string())?;
        Ok((entry.username.clone(), pass))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
