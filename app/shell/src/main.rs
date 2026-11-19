//! The browser executable. P1-01: open one window with our UI page.

// No console window in release builds on Windows.
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod brand;
mod selftest;

use std::path::{Path, PathBuf};

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

fn ui_url() -> String {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .unwrap_or_default();
    let source_ui = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("ui");
    match engine_api::find_ui_dir(&exe_dir, &source_ui) {
        Some(dir) => {
            let index = dir.join("index.html");
            engine_api::file_url(&std::fs::canonicalize(&index).unwrap_or(index))
        }
        // Never a blank screen without a word (EDGE_CASES §1).
        None => "data:text/html;charset=utf-8,<body style='background:#000;color:#aeb7bf;font:15px system-ui;padding:40px'>The browser's interface files are missing. Please reinstall.</body>".to_string(),
    }
}

fn main() {
    let selftest = std::env::args().any(|a| a == "--engine-selftest");
    let result = engine_cef::run(
        || engine_cef::StartOptions {
            // The self-test uses its own throwaway profile so it never touches real data.
            profile_dir: if selftest {
                std::env::temp_dir().join(format!("{}-selftest", brand::DATA_DIR))
            } else {
                profile_dir()
            },
            ui_url: ui_url(),
            title: brand::NAME.to_string(),
        },
        move |engine| {
            if selftest {
                selftest::start(engine);
            }
        },
    );
    match result {
        Ok(engine_cef::Role::Helper(code)) => std::process::exit(code),
        Ok(engine_cef::Role::BrowserFinished) => {
            if selftest {
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
