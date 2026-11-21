//! "Clear when Constellations closes" (AUDIT-40).
//!
//! Chromium keeps its data in files in the profile folder, and they are locked while it runs. So
//! the clearing happens at a safe point: after the engine has shut down, and again at the next
//! start if the last run didn't end cleanly (a crash, a kill, a power cut). A `running.lock` file
//! marks a run in progress; it is removed only after a clean exit whose clearing fully succeeded.
//!
//! Folder and file names are Chromium's; names that don't exist in a given version are simply
//! skipped (unverified for future versions: the unit test only uses a made-up profile).

use serde_json::Value;
use std::path::Path;

const LOCK: &str = "running.lock";

/// Chromium's own browsing history, favicons and suggestion data (under `Default/`).
const CHROMIUM_HISTORY: &[&str] = &[
    "History",
    "History-journal",
    "Favicons",
    "Favicons-journal",
    "Top Sites",
    "Top Sites-journal",
    "Visited Links",
    "Shortcuts",
    "Shortcuts-journal",
    "Network Action Predictor",
    "Network Action Predictor-journal",
];

/// Cookies, site storage and caches (under `Default/`).
const SITE_DATA: &[&str] = &[
    "Network/Cookies",
    "Network/Cookies-journal",
    "Network/Reporting and NEL",
    "Network/Reporting and NEL-journal",
    "Network/Trust Tokens",
    "Network/Trust Tokens-journal",
    "Network/Device Bound Sessions",
    "Network/Device Bound Sessions-journal",
    "Local Storage",
    "Session Storage",
    "IndexedDB",
    "Service Worker",
    "File System",
    "Storage",
    "blob_storage",
    "shared_proto_db",
    "Cache",
    "Code Cache",
    "DIPS",
    "DIPS-wal",
    "Trust Tokens",
    "Trust Tokens-journal",
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Flags {
    pub history: bool,
    pub site_data: bool,
}

impl Flags {
    pub fn from_settings(settings: &Value) -> Self {
        let on = |k: &str| settings.get(k).and_then(Value::as_bool).unwrap_or(false);
        Self {
            history: on("clearHistoryOnExit"),
            site_data: on("clearSiteDataOnExit"),
        }
    }
    fn any(self) -> bool {
        self.history || self.site_data
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Report {
    pub removed: usize,
    pub failed: Vec<String>,
}

fn remove(path: &Path, report: &mut Report) {
    let result = if path.is_dir() {
        std::fs::remove_dir_all(path)
    } else if path.exists() {
        std::fs::remove_file(path)
    } else {
        return;
    };
    match result {
        Ok(()) => report.removed += 1,
        Err(e) => report.failed.push(format!("{}: {e}", path.display())),
    }
}

/// Removes what the flags ask for from `profile`. Never touches settings, saved logins, open
/// tabs or the shell's other files.
pub fn clear(profile: &Path, flags: Flags) -> Report {
    let mut report = Report::default();
    if flags.history {
        for suffix in ["", "-wal", "-shm"] {
            remove(&profile.join(format!("history.db{suffix}")), &mut report);
        }
        for name in CHROMIUM_HISTORY {
            remove(&profile.join("Default").join(name), &mut report);
        }
        forget_closed_tabs(profile, &mut report);
    }
    if flags.site_data {
        for name in SITE_DATA {
            remove(&profile.join("Default").join(name), &mut report);
        }
    }
    report
}

/// The "reopen closed tab" list is history too: drop it from `session.json` (open tabs stay).
fn forget_closed_tabs(profile: &Path, report: &mut Report) {
    let path = profile.join("session.json");
    let Ok(text) = std::fs::read_to_string(&path) else {
        return;
    };
    let Ok(mut v) = serde_json::from_str::<Value>(&text) else {
        return;
    };
    if let Some(o) = v.as_object_mut()
        && o.get("closed")
            .is_some_and(|c| c.as_array().is_some_and(|a| !a.is_empty()))
    {
        o.insert("closed".into(), Value::Array(Vec::new()));
        let tmp = profile.join("session.json.tmp");
        let written = serde_json::to_string_pretty(&v)
            .map_err(|e| e.to_string())
            .and_then(|t| std::fs::write(&tmp, t).map_err(|e| e.to_string()))
            .and_then(|()| std::fs::rename(&tmp, &path).map_err(|e| e.to_string()));
        match written {
            Ok(()) => report.removed += 1,
            Err(e) => report.failed.push(format!("session.json: {e}")),
        }
    }
}

/// Call once at start. Returns true when the previous run did not end cleanly (the lock was
/// still there). Then writes the lock for this run.
pub fn mark_running(profile: &Path) -> bool {
    let crashed = profile.join(LOCK).exists();
    let _ = std::fs::create_dir_all(profile);
    let _ = std::fs::write(profile.join(LOCK), b"running");
    crashed
}

/// After a clean exit (the engine has shut down): clear per the settings and, if that fully
/// worked, remove the lock. If something couldn't be removed the lock stays, so the next start
/// tries again.
pub fn on_clean_exit(profile: &Path, flags: Flags) -> Report {
    let report = if flags.any() {
        clear(profile, flags)
    } else {
        Report::default()
    };
    if report.failed.is_empty() {
        let _ = std::fs::remove_file(profile.join(LOCK));
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_profile() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "privacy-test-{}-{}",
            std::process::id(),
            rand_suffix()
        ));
        let d = dir.join("Default");
        std::fs::create_dir_all(d.join("Network")).unwrap();
        std::fs::create_dir_all(d.join("Local Storage").join("leveldb")).unwrap();
        std::fs::create_dir_all(d.join("IndexedDB")).unwrap();
        std::fs::create_dir_all(d.join("Cache")).unwrap();
        for f in [
            "History",
            "History-journal",
            "Favicons",
            "Network/Cookies",
            "Local Storage/leveldb/000003.log",
        ] {
            std::fs::write(d.join(f), b"x").unwrap();
        }
        for f in [
            "history.db",
            "history.db-wal",
            "settings.json",
            "vault.json",
        ] {
            std::fs::write(dir.join(f), b"{}").unwrap();
        }
        std::fs::write(
            dir.join("session.json"),
            r#"{"tabs":[{"url":"https://open.example","title":"Open","opener_index":null}],"active_tab_index":0,"rail_width":56,"active_space":0,"closed":[{"url":"https://closed.example","title":"C","host":"closed.example","closed_at":1}]}"#,
        )
        .unwrap();
        dir
    }

    fn rand_suffix() -> u128 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    }

    #[test]
    fn nothing_is_cleared_when_the_switches_are_off() {
        let p = fake_profile();
        let r = on_clean_exit(&p, Flags::default());
        assert_eq!(r, Report::default());
        assert!(p.join("history.db").exists() && p.join("Default/Network/Cookies").exists());
        assert!(!p.join(LOCK).exists(), "a clean exit removes the lock");
    }

    #[test]
    fn history_switch_clears_history_but_not_site_data_or_other_files() {
        let p = fake_profile();
        let r = clear(
            &p,
            Flags {
                history: true,
                site_data: false,
            },
        );
        assert!(r.failed.is_empty(), "{:?}", r.failed);
        assert!(!p.join("history.db").exists() && !p.join("history.db-wal").exists());
        assert!(!p.join("Default/History").exists() && !p.join("Default/Favicons").exists());
        assert!(
            p.join("Default/Network/Cookies").exists(),
            "site data stays"
        );
        assert!(p.join("settings.json").exists() && p.join("vault.json").exists());
        let s: Value =
            serde_json::from_str(&std::fs::read_to_string(p.join("session.json")).unwrap())
                .unwrap();
        assert_eq!(
            s["closed"].as_array().unwrap().len(),
            0,
            "closed tabs are history"
        );
        assert_eq!(s["tabs"].as_array().unwrap().len(), 1, "open tabs stay");
    }

    #[test]
    fn site_data_switch_clears_cookies_storage_and_caches_only() {
        let p = fake_profile();
        let r = clear(
            &p,
            Flags {
                history: false,
                site_data: true,
            },
        );
        assert!(r.failed.is_empty(), "{:?}", r.failed);
        for gone in [
            "Default/Network/Cookies",
            "Default/Local Storage",
            "Default/IndexedDB",
            "Default/Cache",
        ] {
            assert!(!p.join(gone).exists(), "{gone} removed");
        }
        assert!(p.join("history.db").exists() && p.join("Default/History").exists());
    }

    #[test]
    fn a_crash_is_noticed_and_the_lock_stays_until_clearing_works() {
        let p = fake_profile();
        assert!(!mark_running(&p), "first start: no crash");
        assert!(
            mark_running(&p),
            "the lock was still there: the last run did not exit cleanly"
        );
        // clean exit with clearing on
        let r = on_clean_exit(
            &p,
            Flags {
                history: true,
                site_data: true,
            },
        );
        assert!(r.failed.is_empty());
        assert!(!p.join(LOCK).exists());
        assert!(
            !mark_running(&p),
            "after a clean exit the next start sees no crash"
        );
    }

    #[test]
    fn a_missing_profile_is_fine() {
        let p = std::env::temp_dir().join("privacy-test-does-not-exist-xyz");
        assert_eq!(
            clear(
                &p,
                Flags {
                    history: true,
                    site_data: true
                }
            ),
            Report::default()
        );
    }
}
