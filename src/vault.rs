// Odyssey's password vault.
//
// Every saved password is encrypted individually with Windows DPAPI
// (CryptProtectData), with no CRYPTPROTECT_LOCAL_MACHINE flag -- which
// means it is tied to the current Windows user's login: only a process
// running as that same Windows account, on this machine, can ever decrypt
// it. There is no separate master password to create, remember, or lose;
// this matches what the user asked for when setting this up.
//
// Entries (origin, username, created/updated timestamps, and the
// DPAPI-encrypted password bytes) are stored as JSON next to Odyssey's own
// config file. The *decrypted* password is never written to disk and is
// only ever handed back across the Tauri bridge when the person explicitly
// asks to reveal, copy, or autofill it (vault_reveal / vault_autofill).

use std::path::PathBuf;

use base64::{engine::general_purpose::STANDARD as B64, Engine};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

#[cfg(windows)]
use std::ffi::c_void;
#[cfg(windows)]
use windows::{
    core::PCWSTR,
    Win32::Foundation::{LocalFree, HLOCAL},
    Win32::Security::Cryptography::{
        CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    },
};

#[derive(Clone, Deserialize, Serialize)]
pub struct VaultEntry {
    pub id: String,
    /// Site/host this login is for, e.g. "github.com". Compared
    /// case-insensitively when matching the current tab.
    pub origin: String,
    pub username: String,
    /// DPAPI-protected password bytes, base64-encoded. Opaque outside this
    /// module; never the plaintext password.
    pub secret: String,
    pub created_at: String,
    pub updated_at: String,
}

/// What the UI is allowed to see for an entry it isn't actively revealing:
/// everything except the password itself.
#[derive(Clone, Serialize)]
pub struct VaultEntrySummary {
    pub id: String,
    pub origin: String,
    pub username: String,
    pub updated_at: String,
}

impl From<&VaultEntry> for VaultEntrySummary {
    fn from(e: &VaultEntry) -> Self {
        VaultEntrySummary {
            id: e.id.clone(),
            origin: e.origin.clone(),
            username: e.username.clone(),
            updated_at: e.updated_at.clone(),
        }
    }
}

#[derive(Default, Deserialize, Serialize)]
struct VaultFile {
    entries: Vec<VaultEntry>,
}

fn vault_path(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_config_dir().ok().map(|d| d.join("vault.json"))
}

