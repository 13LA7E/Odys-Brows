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
fn page_bounds(engine: &CefEngine) -> Rect {
    let (w, h) = engine.client_size();
    Rect {
        x: RAIL_WIDTH,
        y: 0,
        width: (w - RAIL_WIDTH).max(1),
        height: h.max(1),
    }
}

#[derive(Default)]
struct Shell {
    views: Vec<ViewId>,
    shown: Option<ViewId>,
}

impl Shell {
    fn show(&mut self, engine: &mut CefEngine, view: ViewId) {
        if !self.views.contains(&view) {
            return;
        }
        if let Some(old) = self.shown.filter(|v| *v != view) {
            engine.set_visible(old, false);
        }
        engine.set_bounds(view, page_bounds(engine));
        engine.set_visible(view, true);
        self.shown = Some(view);
    }

    fn handle(&mut self, engine: &mut CefEngine, message: FromUi) {
        let known = |s: &Shell, v: ViewId| s.views.contains(&v);
        match message {
            FromUi::Open { url } => {
                let view = engine.create_view(&url);
                self.views.push(view);
                self.show(engine, view);
                engine.send_to_ui(&bridge::to_json(&ToUi::ViewOpened { view }));
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
            s.views.retain(|v| v != view);
            if s.shown == Some(*view) {
                s.shown = None;
            }
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
