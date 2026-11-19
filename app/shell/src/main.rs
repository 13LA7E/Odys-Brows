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
use history::HistoryStore;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
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
    pub space: usize,
    pub pinned: bool,
}

impl Tab {
    pub fn new(view: ViewId, opener: Option<ViewId>) -> Self {
        Self {
            view,
            opener,
            space: 0,
            pinned: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ClosedTabRecord {
    pub id: u64,
    pub view: ViewId,
    pub url: String,
    pub title: String,
    pub host: String,
    pub closed_at: i64,
    pub opener: Option<ViewId>,
}

#[derive(Debug)]
struct Shell {
    tabs: Vec<Tab>,
    shown: Option<ViewId>,
    rail_width: i32,
    active_space: usize,
    is_private: bool,
    history: Option<HistoryStore>,
    tab_visit_id: HashMap<ViewId, i64>,
    tab_urls: HashMap<ViewId, String>,
    tab_titles: HashMap<ViewId, String>,
    closed_tabs: VecDeque<ClosedTabRecord>,
    next_closed_id: u64,
}

impl Default for Shell {
    fn default() -> Self {
        Self {
            tabs: Vec::new(),
            shown: None,
            rail_width: RAIL_WIDTH,
            active_space: 0,
            is_private: false,
            history: None,
            tab_visit_id: HashMap::new(),
            tab_urls: HashMap::new(),
            tab_titles: HashMap::new(),
            closed_tabs: VecDeque::new(),
            next_closed_id: 1,
        }
    }
}

impl Shell {
    fn contains(&self, view: ViewId) -> bool {
        self.tabs.iter().any(|t| t.view == view)
    }

    fn close_tab(&mut self, view: ViewId) -> Option<ClosedTabRecord> {
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
        self.tab_visit_id.remove(&view);
        if self.shown == Some(view) {
            self.shown = None;
        }

        if self.is_private {
            return None;
        }

        let url = self.tab_urls.remove(&view).unwrap_or_default();
        let title = self.tab_titles.remove(&view).unwrap_or_default();
        if url.is_empty() || url.starts_with("odyssey://") || url.starts_with("odyssey-ui://") {
            return None;
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let host = url
            .strip_prefix("https://")
            .or_else(|| url.strip_prefix("http://"))
            .unwrap_or(&url)
            .split(['/', ':', '?', '#'])
            .next()
            .unwrap_or(&url)
            .to_string();

        let id = self.next_closed_id;
        self.next_closed_id += 1;

        let record = ClosedTabRecord {
            id,
            view,
            url,
            title: if title.is_empty() {
                host.clone()
            } else {
                title
            },
            host,
            closed_at: now,
            opener: parent_opener,
        };

        self.closed_tabs.push_front(record.clone());
        // 24 hour expiry (86,400 seconds)
        self.closed_tabs.retain(|c| now - c.closed_at < 86400);

        self.persist_session();
        Some(record)
    }

    fn relight_closed_tab(&mut self, id: u64) -> Option<(String, Option<ViewId>)> {
        let pos = self.closed_tabs.iter().position(|c| c.id == id)?;
        let record = self.closed_tabs.remove(pos)?;
        Some((record.url, record.opener))
    }

    fn closed_tab_infos(&self) -> Vec<bridge::ClosedTabInfo> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        self.closed_tabs
            .iter()
            .map(|c| {
                let secs = (now - c.closed_at).max(0);
                let closed_ago = if secs < 60 {
                    "Closed just now".to_string()
                } else if secs < 3600 {
                    format!("Closed {} minutes ago", secs / 60)
                } else {
                    format!("Closed {} hours ago", secs / 3600)
                };
                bridge::ClosedTabInfo {
                    id: c.id,
                    title: c.title.clone(),
                    url: c.url.clone(),
                    host: c.host.clone(),
                    closed_ago,
                }
            })
            .collect()
    }

    fn persist_session(&self) {
        if self.is_private {
            return;
        }
        let session = self.to_session_data();
        let dir = profile_dir();
        if let Err(e) = save_session_atomic(&dir, &session) {
            eprintln!("Failed to persist session: {e}");
        }
    }

    fn to_session_data(&self) -> SessionData {
        let tabs = self
            .tabs
            .iter()
            .map(|t| {
                let url = self.tab_urls.get(&t.view).cloned().unwrap_or_default();
                let title = self.tab_titles.get(&t.view).cloned().unwrap_or_default();
                let opener_index = t
                    .opener
                    .and_then(|op| self.tabs.iter().position(|other| other.view == op));
                SavedTab {
                    url,
                    title,
                    opener_index,
                    space: t.space,
                    pinned: t.pinned,
                }
            })
            .collect();

        let active_tab_index = self
            .shown
            .and_then(|v| self.tabs.iter().position(|t| t.view == v));

        SessionData {
            tabs,
            active_tab_index,
            rail_width: self.rail_width,
            active_space: self.active_space,
        }
    }

    fn switch_space_state(&mut self, space: usize) -> Option<ViewId> {
        let space = space.min(2);
        self.active_space = space;
        let current_in_space = self
            .shown
            .and_then(|v| self.tabs.iter().find(|t| t.view == v))
            .map(|t| t.space == space)
            .unwrap_or(false);
        if !current_in_space {
            let target = self.tabs.iter().find(|t| t.space == space).map(|t| t.view);
            if target.is_some() {
                self.shown = target;
            }
            return target;
        }
        None
    }

    fn switch_space(&mut self, engine: &mut CefEngine, space: usize) {
        if let Some(target) = self.switch_space_state(space) {
            self.show(engine, target);
        }
        self.persist_session();
        engine.send_to_ui(&bridge::to_json(&ToUi::SpaceChanged {
            space: self.active_space,
        }));
    }

    fn open_pinned_site(&mut self, engine: &mut CefEngine, name: &str) {
        let url = match name.to_lowercase().as_str() {
            "mail" => "https://mail.google.com/",
            "calendar" => "https://calendar.google.com/",
            "notes" => "odyssey://notes",
            "files" => "odyssey://files",
            "maps" => "https://maps.google.com/",
            "odysseus" => "odyssey://ai",
            _ => "odyssey://pinned",
        };
        if let Some(existing) = self
            .tabs
            .iter()
            .find(|t| t.pinned && self.tab_urls.get(&t.view).map(|u| u.as_str()) == Some(url))
        {
            let view = existing.view;
            self.show(engine, view);
        } else {
            let view = engine.create_view(url);
            self.tabs.push(Tab {
                view,
                opener: None,
                space: self.active_space,
                pinned: true,
            });
            self.show(engine, view);
            self.on_url_changed(view, url);
            engine.send_to_ui(&bridge::to_json(&ToUi::ViewOpened { view, opener: None }));
        }
        self.persist_session();
    }

    fn on_url_changed(&mut self, view: ViewId, url: &str) {
        if url.is_empty()
            || url.starts_with("odyssey://")
            || url.starts_with("odyssey-ui://")
            || url.starts_with("about:")
        {
            return;
        }
        self.tab_urls.insert(view, url.to_string());
        if self.shown == Some(view) {
            engine_api::Engine::set_bounds(
                &mut CefEngine,
                view,
                page_bounds(&CefEngine, self.rail_width),
            );
            engine_api::Engine::set_visible(&mut CefEngine, view, true);
        }
        if self.is_private {
            return;
        }
        let opener_visit_id = self
            .tabs
            .iter()
            .find(|t| t.view == view)
            .and_then(|t| t.opener)
            .and_then(|op| self.tab_visit_id.get(&op).copied());
        if let Some(h) = &self.history {
            match h.record(url, "", opener_visit_id) {
                Ok(id) => {
                    self.tab_visit_id.insert(view, id);
                }
                Err(e) => eprintln!("Failed to record history visit: {e}"),
            }
        }
        self.persist_session();
    }

    fn on_title_changed(&mut self, view: ViewId, title: &str) {
        self.tab_titles.insert(view, title.to_string());
        if self.is_private {
            return;
        }
        let visit_id = match self.tab_visit_id.get(&view).copied() {
            Some(id) => id,
            None => return,
        };
        if let Some(h) = &self.history {
            let _ = h.update_title(visit_id, title);
        }
        self.persist_session();
    }

    fn show(&mut self, engine: &mut CefEngine, view: ViewId) {
        if !self.contains(view) {
            return;
        }
        if let Some(old) = self.shown.filter(|v| *v != view) {
            engine.set_visible(old, false);
        }
        let url = self
            .tab_urls
            .get(&view)
            .map(|s| s.as_str())
            .unwrap_or_default();
        if url.is_empty()
            || url == "about:blank"
            || url.starts_with("odyssey://")
            || url.starts_with("odyssey-ui://")
        {
            engine.set_visible(view, false);
        } else {
            engine.set_bounds(view, page_bounds(engine, self.rail_width));
            engine.set_visible(view, true);
        }
        self.shown = Some(view);
        self.persist_session();
    }

    fn handle(&mut self, engine: &mut CefEngine, message: FromUi) {
        let known = |s: &Shell, v: ViewId| s.contains(v);
        match message {
            FromUi::Open { url, opener } => {
                let view = engine.create_view(&url);
                self.tabs.push(Tab {
                    view,
                    opener,
                    space: self.active_space,
                    pinned: false,
                });
                self.show(engine, view);
                if !url.is_empty() {
                    self.on_url_changed(view, &url);
                }
                engine.send_to_ui(&bridge::to_json(&ToUi::ViewOpened { view, opener }));
            }
            FromUi::Navigate { view, url } => {
                let target = view.or(self.shown);
                if let Some(v) = target {
                    if known(self, v) {
                        engine.navigate(v, &url);
                        if !url.is_empty() {
                            self.on_url_changed(v, &url);
                        }
                    }
                } else if !url.is_empty() {
                    let new_view = engine.create_view(&url);
                    self.tabs.push(Tab {
                        view: new_view,
                        opener: None,
                        space: self.active_space,
                        pinned: false,
                    });
                    self.show(engine, new_view);
                    self.on_url_changed(new_view, &url);
                    engine.send_to_ui(&bridge::to_json(&ToUi::ViewOpened {
                        view: new_view,
                        opener: None,
                    }));
                }
            }
            FromUi::Back { view } => {
                if let Some(v) = view.or(self.shown).filter(|&v| known(self, v)) {
                    engine.go_back(v);
                }
            }
            FromUi::Forward { view } => {
                if let Some(v) = view.or(self.shown).filter(|&v| known(self, v)) {
                    engine.go_forward(v);
                }
            }
            FromUi::Reload { view } => {
                if let Some(v) = view.or(self.shown).filter(|&v| known(self, v)) {
                    engine.reload(v);
                }
            }
            FromUi::Stop { view } => {
                if let Some(v) = view.or(self.shown).filter(|&v| known(self, v)) {
                    engine.stop(v);
                }
            }
            FromUi::Show { view } => {
                if let Some(v) = view.or(self.shown) {
                    self.show(engine, v);
                }
            }
            FromUi::Close { view } => {
                if let Some(v) = view.or(self.shown).filter(|&v| known(self, v)) {
                    engine.close_view(v);
                }
            }
            FromUi::MinimizeWindow => engine.minimize_window(),
            FromUi::MaximizeWindow => engine.toggle_maximize_window(),
            FromUi::CloseWindow => engine.close_window(),
            FromUi::SetRailWidth { width } => {
                self.rail_width = width;
                if let Some(view) = self.shown {
                    engine.set_bounds(view, page_bounds(engine, self.rail_width));
                }
                self.persist_session();
            }
            FromUi::SaveSettings { settings } => {
                if let Some(c) = settings.get("connections").and_then(|v| v.as_u64()) {
                    segdl::set_connections(c as usize);
                }
                if !self.is_private {
                    let dir = profile_dir();
                    if let Err(e) = save_settings_atomic(&dir, &settings) {
                        eprintln!("Failed to save settings: {e}");
                    }
                }
            }
            FromUi::LoadSettings => {
                let settings = if self.is_private {
                    default_settings()
                } else {
                    let dir = profile_dir();
                    load_settings_or_default(&dir)
                };
                if let Some(c) = settings.get("connections").and_then(|v| v.as_u64()) {
                    segdl::set_connections(c as usize);
                }
                engine.send_to_ui(&bridge::to_json(&ToUi::SettingsLoaded { settings }));
                for t in &self.tabs {
                    engine.send_to_ui(&bridge::to_json(&ToUi::ViewOpened {
                        view: t.view,
                        opener: t.opener,
                    }));
                    if let Some(url) = self.tab_urls.get(&t.view) {
                        engine.send_to_ui(&bridge::to_json(&ToUi::Url {
                            view: t.view,
                            url: url.clone(),
                        }));
                    }
                    if let Some(title) = self.tab_titles.get(&t.view) {
                        engine.send_to_ui(&bridge::to_json(&ToUi::Title {
                            view: t.view,
                            title: title.clone(),
                        }));
                    }
                }
            }
            FromUi::DownloadControl { id, action } => {
                if !segdl::control(&id, &action) {
                    let _ = id
                        .parse::<u32>()
                        .map(|dl_id| engine.download_control(dl_id, &action));
                }
            }
            FromUi::OpenDownload { path } => {
                let p = PathBuf::from(&path);
                if let Err(e) = open_file(&p) {
                    eprintln!("Failed to open file {path}: {e}");
                }
            }
            FromUi::ShowInFolder { path } => {
                let p = PathBuf::from(&path);
                if let Err(e) = show_in_folder(&p) {
                    eprintln!("Failed to show in folder {path}: {e}");
                }
            }
            FromUi::QueryHistory => {
                let groups = self
                    .history
                    .as_ref()
                    .and_then(|h| h.query_day_groups().ok())
                    .unwrap_or_default();
                engine.send_to_ui(&bridge::to_json(&ToUi::HistoryLoaded { groups }));
            }
            FromUi::SearchHistory { query } => {
                let groups = self
                    .history
                    .as_ref()
                    .map(|h| {
                        let visits = h.search(&query).unwrap_or_default();
                        history::HistoryStore::group_visits(visits)
                    })
                    .unwrap_or_default();
                engine.send_to_ui(&bridge::to_json(&ToUi::HistoryLoaded { groups }));
            }
            FromUi::ForgetVisit { id } => {
                if let Some(h) = &self.history {
                    let _ = h.forget(id);
                }
                let groups = self
                    .history
                    .as_ref()
                    .and_then(|h| h.query_day_groups().ok())
                    .unwrap_or_default();
                engine.send_to_ui(&bridge::to_json(&ToUi::HistoryLoaded { groups }));
            }
            FromUi::ClearHistoryDay { day_start_secs } => {
                if let Some(h) = &self.history {
                    let _ = h.clear_day(day_start_secs);
                }
                let groups = self
                    .history
                    .as_ref()
                    .and_then(|h| h.query_day_groups().ok())
                    .unwrap_or_default();
                engine.send_to_ui(&bridge::to_json(&ToUi::HistoryLoaded { groups }));
            }
            FromUi::ClearAllHistory => {
                if let Some(h) = &self.history {
                    let _ = h.clear_all();
                }
                engine.send_to_ui(&bridge::to_json(&ToUi::HistoryLoaded {
                    groups: Vec::new(),
                }));
            }
            FromUi::RelightClosedTab { id } => {
                if let Some((url, opener)) = self.relight_closed_tab(id) {
                    let view = engine.create_view(&url);
                    self.tabs.push(Tab {
                        view,
                        opener,
                        space: self.active_space,
                        pinned: false,
                    });
                    self.show(engine, view);
                    self.on_url_changed(view, &url);
                    engine.send_to_ui(&bridge::to_json(&ToUi::ViewOpened { view, opener }));
                    let infos = self.closed_tab_infos();
                    engine.send_to_ui(&bridge::to_json(&ToUi::ClosedTabsUpdated { tabs: infos }));
                }
            }
            FromUi::ForgetClosedTab { id } => {
                self.closed_tabs.retain(|c| c.id != id);
                let infos = self.closed_tab_infos();
                engine.send_to_ui(&bridge::to_json(&ToUi::ClosedTabsUpdated { tabs: infos }));
            }
            FromUi::SetZoom { view, factor } if known(self, view) => engine.set_zoom(view, factor),
            FromUi::FindInPage {
                view,
                text,
                forward,
                match_case,
                next,
            } if known(self, view) => engine.find(view, &text, forward, match_case, next),
            FromUi::StopFinding {
                view,
                clear_selection,
            } if known(self, view) => engine.stop_finding(view, clear_selection),
            FromUi::PrintPage { view } if known(self, view) => engine.print(view),
            FromUi::OpenDevTools { view } if known(self, view) => engine.open_devtools(view),
            FromUi::SwitchSpace { space } => self.switch_space(engine, space),
            FromUi::OpenPinnedSite { name } => self.open_pinned_site(engine, &name),
            FromUi::ToggleShieldsException { host } => {
                let active = engine.toggle_shields_exception(&host);
                if let Some(v) = self.shown {
                    let count = engine.shields_count(v);
                    engine.send_to_ui(&bridge::to_json(&ToUi::ShieldsStatus {
                        view: v,
                        count,
                        enabled: active,
                    }));
                }
            }
            FromUi::GetShieldsStatus { view } => {
                if let Some(v) = view.or(self.shown) {
                    let url = self.tab_urls.get(&v).cloned().unwrap_or_default();
                    let host = url
                        .strip_prefix("https://")
                        .or_else(|| url.strip_prefix("http://"))
                        .unwrap_or(&url)
                        .split(['/', ':', '?', '#'])
                        .next()
                        .unwrap_or("");
                    let active = engine.is_shields_active_for(host);
                    let count = engine.shields_count(v);
                    engine.send_to_ui(&bridge::to_json(&ToUi::ShieldsStatus {
                        view: v,
                        count,
                        enabled: active,
                    }));
                }
            }
            FromUi::SelftestResult { ok, log } => selftest::ipc_result(ok, &log),
            other => eprintln!("bridge: ignored {other:?} (unknown view)"),
        }
    }
}

fn connect(engine: CefEngine, is_private: bool, any_selftest: bool) {
    let history_store = if is_private {
        None
    } else {
        HistoryStore::open(&profile_dir().join("history.db")).ok()
    };
    let saved_session = if is_private || any_selftest {
        SessionData::default()
    } else {
        load_session_or_default(&profile_dir())
    };
    let shell = Rc::new(RefCell::new(Shell {
        is_private,
        history: history_store,
        rail_width: saved_session.rail_width,
        active_space: saved_session.active_space,
        ..Shell::default()
    }));

    if !saved_session.tabs.is_empty() {
        let mut s = shell.borrow_mut();
        let mut created_views = Vec::new();
        for tab in &saved_session.tabs {
            let view = CefEngine.create_view(&tab.url);
            created_views.push(view);
            s.tabs.push(Tab {
                view,
                opener: None,
                space: tab.space,
                pinned: tab.pinned,
            });
            s.tab_urls.insert(view, tab.url.clone());
            s.tab_titles.insert(view, tab.title.clone());
        }
        for (i, t) in s.tabs.iter_mut().enumerate() {
            if let Some(op_idx) = saved_session.tabs[i]
                .opener_index
                .filter(|&idx| idx < created_views.len())
            {
                t.opener = Some(created_views[op_idx]);
            }
        }
        if let Some(active_idx) = saved_session
            .active_tab_index
            .filter(|&idx| idx < created_views.len())
        {
            s.show(&mut CefEngine, created_views[active_idx]);
        } else if let Some(&first) = created_views.first() {
            s.show(&mut CefEngine, first);
        }
    }

    let pending_events = Rc::new(RefCell::new(Vec::new()));
    let pending_events_sink = pending_events.clone();
    let events_shell = shell.clone();
    engine.set_event_sink(move |event| {
        let needs_shell = matches!(
            &event,
            engine_api::Event::Closed { .. }
                | engine_api::Event::UrlChanged { .. }
                | engine_api::Event::TitleChanged { .. }
        );
        if needs_shell {
            if let Ok(mut s) = events_shell.try_borrow_mut() {
                match &event {
                    engine_api::Event::Closed { view } => {
                        s.close_tab(*view);
                        let infos = s.closed_tab_infos();
                        CefEngine
                            .send_to_ui(&bridge::to_json(&ToUi::ClosedTabsUpdated { tabs: infos }));
                    }
                    engine_api::Event::UrlChanged { view, url } => {
                        s.on_url_changed(*view, url);
                    }
                    engine_api::Event::TitleChanged { view, title } => {
                        s.on_title_changed(*view, title);
                    }
                    _ => {}
                }
            } else {
                pending_events_sink.borrow_mut().push(event.clone());
            }
        }
        if let Some(message) = bridge::event_to_ui(&event) {
            CefEngine.send_to_ui(&bridge::to_json(&message));
        }
    });

    let ui_shell = shell.clone();
    let drain_pending = pending_events.clone();
    engine.set_ui_message_sink(move |json| {
        match bridge::parse(&json) {
            Ok(message) => ui_shell.borrow_mut().handle(&mut CefEngine, message),
            Err(reason) => eprintln!("bridge: ignored a malformed message: {reason}"),
        }
        loop {
            let next_event = drain_pending.borrow_mut().pop();
            match next_event {
                Some(ev) => {
                    if let Ok(mut s) = ui_shell.try_borrow_mut() {
                        match &ev {
                            engine_api::Event::Closed { view } => {
                                s.close_tab(*view);
                                let infos = s.closed_tab_infos();
                                CefEngine.send_to_ui(&bridge::to_json(&ToUi::ClosedTabsUpdated {
                                    tabs: infos,
                                }));
                            }
                            engine_api::Event::UrlChanged { view, url } => {
                                s.on_url_changed(*view, url);
                            }
                            engine_api::Event::TitleChanged { view, title } => {
                                s.on_title_changed(*view, title);
                            }
                            _ => {}
                        }
                    }
                }
                None => break,
            }
        }
    });
}

fn main() {
    let engine_selftest = std::env::args().any(|a| a == "--engine-selftest");
    let ipc_selftest = std::env::args().any(|a| a == "--ipc-selftest");
    let is_private =
        std::env::args().any(|a| a == "--private" || a == "-private" || a == "--incognito");
    let any_selftest = engine_selftest || ipc_selftest;
    let result = engine_cef::run(
        move || engine_cef::StartOptions {
            // Self-tests use a throwaway profile so they never touch real data.
            // Private windows use an in-memory profile: no cookies, cache or storage on disk (P3-02).
            profile_dir: if is_private {
                PathBuf::new()
            } else if any_selftest {
                std::env::temp_dir().join(format!("{}-selftest", brand::DATA_DIR))
            } else {
                profile_dir()
            },
            in_memory: is_private,
            ui_dir: ui_dir(),
            ui_suffix: if ipc_selftest {
                "#ipc-selftest".into()
            } else if is_private {
                "?fixture=Private".into()
            } else {
                String::new()
            },
            title: if is_private {
                format!("{} (Private)", brand::NAME)
            } else {
                brand::NAME.to_string()
            },
        },
        move |engine| {
            if engine_selftest {
                selftest::start(engine);
            } else {
                connect(engine, is_private, any_selftest);
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

/// A tab saved in session restore (P4-05, P4-07).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SavedTab {
    pub url: String,
    pub title: String,
    pub opener_index: Option<usize>,
    #[serde(default)]
    pub space: usize,
    #[serde(default)]
    pub pinned: bool,
}

/// The session state preserved across restarts or crashes (P4-05, P4-07).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionData {
    pub tabs: Vec<SavedTab>,
    pub active_tab_index: Option<usize>,
    pub rail_width: i32,
    #[serde(default)]
    pub active_space: usize,
}

impl Default for SessionData {
    fn default() -> Self {
        Self {
            tabs: Vec::new(),
            active_tab_index: None,
            rail_width: RAIL_WIDTH,
            active_space: 0,
        }
    }
}

/// Atomically write session data to disk (EDGE_CASES §1 & §6).
pub fn save_session_atomic(dir: &Path, session: &SessionData) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let target = dir.join("session.json");
    let tmp = dir.join("session.json.tmp");
    let content = serde_json::to_string_pretty(session)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(&tmp, content)?;
    std::fs::rename(&tmp, &target)?;
    Ok(())
}

/// Load session data or fall back to defaults, creating a backup if corrupted (EDGE_CASES §1).
pub fn load_session_or_default(dir: &Path) -> SessionData {
    let target = dir.join("session.json");
    if !target.exists() {
        return SessionData::default();
    }
    match std::fs::read_to_string(&target) {
        Ok(text) => match serde_json::from_str::<SessionData>(&text) {
            Ok(session) => session,
            Err(e) => {
                eprintln!("Warning: session.json corrupted ({e}), creating backup");
                let bak = dir.join("session.json.bak");
                let _ = std::fs::rename(&target, &bak);
                SessionData::default()
            }
        },
        Err(_) => SessionData::default(),
    }
}

/// Reveal a downloaded file in the system file explorer (EDGE_CASES §5).
pub fn show_in_folder(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Err("File does not exist".into());
    }
    #[cfg(windows)]
    {
        std::process::Command::new("explorer")
            .arg(format!("/select,{}", path.display()))
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let parent = path.parent().unwrap_or(path);
        std::process::Command::new("xdg-open")
            .arg(parent)
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

/// Open a downloaded file with the default associated application (EDGE_CASES §5).
pub fn open_file(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Err("File does not exist".into());
    }
    #[cfg(windows)]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", &path.to_string_lossy()])
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
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
        shell.tabs.push(Tab::new(1, None));
        shell.tabs.push(Tab::new(2, Some(1)));
        shell.tabs.push(Tab::new(3, Some(2)));
        shell.tabs.push(Tab::new(4, Some(1)));

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

    #[test]
    fn private_mode_leaves_no_files_on_disk() {
        let tmp = std::env::temp_dir().join(format!("ody_priv_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);

        let mut shell = Shell {
            is_private: true,
            ..Shell::default()
        };

        let custom = serde_json::json!({
            "stars": "Drifting",
            "accent": "Lilac"
        });
        shell.handle(&mut CefEngine, FromUi::SaveSettings { settings: custom });

        assert!(
            !tmp.exists(),
            "Private mode must not create directories or save settings to disk"
        );
    }

    #[test]
    fn private_mode_never_writes_history() {
        let mut shell = Shell {
            is_private: true,
            history: None,
            ..Shell::default()
        };

        shell.on_url_changed(1, "https://private-example.com");
        shell.on_title_changed(1, "Private Page");
        assert!(shell.tab_visit_id.is_empty());
        assert!(shell.history.is_none());
    }

    #[test]
    fn closing_tab_records_closed_tab_and_relight_restores_it() {
        let mut shell = Shell::default();
        shell.tabs.push(Tab::new(1, None));
        shell
            .tab_urls
            .insert(1, "https://tides.example/tables".into());
        shell.tab_titles.insert(1, "Tide tables".into());

        // Close the tab
        let closed = shell.close_tab(1);
        assert!(closed.is_some());
        let rec = closed.unwrap();
        assert_eq!(rec.title, "Tide tables");
        assert_eq!(rec.url, "https://tides.example/tables");
        assert_eq!(rec.host, "tides.example");
        assert_eq!(shell.closed_tabs.len(), 1);

        let infos = shell.closed_tab_infos();
        assert_eq!(infos.len(), 1);
        assert_eq!(infos[0].title, "Tide tables");
        assert_eq!(infos[0].host, "tides.example");

        // Relight closed tab
        let closed_id = rec.id;
        let restored = shell.relight_closed_tab(closed_id);
        assert!(restored.is_some());
        let (url, opener) = restored.unwrap();
        assert_eq!(url, "https://tides.example/tables");
        assert_eq!(opener, None);

        // The closed tab should now be removed from closed_tabs
        assert_eq!(shell.closed_tabs.len(), 0);
    }

    #[test]
    fn closed_tabs_expire_after_24_hours() {
        let mut shell = Shell::default();
        shell.tabs.push(Tab::new(10, None));
        shell.tab_urls.insert(10, "https://old.example".into());
        shell.tab_titles.insert(10, "Old page".into());

        shell.close_tab(10);
        assert_eq!(shell.closed_tabs.len(), 1);

        // Manually age the closed tab beyond 24 hours (86,400s + 1s)
        shell.closed_tabs[0].closed_at -= 86401;

        // When another tab is closed, expired ones are evicted
        shell.tabs.push(Tab::new(11, None));
        shell.tab_urls.insert(11, "https://new.example".into());
        shell.close_tab(11);

        assert_eq!(shell.closed_tabs.len(), 1);
        assert_eq!(shell.closed_tabs[0].url, "https://new.example");
    }

    #[test]
    fn private_mode_never_records_closed_tabs() {
        let mut shell = Shell {
            is_private: true,
            ..Shell::default()
        };
        shell.tabs.push(Tab::new(20, None));
        shell.tab_urls.insert(20, "https://secret.example".into());
        shell.tab_titles.insert(20, "Secret".into());

        let closed = shell.close_tab(20);
        assert!(closed.is_none());
        assert!(shell.closed_tabs.is_empty());
    }

    #[test]
    fn save_and_load_session_round_trip() {
        let tmp_dir = std::env::temp_dir().join(format!("ody_test_session_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp_dir);

        let session = SessionData {
            tabs: vec![
                SavedTab {
                    url: "https://example.com".into(),
                    title: "Example Domain".into(),
                    opener_index: None,
                    space: 0,
                    pinned: false,
                },
                SavedTab {
                    url: "https://sub.example.com".into(),
                    title: "Sub Page".into(),
                    opener_index: Some(0),
                    space: 1,
                    pinned: true,
                },
            ],
            active_tab_index: Some(1),
            rail_width: 264,
            active_space: 1,
        };

        assert!(save_session_atomic(&tmp_dir, &session).is_ok());
        let loaded = load_session_or_default(&tmp_dir);
        assert_eq!(loaded, session);

        let _ = std::fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn corrupted_session_recovers_gracefully_and_makes_backup() {
        let tmp_dir =
            std::env::temp_dir().join(format!("ody_test_corrupt_sess_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp_dir);
        std::fs::create_dir_all(&tmp_dir).unwrap();

        let target = tmp_dir.join("session.json");
        std::fs::write(&target, "{ corrupted session content ...").unwrap();

        let loaded = load_session_or_default(&tmp_dir);
        assert_eq!(loaded, SessionData::default());

        let bak = tmp_dir.join("session.json.bak");
        assert!(bak.exists(), "session.json.bak should be created");
        assert_eq!(
            std::fs::read_to_string(&bak).unwrap(),
            "{ corrupted session content ..."
        );

        let _ = std::fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn preserves_tab_tree_hierarchy_across_session_restore() {
        let mut shell = Shell::default();
        shell.tabs.push(Tab::new(100, None));
        shell.tab_urls.insert(100, "https://root.com".into());
        shell.tab_titles.insert(100, "Root".into());

        shell.tabs.push(Tab::new(101, Some(100)));
        shell.tab_urls.insert(101, "https://child.com".into());
        shell.tab_titles.insert(101, "Child".into());

        shell.shown = Some(101);
        shell.rail_width = 264;

        let session = shell.to_session_data();
        assert_eq!(session.tabs.len(), 2);
        assert_eq!(session.tabs[0].opener_index, None);
        assert_eq!(session.tabs[1].opener_index, Some(0));
        assert_eq!(session.active_tab_index, Some(1));
        assert_eq!(session.rail_width, 264);
        assert_eq!(session.active_space, 0);
    }

    #[test]
    fn space_switching_and_pinned_tabs() {
        let mut shell = Shell::default();
        // Space 0: Tab 1
        shell.tabs.push(Tab {
            view: 1,
            opener: None,
            space: 0,
            pinned: false,
        });
        // Space 1: Tab 2 (pinned), Tab 3
        shell.tabs.push(Tab {
            view: 2,
            opener: None,
            space: 1,
            pinned: true,
        });
        shell.tabs.push(Tab {
            view: 3,
            opener: None,
            space: 1,
            pinned: false,
        });
        shell.shown = Some(1);

        // Switch to space 1
        let target = shell.switch_space_state(1);
        assert_eq!(shell.active_space, 1);
        assert_eq!(target, Some(2));
        assert_eq!(shell.shown, Some(2));

        // Switch to space 2 (empty)
        let target2 = shell.switch_space_state(2);
        assert_eq!(shell.active_space, 2);
        assert_eq!(target2, None); // no tab in space 2 yet

        // Session data preserves space and pinned attributes
        let session = shell.to_session_data();
        assert_eq!(session.active_space, 2);
        assert_eq!(session.tabs.len(), 3);
        assert_eq!(session.tabs[1].space, 1);
        assert!(session.tabs[1].pinned);
        assert_eq!(session.tabs[2].space, 1);
        assert!(!session.tabs[2].pinned);
    }
}