fn load(app: &AppHandle) -> VaultFile {
    vault_path(app)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save(app: &AppHandle, vault: &VaultFile) -> Result<(), String> {
    let path = vault_path(app).ok_or("no config dir")?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let s = serde_json::to_string_pretty(vault).map_err(|e| e.to_string())?;
    std::fs::write(path, s).map_err(|e| e.to_string())
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

// ------------------------------------------------------------- DPAPI

#[cfg(windows)]
fn dpapi_protect(plaintext: &[u8]) -> Result<Vec<u8>, String> {
    let mut input_buf = plaintext.to_vec();
    let input = CRYPT_INTEGER_BLOB {
        cbData: input_buf.len() as u32,
        pbData: input_buf.as_mut_ptr(),
    };
    let mut output = CRYPT_INTEGER_BLOB::default();
    unsafe {
        CryptProtectData(
            &input,
            PCWSTR::null(),
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
        .map_err(|e| format!("Windows couldn't encrypt that (DPAPI): {e}"))?;
    }
    let result = unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec() };
    unsafe {
        LocalFree(Some(HLOCAL(output.pbData as *mut c_void)));
    }
    Ok(result)
}

#[cfg(windows)]
fn dpapi_unprotect(ciphertext: &[u8]) -> Result<Vec<u8>, String> {
    let mut input_buf = ciphertext.to_vec();
    let input = CRYPT_INTEGER_BLOB {
        cbData: input_buf.len() as u32,
        pbData: input_buf.as_mut_ptr(),
    };
    let mut output = CRYPT_INTEGER_BLOB::default();
    unsafe {
        CryptUnprotectData(
            &input,
            None,
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
        .map_err(|e| {
            format!(
                "Windows couldn't decrypt that (DPAPI: {e}). This usually means it was \
                 saved under a different Windows account or on a different PC."
            )
        })?;
    }
    let result = unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec() };
    unsafe {
        LocalFree(Some(HLOCAL(output.pbData as *mut c_void)));
    }
    Ok(result)
}

#[cfg(not(windows))]
fn dpapi_protect(_plaintext: &[u8]) -> Result<Vec<u8>, String> {
    Err("Odyssey's password vault is only implemented on Windows (it uses DPAPI, tied to your Windows login).".into())
}

#[cfg(not(windows))]
fn dpapi_unprotect(_ciphertext: &[u8]) -> Result<Vec<u8>, String> {
    Err("Odyssey's password vault is only implemented on Windows (it uses DPAPI, tied to your Windows login).".into())
}

/// Decrypts one entry and returns (username, password). Not a Tauri
/// command -- only called from inside the Rust side (vault_reveal, and
/// main.rs's vault_autofill) so the plaintext only ever crosses the
/// frontend bridge when the person explicitly asked to see or use it.
pub fn get_entry_credentials(app: &AppHandle, id: &str) -> Result<(String, String), String> {
    let vault = load(app);
    let entry = vault.entries.iter().find(|e| e.id == id).ok_or("That saved login wasn't found.")?;
    let encrypted = B64.decode(&entry.secret).map_err(|e| e.to_string())?;
    let plain = dpapi_unprotect(&encrypted)?;
    let password = String::from_utf8(plain).map_err(|_| "Decrypted password wasn't valid text.".to_string())?;
    Ok((entry.username.clone(), password))
}

// ------------------------------------------------------------- commands

#[tauri::command]
pub async fn vault_list(app: AppHandle) -> Result<Vec<VaultEntrySummary>, String> {
    let vault = load(&app);
    let mut out: Vec<VaultEntrySummary> = vault.entries.iter().map(VaultEntrySummary::from).collect();
    out.sort_by(|a, b| a.origin.to_lowercase().cmp(&b.origin.to_lowercase()));
    Ok(out)
}

/// Saved logins whose origin matches the given host, for "fill on this
/// page" suggestions. Case-insensitive, exact host match only (no
/// subdomain guessing -- a wrong guess here would be worse than none).
#[tauri::command]
pub async fn vault_list_for_origin(app: AppHandle, origin: String) -> Result<Vec<VaultEntrySummary>, String> {
    let needle = origin.trim().to_lowercase();
    let vault = load(&app);
    Ok(vault
        .entries
        .iter()
        .filter(|e| e.origin.trim().to_lowercase() == needle)
        .map(VaultEntrySummary::from)
        .collect())
}

#[tauri::command]
pub async fn vault_add(app: AppHandle, origin: String, username: String, password: String) -> Result<VaultEntrySummary, String> {
    let origin = origin.trim().to_string();
    if origin.is_empty() {
        return Err("Site is required.".into());
    }
    let encrypted = dpapi_protect(password.as_bytes())?;
    let ts = now_ms();
    let entry = VaultEntry {
        id: new_id(),
        origin,
        username,
        secret: B64.encode(encrypted),
        created_at: ts.clone(),
        updated_at: ts,
    };
    let summary = VaultEntrySummary::from(&entry);
    let mut vault = load(&app);
    vault.entries.push(entry);
    save(&app, &vault)?;
    Ok(summary)
}

#[tauri::command]
pub async fn vault_update(
    app: AppHandle,
    id: String,
    origin: Option<String>,
    username: Option<String>,
    password: Option<String>,
) -> Result<(), String> {
    // Encrypt before taking the lock on the loaded vault so a failed
    // encrypt never leaves a half-updated entry.
    let new_secret = match password {
        Some(p) => Some(B64.encode(dpapi_protect(p.as_bytes())?)),
        None => None,
    };
    let mut vault = load(&app);
    let entry = vault
        .entries
        .iter_mut()
        .find(|e| e.id == id)
        .ok_or("That saved login wasn't found.")?;
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
    save(&app, &vault)
}

#[tauri::command]
pub async fn vault_delete(app: AppHandle, id: String) -> Result<(), String> {
    let mut vault = load(&app);
    let before = vault.entries.len();
    vault.entries.retain(|e| e.id != id);
    if vault.entries.len() == before {
        return Err("That saved login wasn't found.".into());
    }
    save(&app, &vault)
}

/// Decrypts and returns one entry's password. Only call this from a
/// person-initiated "reveal" or "copy password" action in the UI.
#[tauri::command]
pub async fn vault_reveal(app: AppHandle, id: String) -> Result<String, String> {
    get_entry_credentials(&app, &id).map(|(_, password)| password)
}
