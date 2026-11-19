//! The browser executable: starts the engine, hosts our UI and connects the two.

// No console window in release builds on Windows.
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod brand;
mod bridge;
mod selftest;

use bridge::{FromUi, ToUi};
use engine_api::{Engine, Rect, ViewId};
use engine_cef::CefEngine;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// Collapsed rail width (DESIGN.md §6). The page area starts here.
const RAIL_WIDTH: i32 = 56;

/// Where the profile lives: `%LOCALAPPDATA%\<name>` on Windows, `$XDG_DATA_HOME/<name>` or
/// `~/.local/share/<name>` on Linux, the temp folder as a last resort.
fn profile_dir() -> PathBuf {
    let base = if cfg!(windows) {
        std::env::var_os("LOCALAPPDATA").map(PathBuf::from)
    } else {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME").map(|h| Path::new(&h).join(".local").join("share"))
            })
    };
    base.unwrap_or_else(std::env::temp_dir)
        .join(brand::DATA_DIR)
}

fn ui_dir() -> Option<PathBuf> {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .unwrap_or_default();
    let source_ui = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("ui");
    engine_api::find_ui_dir(&exe_dir, &source_ui)
}

/// The page area: right of the rail, full height (DESIGN.md §4).
fn page_bounds(engine: &CefEngine, rail_width: i32) -> Rect {
    let (w, h) = engine.client_size();
    Rect {
        x: rail_width,
        y: 0,
        width: (w - rail_width).max(1),
        height: h.max(1),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tab {
    pub view: ViewId,
    pub opener: Option<ViewId>,
}

#[derive(Debug)]
struct Shell {
    tabs: Vec<Tab>,
    shown: Option<ViewId>,
    rail_width: i32,
}

impl Default for Shell {
    fn default() -> Self {
        Self {
            tabs: Vec::new(),
            shown: None,
            rail_width: RAIL_WIDTH,
        }
    }
}

impl Shell {
    fn contains(&self, view: ViewId) -> bool {
        self.tabs.iter().any(|t| t.view == view)
    }

    fn close_tab(&mut self, view: ViewId) {
        let parent_opener = self
            .tabs
            .iter()
            .find(|t| t.view == view)
            .and_then(|t| t.opener);
        self.tabs.retain(|t| t.view != view);
        for tab in &mut self.tabs {
            if tab.opener == Some(view) {
                tab.opener = parent_opener;
            }
        }
        if self.shown == Some(view) {
            self.shown = None;
        }
    }

    fn show(&mut self, engine: &mut CefEngine, view: ViewId) {
        if !self.contains(view) {
            return;
        }
        if let Some(old) = self.shown.filter(|v| *v != view) {
            engine.set_visible(old, false);
        }
        engine.set_bounds(view, page_bounds(engine, self.rail_width));
        engine.set_visible(view, true);
        self.shown = Some(view);
    }

    fn handle(&mut self, engine: &mut CefEngine, message: FromUi) {
        let known = |s: &Shell, v: ViewId| s.contains(v);
        match message {
            FromUi::Open { url, opener } => {
                let view = engine.create_view(&url);
                self.tabs.push(Tab { view, opener });
                self.show(engine, view);
                engine.send_to_ui(&bridge::to_json(&ToUi::ViewOpened { view, opener }));
            }
            FromUi::Navigate { view, url } if known(self, view) => engine.navigate(view, &url),
            FromUi::Back { view } if known(self, view) => engine.go_back(view),
            FromUi::Forward { view } if known(self, view) => engine.go_forward(view),
            FromUi::Reload { view } if known(self, view) => engine.reload(view),
            FromUi::Stop { view } if known(self, view) => engine.stop(view),
            FromUi::Show { view } => self.show(engine, view),
            FromUi::Close { view } if known(self, view) => engine.close_view(view),
            FromUi::MinimizeWindow => engine.minimize_window(),
            FromUi::MaximizeWindow => engine.toggle_maximize_window(),
            FromUi::CloseWindow => engine.close_window(),
            FromUi::SetRailWidth { width } => {
                self.rail_width = width;
                if let Some(view) = self.shown {
                    engine.set_bounds(view, page_bounds(engine, self.rail_width));
                }
            }
            FromUi::SaveSettings { settings } => {
                let dir = profile_dir();
                if let Err(e) = save_settings_atomic(&dir, &settings) {
                    eprintln!("Failed to save settings: {e}");
                }
            }
            FromUi::LoadSettings => {
                let dir = profile_dir();
                let settings = load_settings_or_default(&dir);
                engine.send_to_ui(&bridge::to_json(&ToUi::SettingsLoaded { settings }));
            }
            FromUi::SelftestResult { ok, log } => selftest::ipc_result(ok, &log),
            other => eprintln!("bridge: ignored {other:?} (unknown view)"),
        }
    }
}

fn connect(engine: CefEngine) {
    let shell = Rc::new(RefCell::new(Shell::default()));

    let events_shell = shell.clone();
    engine.set_event_sink(move |event| {
        if let engine_api::Event::Closed { view } = &event {
            let mut s = events_shell.borrow_mut();
            s.close_tab(*view);
        }
        if let Some(message) = bridge::event_to_ui(&event) {
            CefEngine.send_to_ui(&bridge::to_json(&message));
        }
    });

    engine.set_ui_message_sink(move |json| match bridge::parse(&json) {
        Ok(message) => shell.borrow_mut().handle(&mut CefEngine, message),
        Err(reason) => eprintln!("bridge: ignored a malformed message: {reason}"),
    });
}

fn main() {
    let engine_selftest = std::env::args().any(|a| a == "--engine-selftest");
    let ipc_selftest = std::env::args().any(|a| a == "--ipc-selftest");
    let any_selftest = engine_selftest || ipc_selftest;
    let result = engine_cef::run(
        || engine_cef::StartOptions {
            // Self-tests use a throwaway profile so they never touch real data.
            profile_dir: if any_selftest {
                std::env::temp_dir().join(format!("{}-selftest", brand::DATA_DIR))
            } else {
                profile_dir()
            },
            ui_dir: ui_dir(),
            ui_suffix: if ipc_selftest {
                "#ipc-selftest".into()
            } else {
                String::new()
            },
            title: brand::NAME.to_string(),
        },
        move |engine| {
            if engine_selftest {
                selftest::start(engine);
            } else {
                connect(engine);
                if ipc_selftest {
                    selftest::ipc_timeout(engine);
                }
            }
        },
    );
    match result {
        Ok(engine_cef::Role::Helper(code)) => std::process::exit(code),
        Ok(engine_cef::Role::BrowserFinished) => {
            if any_selftest {
                std::process::exit(if selftest::result() == Some(true) {
                    0
                } else {
                    1
                });
            }
        }
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    }
}

/// Default user settings (DESIGN.md §9).
pub fn default_settings() -> serde_json::Value {
    serde_json::json!({
        "stars": "Still",
        "shootingStars": false,
        "accent": "Sea",
        "trail": "Narrow",
        "connections": 8
    })
}

/// Atomically save settings to disk (EDGE_CASES §1: atomic writes via tmp + rename).
pub fn save_settings_atomic(dir: &Path, settings: &serde_json::Value) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let target = dir.join("settings.json");
    let tmp = dir.join("settings.json.tmp");
    let content = serde_json::to_string_pretty(settings)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(&tmp, content)?;
    std::fs::rename(&tmp, &target)?;
    Ok(())
}

/// Load settings from disk or return defaults if missing or corrupted (EDGE_CASES §1).
pub fn load_settings_or_default(dir: &Path) -> serde_json::Value {
    let target = dir.join("settings.json");
    if !target.exists() {
        return default_settings();
    }
    match std::fs::read_to_string(&target) {
        Ok(text) => match serde_json::from_str::<serde_json::Value>(&text) {
            Ok(val) => val,
            Err(e) => {
                eprintln!(
                    "Warning: settings.json corrupted ({e}), creating backup and restoring defaults"
                );
                let bak = dir.join("settings.json.bak");
                let _ = std::fs::rename(&target, &bak);
                default_settings()
            }
        },
        Err(_) => default_settings(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_and_load_settings_round_trip() {
        let tmp_dir =
            std::env::temp_dir().join(format!("ody_test_settings_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp_dir);

        let custom = serde_json::json!({
            "stars": "Drifting",
            "shootingStars": true,
            "accent": "Rose",
            "trail": "Wide",
            "connections": 16
        });

        assert!(save_settings_atomic(&tmp_dir, &custom).is_ok());
        let loaded = load_settings_or_default(&tmp_dir);
        assert_eq!(loaded["accent"], "Rose");
        assert_eq!(loaded["stars"], "Drifting");
        assert_eq!(loaded["shootingStars"], true);
        assert_eq!(loaded["trail"], "Wide");
        assert_eq!(loaded["connections"], 16);

        let _ = std::fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn corrupted_settings_recovers_defaults_and_makes_backup() {
        let tmp_dir = std::env::temp_dir().join(format!("ody_test_corrupt_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp_dir);
        std::fs::create_dir_all(&tmp_dir).unwrap();

        let target = tmp_dir.join("settings.json");
        std::fs::write(&target, "{ corrupted invalid json content ...").unwrap();

        let loaded = load_settings_or_default(&tmp_dir);
        assert_eq!(loaded, default_settings());

        let bak = tmp_dir.join("settings.json.bak");
        assert!(bak.exists(), "settings.json.bak should be created");
        assert_eq!(
            std::fs::read_to_string(&bak).unwrap(),
            "{ corrupted invalid json content ..."
        );

        let _ = std::fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn closing_parent_reattaches_children_to_grandparent() {
        let mut shell = Shell::default();
        shell.tabs.push(Tab {
            view: 1,
            opener: None,
        });
        shell.tabs.push(Tab {
            view: 2,
            opener: Some(1),
        });
        shell.tabs.push(Tab {
            view: 3,
            opener: Some(2),
        });
        shell.tabs.push(Tab {
            view: 4,
            opener: Some(1),
        });

        // Close tab 2 (child of 1): tab 3 should now have opener Some(1)
        shell.close_tab(2);
        assert_eq!(shell.tabs.len(), 3);
        assert_eq!(
            shell.tabs.iter().find(|t| t.view == 3).unwrap().opener,
            Some(1)
        );
        assert_eq!(
            shell.tabs.iter().find(|t| t.view == 4).unwrap().opener,
            Some(1)
        );

        // Close tab 1 (root): tabs 3 and 4 should now have opener None (root level)
        shell.close_tab(1);
        assert_eq!(shell.tabs.len(), 2);
        assert_eq!(
            shell.tabs.iter().find(|t| t.view == 3).unwrap().opener,
            None
        );
        assert_eq!(
            shell.tabs.iter().find(|t| t.view == 4).unwrap().opener,
            None
        );
    }
}
