//! The browser executable: starts the engine, hosts our UI and connects the two.

// No console window in release builds on Windows.
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod auth;
mod brand;
mod bridge;
mod extensions;
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
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Collapsed rail width (DESIGN.md §6). The page area starts here.
const RAIL_WIDTH: i32 = 56;

/// Where the profile lives: `%LOCALAPPDATA%\<name>` on Windows, `$XDG_DATA_HOME/<name>` or
/// `~/.local/share/<name>` on Linux, the temp folder as a last resort.
static PROFILE_OVERRIDE: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

fn profile_dir() -> PathBuf {
    // `--profile-dir=` moves the shell's own files (settings, session, history, vault) too.
    if let Some(p) = PROFILE_OVERRIDE.get() {
        return p.clone();
    }
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
    pub sleeping: bool,
    pub has_audio: bool,
    pub has_form_input: bool,
    pub last_active: i64,
    /// Restored from the last session but not loaded yet; loads when first shown (lazy restore).
    pub restore_pending: bool,
}

impl Tab {
    pub fn new(view: ViewId, opener: Option<ViewId>) -> Self {
        Self::with_details(view, opener, 0, false)
    }

    pub fn with_details(view: ViewId, opener: Option<ViewId>, space: usize, pinned: bool) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        Self {
            view,
            opener,
            space,
            pinned,
            sleeping: false,
            has_audio: false,
            has_form_input: false,
            last_active: now,
            restore_pending: false,
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

const AI_KEY_NAME: &str = "ai-api-key";

fn load_ai_config(dir: &Path) -> ai::AiConfig {
    let path = dir.join("ai.json");
    let mut config: ai::AiConfig = std::fs::read_to_string(path)
        .ok()
        .and_then(|data| serde_json::from_str(&data).ok())
        .unwrap_or_default();
    // The key lives in the OS secure store, never in ai.json (D-014).
    config.api_key = vault::secrets::get(AI_KEY_NAME).ok().flatten();
    config
}

/// Applies a config sent by the UI. A save with no key keeps the stored key; an explicit
/// empty string clears it. Returns an error text if the secure store failed.
fn apply_ai_config(
    current: &ai::AiConfig,
    mut incoming: ai::AiConfig,
) -> (ai::AiConfig, Option<String>) {
    let mut error = None;
    match incoming.api_key.take() {
        None => incoming.api_key = current.api_key.clone(),
        Some(k) if k.trim().is_empty() => {
            if let Err(e) = vault::secrets::delete(AI_KEY_NAME) {
                error = Some(e);
            }
        }
        Some(k) => match vault::secrets::set(AI_KEY_NAME, k.trim()) {
            Ok(()) => incoming.api_key = Some(k.trim().to_string()),
            Err(e) => {
                error = Some(e);
                incoming.api_key = current.api_key.clone();
            }
        },
    }
    (incoming, error)
}

fn save_ai_config(dir: &Path, config: &ai::AiConfig) {
    let path = dir.join("ai.json");
    if let Ok(data) = serde_json::to_string_pretty(config) {
        let _ = std::fs::write(path, data);
    }
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
    extensions: extensions::ExtensionsStore,
    vault: vault::VaultStore,
    ai_client: Arc<ai::AiClient>,
    ai_config: ai::AiConfig,
    ai_cancel: Arc<AtomicBool>,
    zoom_index: HashMap<ViewId, usize>,
    download_paths: HashMap<u32, String>,
    /// Downloads in progress and the tab that started them (those tabs don't sleep).
    active_downloads: HashMap<u32, Option<ViewId>>,
    /// The URL last written to history per tab, so one navigation isn't recorded twice.
    last_recorded: HashMap<ViewId, String>,
    /// "Save password?" questions waiting for an answer. The password stays here, in Rust.
    pending_logins: HashMap<u64, PendingLogin>,
    /// "Which login?" questions: request -> (tab, site key).
    pending_fills: HashMap<u64, (ViewId, String)>,
    next_request: u64,
    last_persist: std::cell::Cell<Option<std::time::Instant>>,
    persist_dirty: std::cell::Cell<bool>,
}

/// A login the page reported, waiting for the user to say Save, Never or Not now.
#[derive(Debug)]
struct PendingLogin {
    view: ViewId,
    host: String,
    username: String,
    password: String,
    /// Set when the site and username are saved already with a different password.
    update_id: Option<String>,
}

/// Autofill only on https, or on this machine (a plain http site could be read by others).
fn autofill_allowed(url: &str) -> bool {
    if url.starts_with("https://") {
        return true;
    }
    let host = vault::host_key(url);
    url.starts_with("http://")
        && (host.starts_with("localhost")
            || host.starts_with("127.0.0.1")
            || host.starts_with("[::1]"))
}

fn setting_on(name: &str) -> bool {
    load_settings_or_default(&profile_dir())
        .get(name)
        .and_then(|v| v.as_bool())
        .unwrap_or(true)
}

/// Chrome's zoom steps (percent / 100).
const ZOOM_LEVELS: [f64; 15] = [
    0.25, 0.33, 0.5, 0.67, 0.75, 0.8, 0.9, 1.0, 1.1, 1.25, 1.5, 1.75, 2.0, 2.5, 3.0,
];
const ZOOM_DEFAULT: usize = 7;

impl Default for Shell {
    fn default() -> Self {
        let dir = profile_dir();
        let ai_cfg = load_ai_config(&dir);
        let vault_store = vault::VaultStore::load(&dir.join("vault.json"));
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
            extensions: extensions::ExtensionsStore::default(),
            vault: vault_store,
            ai_client: Arc::new(ai::AiClient::new()),
            ai_config: ai_cfg,
            ai_cancel: Arc::new(AtomicBool::new(false)),
            zoom_index: HashMap::new(),
            download_paths: HashMap::new(),
            active_downloads: HashMap::new(),
            last_recorded: HashMap::new(),
            pending_logins: HashMap::new(),
            pending_fills: HashMap::new(),
            next_request: 0,
            last_persist: std::cell::Cell::new(None),
            persist_dirty: std::cell::Cell::new(false),
        }
    }
}

impl Shell {
    /// Moves one Chrome zoom step (`dir` < 0 out, > 0 in, 0 reset) and returns the factor.
    fn zoom_step(&mut self, view: ViewId, dir: i32) -> f64 {
        let cur = self.zoom_index.get(&view).copied().unwrap_or(ZOOM_DEFAULT);
        let next = match dir {
            0 => ZOOM_DEFAULT,
            d if d > 0 => (cur + 1).min(ZOOM_LEVELS.len() - 1),
            _ => cur.saturating_sub(1),
        };
        self.zoom_index.insert(view, next);
        ZOOM_LEVELS[next]
    }

    fn space_tabs(&self) -> Vec<ViewId> {
        self.tabs
            .iter()
            .filter(|t| t.space == self.active_space)
            .map(|t| t.view)
            .collect()
    }

    fn vault_path() -> PathBuf {
        profile_dir().join("vault.json")
    }

    fn send_vault(&self) {
        CefEngine.send_to_ui(&bridge::to_json(&ToUi::VaultLoaded {
            entries: self.vault.list(),
            notice: self.vault.load_notice.clone(),
            never: self.vault.never.clone(),
        }));
    }

    /// The page reported a submitted login. Offer to save or update it (Brave/Chrome style).
    fn on_login_submitted(&mut self, view: ViewId, username: &str, password: &str) {
        if self.is_private
            || password.is_empty()
            || password.len() > 1024
            || username.len() > 512
            || !setting_on("offerSavePasswords")
        {
            return;
        }
        // The site comes from our own record of the tab, never from the page.
        let Some(url) = self.tab_urls.get(&view).cloned() else {
            return;
        };
        if !(url.starts_with("https://") || url.starts_with("http://")) {
            return;
        }
        let host = vault::host_key(&url);
        if host.is_empty() || self.vault.is_never(&host) {
            return;
        }
        let update_id = match self.vault.find_login(&host, username) {
            Some(c) if c.password == password => return,
            Some(c) => Some(c.id),
            None => None,
        };
        self.pending_logins.retain(|_, p| p.view != view);
        self.next_request += 1;
        let request = self.next_request;
        self.pending_logins.insert(
            request,
            PendingLogin {
                view,
                host: host.clone(),
                username: username.to_string(),
                password: password.to_string(),
                update_id: update_id.clone(),
            },
        );
        CefEngine.send_to_ui(&bridge::to_json(&ToUi::SaveLoginPrompt {
            view,
            request,
            host,
            username: username.to_string(),
            update: update_id.is_some(),
        }));
    }

    fn answer_save_login(&mut self, request: u64, action: &str) {
        let Some(p) = self.pending_logins.remove(&request) else {
            return;
        };
        match action {
            "save" => {
                let result = match &p.update_id {
                    Some(id) => self.vault.update(id, None, None, Some(p.password.clone())),
                    None => self
                        .vault
                        .add(&p.host, &p.username, &p.password)
                        .map(|_| ()),
                };
                if let Err(e) = result {
                    eprintln!("Couldn't save the login: {e}");
                    return;
                }
            }
            "never" => self.vault.add_never(&p.host),
            _ => return,
        }
        if let Err(e) = self.vault.save(&Self::vault_path()) {
            eprintln!("Couldn't write saved logins: {e}");
        }
        self.send_vault();
    }

    /// The user clicked a login field: fill the saved login, or ask which one.
    fn on_fill_requested(&mut self, view: ViewId) {
        if !setting_on("autofillPasswords") {
            return;
        }
        let Some(url) = self.tab_urls.get(&view).cloned() else {
            return;
        };
        if !autofill_allowed(&url) {
            return;
        }
        let host = vault::host_key(&url);
        let creds = self.vault.credentials_for_host(&host).unwrap_or_default();
        match creds.len() {
            0 => {}
            1 => CefEngine.fill_login(view, &creds[0].username, &creds[0].password),
            _ => {
                self.next_request += 1;
                let request = self.next_request;
                self.pending_fills.retain(|_, (v, _)| *v != view);
                self.pending_fills.insert(request, (view, host.clone()));
                CefEngine.send_to_ui(&bridge::to_json(&ToUi::FillChoices {
                    view,
                    request,
                    host,
                    logins: creds
                        .iter()
                        .map(|c| bridge::FillChoice {
                            id: c.id.clone(),
                            username: c.username.clone(),
                        })
                        .collect(),
                }));
            }
        }
    }

    fn answer_fill(&mut self, request: u64, id: Option<&str>) {
        let Some((view, host)) = self.pending_fills.remove(&request) else {
            return;
        };
        let Some(id) = id else { return };
        // The tab must still be on the same site as when the question was asked.
        let still = self
            .tab_urls
            .get(&view)
            .is_some_and(|u| vault::host_key(u) == host && autofill_allowed(u));
        if !still {
            return;
        }
        if let Ok(creds) = self.vault.credentials_for_host(&host)
            && let Some(c) = creds.iter().find(|c| c.id == id)
        {
            CefEngine.fill_login(view, &c.username, &c.password);
        }
    }

    fn is_restore_pending(&self, view: ViewId) -> bool {
        self.tabs
            .iter()
            .any(|t| t.view == view && t.restore_pending)
    }

    fn is_sleeping(&self, view: ViewId) -> bool {
        self.tabs.iter().any(|t| t.view == view && t.sleeping)
    }

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
        self.last_recorded.remove(&view);
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
        self.expire_closed();
        let pos = self.closed_tabs.iter().position(|c| c.id == id)?;
        let record = self.closed_tabs.remove(pos)?;
        Some((record.url, record.opener))
    }

    fn relight_last_closed(&mut self) -> Option<(String, Option<ViewId>)> {
        self.expire_closed();
        let record = self.closed_tabs.pop_front()?;
        Some((record.url, record.opener))
    }

    fn expire_closed(&mut self) {
        let now = unix_now();
        self.closed_tabs.retain(|c| now - c.closed_at < 86400);
    }

    /// Closed tabs saved in the last session (those under 24 hours old).
    fn restore_closed(&mut self, saved: &[SavedClosed]) {
        let now = unix_now();
        for c in saved.iter().filter(|c| now - c.closed_at < 86400) {
            let id = self.next_closed_id;
            self.next_closed_id += 1;
            self.closed_tabs.push_back(ClosedTabRecord {
                id,
                view: 0,
                url: c.url.clone(),
                title: c.title.clone(),
                host: c.host.clone(),
                closed_at: c.closed_at,
                opener: None,
            });
        }
    }

    fn closed_tab_infos(&self) -> Vec<bridge::ClosedTabInfo> {
        let now = unix_now();
        self.closed_tabs
            .iter()
            .filter(|c| now - c.closed_at < 86400)
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

    /// Marks the session changed and writes it at most about once a second; `flush_session`
    /// (called by a timer) writes what is still pending.
    fn persist_session(&self) {
        if self.is_private {
            return;
        }
        self.persist_dirty.set(true);
        let due = self
            .last_persist
            .get()
            .is_none_or(|t| t.elapsed() >= std::time::Duration::from_secs(1));
        if due {
            self.flush_session();
        }
    }

    fn flush_session(&self) {
        if self.is_private || !self.persist_dirty.get() {
            return;
        }
        self.persist_dirty.set(false);
        self.last_persist.set(Some(std::time::Instant::now()));
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

        let now = unix_now();
        let closed = self
            .closed_tabs
            .iter()
            .filter(|c| now - c.closed_at < 86400)
            .map(|c| SavedClosed {
                url: c.url.clone(),
                title: c.title.clone(),
                host: c.host.clone(),
                closed_at: c.closed_at,
            })
            .collect();
        SessionData {
            tabs,
            active_tab_index,
            rail_width: self.rail_width,
            active_space: self.active_space,
            closed,
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
            self.tabs
                .push(Tab::with_details(view, None, self.active_space, true));
            self.show(engine, view);
            self.on_url_changed(view, url);
            engine.send_to_ui(&bridge::to_json(&ToUi::ViewOpened {
                view,
                opener: None,
                url: Some(url.to_string()),
            }));
        }
        self.persist_session();
    }

    fn on_url_changed(&mut self, view: ViewId, url: &str) {
        if self.is_sleeping(view) || (self.is_restore_pending(view) && url.starts_with("about:")) {
            // The engine loads about:blank to free memory; the real URL stays ours.
            return;
        }
        if url.starts_with("about:")
            && self
                .tab_urls
                .get(&view)
                .is_some_and(|u| u.starts_with("odyssey://"))
        {
            // The blank page behind an internal page; the tab's address stays odyssey://.
            return;
        }
        if let Some(name) = url.strip_prefix("odyssey://") {
            let title = internal_page_title(name);
            self.tab_titles.insert(view, title.clone());
            CefEngine.send_to_ui(&bridge::to_json(&ToUi::Title { view, title }));
        }
        if !url.is_empty() {
            self.tab_urls.insert(view, url.to_string());
            CefEngine.send_to_ui(&bridge::to_json(&ToUi::Url {
                view,
                url: url.to_string(),
            }));
        }
        if url.is_empty()
            || url.starts_with("odyssey://")
            || url.starts_with("odyssey-ui://")
            || url.starts_with("about:")
        {
            if self.shown == Some(view) {
                engine_api::Engine::set_visible(&mut CefEngine, view, false);
            }
            return;
        }
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
        if self.last_recorded.get(&view).map(String::as_str) == Some(url) {
            return;
        }
        self.last_recorded.insert(view, url.to_string());
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
        if self.is_sleeping(view) || self.is_restore_pending(view) {
            return;
        }
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
        let pending_url = self
            .tabs
            .iter_mut()
            .find(|t| t.view == view && t.restore_pending)
            .map(|t| {
                t.restore_pending = false;
                t.view
            })
            .and_then(|v| self.tab_urls.get(&v).cloned())
            .filter(|u| {
                u.starts_with("http://") || u.starts_with("https://") || u.starts_with("file:")
            });
        if let Some(url) = pending_url {
            engine.navigate(view, &url);
        }
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.view == view) {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            tab.last_active = now;
            if tab.sleeping {
                tab.sleeping = false;
                engine.wake(view);
                engine.send_to_ui(&bridge::to_json(&ToUi::TabSleeping {
                    view,
                    sleeping: false,
                }));
            }
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

    fn check_tab_sleeping(&mut self, engine: &mut CefEngine, timeout_secs: i64) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let downloading: Vec<ViewId> = self.active_downloads.values().flatten().copied().collect();
        for tab in &mut self.tabs {
            // Edge cases §9: Never sleep active tab, tabs playing audio, pinned tabs, tabs with
            // unsaved form input, tabs with a download in progress, or already sleeping
            if Some(tab.view) == self.shown
                || tab.pinned
                || tab.has_audio
                || tab.has_form_input
                || downloading.contains(&tab.view)
                || tab.sleeping
            {
                continue;
            }
            if now - tab.last_active >= timeout_secs {
                tab.sleeping = true;
                engine.sleep(tab.view);
                engine.send_to_ui(&bridge::to_json(&ToUi::TabSleeping {
                    view: tab.view,
                    sleeping: true,
                }));
            }
        }
    }

    /// Creates a tab. A background tab is created without being shown.
    fn open_tab(
        &mut self,
        engine: &mut CefEngine,
        url: &str,
        opener: Option<ViewId>,
        background: bool,
    ) {
        let view = engine.create_view(if url.starts_with("odyssey://") {
            ""
        } else {
            url
        });
        self.tabs
            .push(Tab::with_details(view, opener, self.active_space, false));
        if !url.is_empty() {
            self.tab_urls.insert(view, url.to_string());
        }
        engine.send_to_ui(&bridge::to_json(&ToUi::ViewOpened {
            view,
            opener,
            url: (!url.is_empty()).then(|| url.to_string()),
        }));
        if background {
            engine.set_visible(view, false);
        } else {
            self.show(engine, view);
            engine.send_to_ui(&bridge::to_json(&ToUi::ViewShown { view }));
        }
        if !url.is_empty() {
            self.on_url_changed(view, url);
        }
        self.persist_session();
    }

    fn handle(&mut self, engine: &mut CefEngine, message: FromUi) {
        let known = |s: &Shell, v: ViewId| s.contains(v);
        match message {
            FromUi::Open { url, opener } => {
                // Internal pages are single: reuse the open one instead of adding another.
                if url.starts_with("odyssey://") {
                    let existing = self
                        .tabs
                        .iter()
                        .find(|t| {
                            self.tab_urls.get(&t.view).map(String::as_str) == Some(url.as_str())
                        })
                        .map(|t| t.view);
                    if let Some(v) = existing {
                        self.show(engine, v);
                        engine.send_to_ui(&bridge::to_json(&ToUi::ViewShown { view: v }));
                        return;
                    }
                }
                let view = engine.create_view(if url.starts_with("odyssey://") {
                    ""
                } else {
                    &url
                });
                self.tabs
                    .push(Tab::with_details(view, opener, self.active_space, false));
                if !url.is_empty() {
                    self.tab_urls.insert(view, url.clone());
                }
                self.show(engine, view);
                engine.send_to_ui(&bridge::to_json(&ToUi::ViewOpened {
                    view,
                    opener,
                    url: if url.is_empty() {
                        None
                    } else {
                        Some(url.clone())
                    },
                }));
                if !url.is_empty() {
                    self.on_url_changed(view, &url);
                }
            }
            FromUi::Navigate { view, url } => {
                let target = view.or(self.shown);
                if let Some(v) = target {
                    if known(self, v) {
                        if !url.is_empty() {
                            self.tab_urls.insert(v, url.clone());
                        }
                        if url.starts_with("odyssey://")
                            || url.starts_with("about:")
                            || url.is_empty()
                        {
                            // Replace the page so it stops running (audio, timers).
                            engine.navigate(v, "about:blank");
                            engine.set_visible(v, false);
                            self.on_url_changed(v, &url);
                        } else {
                            engine.navigate(v, &url);
                            self.show(engine, v);
                            self.on_url_changed(v, &url);
                        }
                    }
                } else if !url.is_empty() {
                    let new_view = engine.create_view(&url);
                    self.tabs
                        .push(Tab::with_details(new_view, None, self.active_space, false));
                    self.tab_urls.insert(new_view, url.clone());
                    self.show(engine, new_view);
                    engine.send_to_ui(&bridge::to_json(&ToUi::ViewOpened {
                        view: new_view,
                        opener: None,
                        url: Some(url.clone()),
                    }));
                    self.on_url_changed(new_view, &url);
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
                let target = view.or(self.shown);
                if let Some(v) = target
                    && known(self, v)
                {
                    self.show(engine, v);
                    engine.send_to_ui(&bridge::to_json(&ToUi::ViewShown { view: v }));
                }
            }
            FromUi::NextTab => {
                let ids = self.space_tabs();
                if !ids.is_empty() {
                    let cur_idx = self
                        .shown
                        .and_then(|cur| ids.iter().position(|v| *v == cur))
                        .unwrap_or(0);
                    let target = ids[(cur_idx + 1) % ids.len()];
                    self.show(engine, target);
                    engine.send_to_ui(&bridge::to_json(&ToUi::ViewShown { view: target }));
                }
            }
            FromUi::PrevTab => {
                let ids = self.space_tabs();
                if !ids.is_empty() {
                    let cur_idx = self
                        .shown
                        .and_then(|cur| ids.iter().position(|v| *v == cur))
                        .unwrap_or(0);
                    let target = ids[(cur_idx + ids.len() - 1) % ids.len()];
                    self.show(engine, target);
                    engine.send_to_ui(&bridge::to_json(&ToUi::ViewShown { view: target }));
                }
            }
            FromUi::Close { view } => {
                if let Some(v) = view.or(self.shown).filter(|&v| known(self, v)) {
                    engine.close_view(v);
                }
            }
            FromUi::OpenPrivateWindow => open_private_window(),
            FromUi::FocusWindow => engine.focus_ui(),
            FromUi::AnswerSaveLogin { request, action } => self.answer_save_login(request, &action),
            FromUi::AnswerFill { request, id } => self.answer_fill(request, id.as_deref()),
            FromUi::VaultRemoveNever { host } => {
                self.vault.remove_never(&host);
                let _ = self.vault.save(&Self::vault_path());
                self.send_vault();
            }
            FromUi::VaultExport => {
                let csv = match self.vault.export_csv() {
                    Ok(c) => c,
                    Err(e) => {
                        engine.send_to_ui(&bridge::to_json(&ToUi::VaultExported {
                            ok: false,
                            message: e,
                            path: String::new(),
                        }));
                        return;
                    }
                };
                let hwnd = engine.main_window_handle();
                std::thread::spawn(move || {
                    let result = auth::verify(hwnd, "Export saved passwords");
                    let msg = match result {
                        auth::Auth::Verified => {
                            let dir = segdl::default_download_dir();
                            let name = format!("constellations-passwords-{}.csv", unix_now());
                            let path = segdl::unique_path(&dir, &name);
                            match std::fs::write(&path, csv) {
                                Ok(()) => ToUi::VaultExported {
                                    ok: true,
                                    message: String::new(),
                                    path: path.to_string_lossy().to_string(),
                                },
                                Err(e) => ToUi::VaultExported {
                                    ok: false,
                                    message: e.to_string(),
                                    path: String::new(),
                                },
                            }
                        }
                        auth::Auth::Cancelled => ToUi::VaultExported {
                            ok: false,
                            message: "Not verified".to_string(),
                            path: String::new(),
                        },
                        auth::Auth::Unavailable(m) => ToUi::VaultExported {
                            ok: false,
                            message: m,
                            path: String::new(),
                        },
                    };
                    CefEngine.post_ui(move || CefEngine.send_to_ui(&bridge::to_json(&msg)));
                });
            }
            FromUi::VaultImport { csv } => {
                let msg = match self.vault.import_csv(&csv) {
                    Ok(summary) => {
                        if let Err(e) = self.vault.save(&Self::vault_path()) {
                            eprintln!("Couldn't write saved logins: {e}");
                        }
                        ToUi::VaultImported {
                            added: summary.added,
                            skipped: summary.skipped,
                            error: None,
                        }
                    }
                    Err(e) => ToUi::VaultImported {
                        added: 0,
                        skipped: 0,
                        error: Some(e),
                    },
                };
                engine.send_to_ui(&bridge::to_json(&msg));
                self.send_vault();
            }
            FromUi::SetNotchHeight { height } => {
                engine.set_notch_height(height.clamp(0.0, 200.0) as f32)
            }
            FromUi::SetOverlayRects { rects } => {
                let rects = rects
                    .iter()
                    .take(16)
                    .filter(|r| r.width > 0.0 && r.height > 0.0)
                    .map(|r| {
                        (
                            Rect {
                                x: r.x.floor() as i32,
                                y: r.y.floor() as i32,
                                width: r.width.ceil() as i32,
                                height: r.height.ceil() as i32,
                            },
                            r.radius.round() as i32,
                        )
                    })
                    .collect();
                engine.set_overlay_rects(rects);
            }
            FromUi::AnswerPermission { request, allow } => engine.answer_permission(request, allow),
            FromUi::AnswerCertificate { request, proceed } => {
                engine.answer_certificate_error(request, proceed)
            }
            FromUi::MinimizeWindow => engine.minimize_window(),
            FromUi::MaximizeWindow => engine.toggle_maximize_window(),
            FromUi::CloseWindow => engine.close_window(),
            FromUi::SetRailWidth { width } => {
                if width == self.rail_width {
                    return;
                }
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
                        url: self.tab_urls.get(&t.view).cloned(),
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
                // The UI treats the last replayed tab as active; tell it which one really is.
                if let Some(v) = self.shown {
                    engine.send_to_ui(&bridge::to_json(&ToUi::ViewShown { view: v }));
                }
            }
            FromUi::DownloadControl { id, action } => {
                // The UI sends the engine's numeric id; our multi-connection downloader is
                // keyed by the file path, so look the path up first (AUDIT-05).
                match id.parse::<u32>() {
                    Ok(n) => {
                        let handled = self
                            .download_paths
                            .get(&n)
                            .is_some_and(|p| segdl::control(p, &action));
                        if !handled {
                            engine.download_control(n, &action);
                        }
                    }
                    Err(_) => {
                        segdl::control(&id, &action);
                    }
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
                let found = match id {
                    Some(cid) => self.relight_closed_tab(cid),
                    None => self.relight_last_closed(),
                };
                if let Some((url, opener)) = found {
                    let view = engine.create_view(if url.starts_with("odyssey://") {
                        ""
                    } else {
                        &url
                    });
                    self.tabs
                        .push(Tab::with_details(view, opener, self.active_space, false));
                    self.show(engine, view);
                    self.on_url_changed(view, &url);
                    engine.send_to_ui(&bridge::to_json(&ToUi::ViewOpened {
                        view,
                        opener,
                        url: Some(url),
                    }));
                    let infos = self.closed_tab_infos();
                    engine.send_to_ui(&bridge::to_json(&ToUi::ClosedTabsUpdated { tabs: infos }));
                }
            }
            FromUi::ForgetClosedTab { id } => {
                self.closed_tabs.retain(|c| c.id != id);
                let infos = self.closed_tab_infos();
                engine.send_to_ui(&bridge::to_json(&ToUi::ClosedTabsUpdated { tabs: infos }));
            }
            FromUi::SetZoom { view, factor } if known(self, view) => {
                if let Some(i) = ZOOM_LEVELS.iter().position(|z| (z - factor).abs() < 0.01) {
                    self.zoom_index.insert(view, i);
                }
                engine.set_zoom(view, factor)
            }
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
            FromUi::SetTabSleeping { view, sleeping } => {
                if let Some(tab) = self.tabs.iter_mut().find(|t| t.view == view) {
                    if sleeping
                        && Some(view) != self.shown
                        && !tab.pinned
                        && !tab.has_audio
                        && !tab.has_form_input
                    {
                        tab.sleeping = true;
                        engine.sleep(view);
                        engine.send_to_ui(&bridge::to_json(&ToUi::TabSleeping {
                            view,
                            sleeping: true,
                        }));
                    } else if !sleeping {
                        tab.sleeping = false;
                        engine.wake(view);
                        engine.send_to_ui(&bridge::to_json(&ToUi::TabSleeping {
                            view,
                            sleeping: false,
                        }));
                    }
                }
            }
            FromUi::GetExtensions => {
                let list = self.extensions.list();
                engine.send_to_ui(&bridge::to_json(&ToUi::ExtensionsLoaded {
                    extensions: list,
                }));
            }
            FromUi::ToggleExtension { id, enabled } => {
                self.extensions.toggle(&id, enabled);
                let _ = self.extensions.save(&profile_dir());
                let list = self.extensions.list();
                engine.send_to_ui(&bridge::to_json(&ToUi::ExtensionsLoaded {
                    extensions: list,
                }));
            }
            FromUi::RemoveExtension { id } => {
                self.extensions.remove(&id);
                let _ = self.extensions.save(&profile_dir());
                let list = self.extensions.list();
                engine.send_to_ui(&bridge::to_json(&ToUi::ExtensionsLoaded {
                    extensions: list,
                }));
            }
            FromUi::LoadUnpackedExtension { path } => {
                let p = PathBuf::from(&path);
                if self.extensions.load_unpacked(&p).is_ok() {
                    let _ = self.extensions.save(&profile_dir());
                    let list = self.extensions.list();
                    engine.send_to_ui(&bridge::to_json(&ToUi::ExtensionsLoaded {
                        extensions: list,
                    }));
                }
            }
            FromUi::AiGetConfig => {
                let ui_cfg = ai::AiSettingsUi::from(&self.ai_config);
                engine.send_to_ui(&bridge::to_json(&ToUi::AiConfigLoaded { config: ui_cfg }));
            }
            FromUi::AiSaveConfig { config } => {
                let (cfg, err) = apply_ai_config(&self.ai_config, config);
                if let Some(e) = err {
                    eprintln!("AI key: {e}");
                }
                self.ai_config = cfg;
                if !self.is_private {
                    save_ai_config(&profile_dir(), &self.ai_config);
                }
                let ui_cfg = ai::AiSettingsUi::from(&self.ai_config);
                engine.send_to_ui(&bridge::to_json(&ToUi::AiConfigLoaded { config: ui_cfg }));
            }
            FromUi::AiTestConnection { mut config } => {
                if config.api_key.as_deref().is_none_or(|k| k.is_empty()) {
                    config.api_key = self.ai_config.api_key.clone();
                }
                let client = self.ai_client.clone();
                std::thread::spawn(move || {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .unwrap();
                    let res = rt.block_on(async { client.test_connection(&config).await });
                    CefEngine.post_ui(move || match res {
                        Ok(msg) => CefEngine.send_to_ui(&bridge::to_json(&ToUi::AiTestResult {
                            ok: true,
                            message: msg,
                        })),
                        Err(e) => CefEngine.send_to_ui(&bridge::to_json(&ToUi::AiTestResult {
                            ok: false,
                            message: e,
                        })),
                    });
                });
            }
            FromUi::AiCancel => {
                self.ai_cancel.store(true, Ordering::Relaxed);
            }
            FromUi::AiAsk { prompt, context } => {
                self.ai_cancel.store(true, Ordering::Relaxed);
                let cancel = Arc::new(AtomicBool::new(false));
                self.ai_cancel = cancel.clone();
                let client = self.ai_client.clone();
                let config = self.ai_config.clone();

                std::thread::spawn(move || {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .unwrap();
                    rt.block_on(async {
                        client
                            .ask(&config, &prompt, context.as_deref(), cancel, move |ev| {
                                CefEngine.post_ui(move || {
                                    CefEngine
                                        .send_to_ui(&bridge::to_json(&ToUi::AiEvent { event: ev }));
                                });
                            })
                            .await;
                    });
                });
            }
            FromUi::VaultList => {
                let entries = self.vault.list();
                engine.send_to_ui(&bridge::to_json(&ToUi::VaultLoaded {
                    entries,
                    notice: self.vault.load_notice.clone(),
                    never: self.vault.never.clone(),
                }));
            }
            FromUi::VaultListForOrigin { origin } => {
                let entries = self.vault.list_for_origin(&origin);
                engine.send_to_ui(&bridge::to_json(&ToUi::VaultLoaded {
                    entries,
                    notice: self.vault.load_notice.clone(),
                    never: self.vault.never.clone(),
                }));
            }
            FromUi::VaultAdd {
                origin,
                username,
                password,
            } => {
                if self.vault.add(&origin, &username, &password).is_ok() {
                    let _ = self.vault.save(&profile_dir().join("vault.json"));
                    let entries = self.vault.list();
                    engine.send_to_ui(&bridge::to_json(&ToUi::VaultLoaded {
                        entries,
                        notice: self.vault.load_notice.clone(),
                        never: self.vault.never.clone(),
                    }));
                }
            }
            FromUi::VaultUpdate {
                id,
                origin,
                username,
                password,
            } => {
                if self.vault.update(&id, origin, username, password).is_ok() {
                    let _ = self.vault.save(&profile_dir().join("vault.json"));
                    let entries = self.vault.list();
                    engine.send_to_ui(&bridge::to_json(&ToUi::VaultLoaded {
                        entries,
                        notice: self.vault.load_notice.clone(),
                        never: self.vault.never.clone(),
                    }));
                }
            }
            FromUi::VaultDelete { id } => {
                if self.vault.delete(&id).is_ok() {
                    let _ = self.vault.save(&profile_dir().join("vault.json"));
                    let entries = self.vault.list();
                    engine.send_to_ui(&bridge::to_json(&ToUi::VaultLoaded {
                        entries,
                        notice: self.vault.load_notice.clone(),
                        never: self.vault.never.clone(),
                    }));
                }
            }
            FromUi::VaultReveal { id } => {
                // Decrypt here, but only send it to the UI after Windows has verified the user.
                let Ok((username, password)) = self.vault.get_credentials(&id) else {
                    return;
                };
                let hwnd = engine.main_window_handle();
                std::thread::spawn(move || {
                    let result = auth::verify(hwnd, "Show a saved password");
                    CefEngine.post_ui(move || {
                        let msg = match result {
                            auth::Auth::Verified => ToUi::VaultCredentials {
                                id,
                                username,
                                password,
                            },
                            auth::Auth::Cancelled => ToUi::VaultRevealDenied {
                                id,
                                message: "Not verified".to_string(),
                            },
                            auth::Auth::Unavailable(m) => {
                                ToUi::VaultRevealDenied { id, message: m }
                            }
                        };
                        CefEngine.send_to_ui(&bridge::to_json(&msg));
                    });
                });
            }
            FromUi::SelftestResult { ok, log } => selftest::ipc_result(ok, &log),
            other => eprintln!("bridge: ignored {other:?} (unknown view)"),
        }
    }
}

fn connect(engine: CefEngine, is_private: bool, any_selftest: bool, urls_file: Option<PathBuf>) {
    let history_store = if is_private {
        None
    } else {
        HistoryStore::open(&profile_dir().join("history.db")).ok()
    };
    let saved_session = if is_private || any_selftest || urls_file.is_some() {
        SessionData::default()
    } else {
        load_session_or_default(&profile_dir())
    };
    let shell = Rc::new(RefCell::new(Shell {
        is_private,
        history: history_store,
        rail_width: saved_session.rail_width,
        active_space: saved_session.active_space,
        extensions: extensions::ExtensionsStore::load_or_default(&profile_dir()),
        ..Shell::default()
    }));

    shell.borrow_mut().restore_closed(&saved_session.closed);

    if let Some(path) = urls_file {
        if let Ok(content) = std::fs::read_to_string(path) {
            let mut s = shell.borrow_mut();
            for u in content.lines().map(str::trim).filter(|l| !l.is_empty()) {
                let view = CefEngine.create_view(u);
                s.tabs.push(Tab::new(view, None));
                s.on_url_changed(view, u);
            }
            if let Some(first) = s.tabs.first().map(|t| t.view) {
                s.show(&mut CefEngine, first);
            }
        }
    } else if !saved_session.tabs.is_empty() {
        let mut s = shell.borrow_mut();
        let mut created_views = Vec::new();
        let active_idx = saved_session.active_tab_index.unwrap_or(0);
        for (i, tab) in saved_session.tabs.iter().enumerate() {
            // Only the active tab loads now; the others load when first shown (EDGE_CASES §6).
            let web = tab.url.starts_with("http://") || tab.url.starts_with("https://");
            let lazy = web && i != active_idx;
            let view = CefEngine.create_view(if lazy || !web { "" } else { &tab.url });
            created_views.push(view);
            let mut t = Tab::with_details(view, None, tab.space, tab.pinned);
            t.restore_pending = lazy;
            s.tabs.push(t);
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

    let pending_events: Rc<RefCell<VecDeque<engine_api::Event>>> =
        Rc::new(RefCell::new(VecDeque::new()));
    let pending_events_sink = pending_events.clone();
    let events_shell = shell.clone();
    fn dispatch_engine_event(s: &mut Shell, event: &engine_api::Event) {
        match event {
            engine_api::Event::Closed { view } => {
                s.close_tab(*view);
                if s.shown.is_none()
                    && let Some(last) = s.tabs.last().map(|t| t.view)
                {
                    s.show(&mut CefEngine, last);
                    CefEngine.send_to_ui(&bridge::to_json(&ToUi::ViewShown { view: last }));
                }
                let infos = s.closed_tab_infos();
                CefEngine.send_to_ui(&bridge::to_json(&ToUi::ClosedTabsUpdated { tabs: infos }));
            }
            engine_api::Event::UrlChanged { view, url } => {
                s.on_url_changed(*view, url);
            }
            engine_api::Event::TitleChanged { view, title } => {
                s.on_title_changed(*view, title);
            }
            engine_api::Event::Download(info) => {
                if !info.path.is_empty() {
                    s.download_paths.insert(info.id, info.path.clone());
                }
                if info.complete || info.cancelled || info.interrupted {
                    s.active_downloads.remove(&info.id);
                } else {
                    s.active_downloads.insert(info.id, info.view);
                }
            }
            engine_api::Event::LoginSubmitted {
                view,
                username,
                password,
            } => s.on_login_submitted(*view, username, password),
            engine_api::Event::LoginFieldClicked { view } => s.on_fill_requested(*view),
            engine_api::Event::FormInputChanged { view, has_input } => {
                if let Some(t) = s.tabs.iter_mut().find(|t| t.view == *view) {
                    t.has_form_input = *has_input;
                }
            }
            engine_api::Event::AudioChanged { view, playing } => {
                if let Some(t) = s.tabs.iter_mut().find(|t| t.view == *view) {
                    t.has_audio = *playing;
                }
            }
            engine_api::Event::ShortcutNewTab => {
                s.handle(
                    &mut CefEngine,
                    FromUi::Open {
                        url: "".into(),
                        opener: None,
                    },
                );
            }
            engine_api::Event::ShortcutCloseTab { view } => {
                s.handle(&mut CefEngine, FromUi::Close { view: Some(*view) });
            }
            engine_api::Event::ShortcutReload { view } => {
                CefEngine.reload(*view);
            }
            engine_api::Event::ShortcutBack { view } => {
                CefEngine.go_back(*view);
            }
            engine_api::Event::ShortcutForward { view } => {
                CefEngine.go_forward(*view);
            }
            engine_api::Event::ShortcutNextTab => {
                s.handle(&mut CefEngine, FromUi::NextTab);
            }
            engine_api::Event::ShortcutPrevTab => {
                s.handle(&mut CefEngine, FromUi::PrevTab);
            }
            engine_api::Event::ShortcutNewPrivateWindow => open_private_window(),
            engine_api::Event::ShortcutQuit => CefEngine.quit(),
            engine_api::Event::ShortcutFocusAddress => {
                CefEngine.focus_ui();
                CefEngine.send_to_ui(&bridge::to_json(&ToUi::HelmFocus));
            }
            engine_api::Event::ShortcutOpenHistory => {
                s.handle(
                    &mut CefEngine,
                    FromUi::Open {
                        url: "odyssey://history".into(),
                        opener: None,
                    },
                );
            }
            engine_api::Event::ShortcutOpenSettings => {
                s.handle(
                    &mut CefEngine,
                    FromUi::Open {
                        url: "odyssey://settings".into(),
                        opener: None,
                    },
                );
            }
            engine_api::Event::ShortcutFindInPage { view } => {
                CefEngine.send_to_ui(&bridge::to_json(&ToUi::FindInPagePrompt { view: *view }));
            }
            engine_api::Event::ShortcutDevTools { view } => {
                CefEngine.open_devtools(*view);
            }
            engine_api::Event::ShortcutRelightClosedTab => {
                s.handle(&mut CefEngine, FromUi::RelightClosedTab { id: None });
            }
            engine_api::Event::ShortcutPrint { view } => {
                CefEngine.print(*view);
            }
            engine_api::Event::ShortcutZoomIn { view } => {
                let f = s.zoom_step(*view, 1);
                CefEngine.set_zoom(*view, f);
            }
            engine_api::Event::ShortcutZoomOut { view } => {
                let f = s.zoom_step(*view, -1);
                CefEngine.set_zoom(*view, f);
            }
            engine_api::Event::ShortcutZoomReset { view } => {
                let f = s.zoom_step(*view, 0);
                CefEngine.set_zoom(*view, f);
            }
            engine_api::Event::NewWindowRequested {
                opener,
                url,
                kind,
                user_gesture,
            } => {
                // Pop-ups nobody asked for are blocked (EDGE_CASES §2).
                if !*user_gesture || url.is_empty() {
                    return;
                }
                let background = *kind == engine_api::NewWindowKind::BackgroundTab;
                s.open_tab(&mut CefEngine, url, Some(*opener), background);
            }
            _ => {}
        }
    }

    let is_shell_event = |event: &engine_api::Event| {
        matches!(
            event,
            engine_api::Event::Closed { .. }
                | engine_api::Event::UrlChanged { .. }
                | engine_api::Event::TitleChanged { .. }
                | engine_api::Event::AudioChanged { .. }
                | engine_api::Event::Download(_)
                | engine_api::Event::FormInputChanged { .. }
                | engine_api::Event::LoginSubmitted { .. }
                | engine_api::Event::LoginFieldClicked { .. }
                | engine_api::Event::ShortcutNewTab
                | engine_api::Event::ShortcutNewPrivateWindow
                | engine_api::Event::ShortcutQuit
                | engine_api::Event::ShortcutCloseTab { .. }
                | engine_api::Event::ShortcutReload { .. }
                | engine_api::Event::ShortcutBack { .. }
                | engine_api::Event::ShortcutForward { .. }
                | engine_api::Event::ShortcutNextTab
                | engine_api::Event::ShortcutPrevTab
                | engine_api::Event::ShortcutFocusAddress
                | engine_api::Event::ShortcutOpenHistory
                | engine_api::Event::ShortcutOpenSettings
                | engine_api::Event::ShortcutFindInPage { .. }
                | engine_api::Event::ShortcutDevTools { .. }
                | engine_api::Event::ShortcutRelightClosedTab
                | engine_api::Event::ShortcutPrint { .. }
                | engine_api::Event::ShortcutZoomIn { .. }
                | engine_api::Event::ShortcutZoomOut { .. }
                | engine_api::Event::ShortcutZoomReset { .. }
                | engine_api::Event::NewWindowRequested { .. }
        )
    };

    engine.set_event_sink(move |event| {
        let sleeping_noise = match &event {
            engine_api::Event::UrlChanged { view, .. }
            | engine_api::Event::TitleChanged { view, .. } => events_shell
                .try_borrow()
                .map(|s| s.is_sleeping(*view))
                .unwrap_or(false),
            _ => false,
        };
        if sleeping_noise {
            return;
        }
        if is_shell_event(&event) {
            if let Ok(mut s) = events_shell.try_borrow_mut() {
                dispatch_engine_event(&mut s, &event);
            } else {
                pending_events_sink.borrow_mut().push_back(event.clone());
            }
        }
        // `on_url_changed` already tells the UI about a new URL; don't send it twice.
        if matches!(event, engine_api::Event::UrlChanged { .. }) {
            return;
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
            let next_event = drain_pending.borrow_mut().pop_front();
            match next_event {
                Some(ev) => {
                    if let Ok(mut s) = ui_shell.try_borrow_mut() {
                        dispatch_engine_event(&mut s, &ev);
                    }
                }
                None => break,
            }
        }
    });

    fn schedule_sleep_timer(shell: Rc<RefCell<Shell>>) {
        let timer_shell = shell.clone();
        CefEngine.after(30_000, move || {
            if let Ok(mut s) = timer_shell.try_borrow_mut()
                && let Some(secs) = sleep_timeout_secs(s.is_private)
            {
                s.check_tab_sleeping(&mut CefEngine, secs);
            }
            schedule_sleep_timer(timer_shell);
        });
    }
    fn schedule_flush_timer(shell: Rc<RefCell<Shell>>) {
        let timer_shell = shell.clone();
        CefEngine.after(1_000, move || {
            if let Ok(s) = timer_shell.try_borrow() {
                s.flush_session();
            }
            schedule_flush_timer(timer_shell);
        });
    }
    schedule_flush_timer(shell.clone());
    schedule_sleep_timer(shell);
}

fn main() {
    let engine_selftest = std::env::args().any(|a| a == "--engine-selftest");
    let ipc_selftest = std::env::args().any(|a| a == "--ipc-selftest");
    let is_private =
        std::env::args().any(|a| a == "--private" || a == "-private" || a == "--incognito");
    let any_selftest = engine_selftest || ipc_selftest;
    let profile_arg =
        std::env::args().find_map(|a| a.strip_prefix("--profile-dir=").map(PathBuf::from));
    let urls_file_arg =
        std::env::args().find_map(|a| a.strip_prefix("--urls-file=").map(PathBuf::from));
    if let Some(p) = &profile_arg {
        let _ = PROFILE_OVERRIDE.set(p.clone());
    }
    let start_profile = profile_arg.clone();
    let result = engine_cef::run(
        move || engine_cef::StartOptions {
            // Self-tests use a throwaway profile so they never touch real data.
            // Private windows use an in-memory profile: no cookies, cache or storage on disk (P3-02).
            profile_dir: if let Some(p) = start_profile {
                p
            } else if is_private {
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
                "?private=1".into()
            } else {
                String::new()
            },
            extension_paths: if is_private || any_selftest {
                Vec::new()
            } else {
                extensions::ExtensionsStore::load_or_default(&profile_dir()).enabled_paths()
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
                connect(engine, is_private, any_selftest, urls_file_arg);
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
        "connections": 8,
        "offerSavePasswords": true,
        "autofillPasswords": true
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
    /// Tabs closed in the last 24 hours, so Ctrl+Shift+T still works after a restart.
    #[serde(default)]
    pub closed: Vec<SavedClosed>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SavedClosed {
    pub url: String,
    pub title: String,
    pub host: String,
    pub closed_at: i64,
}

fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

impl Default for SessionData {
    fn default() -> Self {
        Self {
            tabs: Vec::new(),
            active_tab_index: None,
            rail_width: RAIL_WIDTH,
            active_space: 0,
            closed: Vec::new(),
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

/// Idle time before background tabs sleep, from the `sleepMinutes` setting (default 10;
/// 0 means never).
fn sleep_timeout_secs(is_private: bool) -> Option<i64> {
    let settings = if is_private {
        default_settings()
    } else {
        load_settings_or_default(&profile_dir())
    };
    let minutes = settings
        .get("sleepMinutes")
        .and_then(|v| v.as_u64())
        .unwrap_or(10);
    (minutes > 0).then_some(minutes as i64 * 60)
}

/// Tab title for an `odyssey://<name>` page.
fn internal_page_title(name: &str) -> String {
    let name = name.trim_matches('/');
    let mut chars = name.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => "New tab".to_string(),
    }
}

/// A private window is its own process with an in-memory profile, so nothing it does can reach
/// the normal profile on disk.
fn open_private_window() {
    match std::env::current_exe() {
        Ok(exe) => {
            if let Err(e) = std::process::Command::new(exe).arg("--private").spawn() {
                eprintln!("Couldn't open a private window: {e}");
            }
        }
        Err(e) => eprintln!("Couldn't find our own program: {e}"),
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

/// File types that run code when opened. These are never opened from the Downloads list
/// (EDGE_CASES §4 external programs): the user can use Show in folder instead.
pub fn is_executable_name(path: &Path) -> bool {
    const RISKY: &[&str] = &[
        "exe", "com", "bat", "cmd", "msi", "scr", "pif", "lnk", "vbs", "vbe", "js", "jse", "wsf",
        "wsh", "ps1", "psm1", "reg", "hta", "cpl", "jar", "sh", "desktop", "appimage", "run",
        "dll", "msc", "gadget",
    ];
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| RISKY.contains(&e.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

/// The program and argument used to open a file. The path is passed as one argument and never
/// goes through a shell, so `&`, `^` or `%` in a file name can't start anything else.
pub fn open_command(path: &Path) -> (&'static str, std::ffi::OsString) {
    if cfg!(windows) {
        ("explorer.exe", path.as_os_str().to_owned())
    } else {
        ("xdg-open", path.as_os_str().to_owned())
    }
}

/// Open a downloaded file with the default associated application (EDGE_CASES §5).
pub fn open_file(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Err("File does not exist".into());
    }
    if is_executable_name(path) {
        return Err("Programs aren't opened from Downloads. Use Show in folder instead.".into());
    }
    let (program, arg) = open_command(path);
    std::process::Command::new(program)
        .arg(arg)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tabs_with_downloads_do_not_sleep() {
        let mut shell = Shell::default();
        let mut shown = Tab::new(1, None);
        shown.last_active = 0;
        let mut dl = Tab::new(2, None);
        dl.last_active = 0;
        shell.tabs.push(shown);
        shell.tabs.push(dl);
        shell.shown = Some(1);
        shell.active_downloads.insert(7, Some(2));
        shell.check_tab_sleeping(&mut CefEngine, 600);
        assert!(
            !shell.tabs[1].sleeping,
            "tab with a download must stay awake"
        );
        shell.active_downloads.clear();
        shell.check_tab_sleeping(&mut CefEngine, 600);
        assert!(shell.tabs[1].sleeping);
    }

    #[test]
    fn internal_pages_get_titles() {
        assert_eq!(internal_page_title("settings"), "Settings");
        assert_eq!(internal_page_title("history/"), "History");
    }

    #[test]
    fn relight_last_closed_reopens_the_most_recent() {
        let mut shell = Shell::default();
        for (v, url) in [(1, "https://one.example"), (2, "https://two.example")] {
            shell.tabs.push(Tab::new(v, None));
            shell.tab_urls.insert(v, url.into());
            shell.close_tab(v);
        }
        let (url, _) = shell.relight_last_closed().unwrap();
        assert_eq!(url, "https://two.example");
    }

    #[test]
    fn zoom_steps_through_chrome_levels() {
        let mut shell = Shell::default();
        assert_eq!(shell.zoom_step(1, 1), 1.1);
        assert_eq!(shell.zoom_step(1, 1), 1.25);
        assert_eq!(shell.zoom_step(1, -1), 1.1);
        assert_eq!(shell.zoom_step(1, 0), 1.0);
        assert_eq!(shell.zoom_step(2, -1), 0.9);
    }

    #[test]
    fn filename_with_ampersand_is_one_argument_and_programs_are_refused() {
        let p = Path::new("C:\\Downloads\\a&calc.exe&.txt");
        let (program, arg) = open_command(p);
        assert!(program == "explorer.exe" || program == "xdg-open");
        assert_eq!(arg, p.as_os_str());
        assert!(!is_executable_name(p));
        assert!(is_executable_name(Path::new("setup.EXE")));
        assert!(is_executable_name(Path::new("run.bat")));
    }

    #[test]
    fn sleeping_tab_keeps_its_url() {
        let mut shell = Shell::default();
        let mut t = Tab::new(1, None);
        t.sleeping = true;
        shell.tabs.push(t);
        shell.tab_urls.insert(1, "https://real.example/page".into());
        shell.on_url_changed(1, "about:blank");
        shell.on_title_changed(1, "");
        assert_eq!(shell.tab_urls[&1], "https://real.example/page");
    }

    #[test]
    fn next_tab_stays_in_the_active_space() {
        let mut shell = Shell::default();
        shell.tabs.push(Tab::with_details(1, None, 0, false));
        shell.tabs.push(Tab::with_details(2, None, 1, false));
        shell.tabs.push(Tab::with_details(3, None, 0, false));
        shell.active_space = 0;
        assert_eq!(shell.space_tabs(), vec![1, 3]);
    }

    #[test]
    fn ai_save_without_key_keeps_the_existing_key() {
        let current = ai::AiConfig {
            api_key: Some("sk-existing".into()),
            ..ai::AiConfig::default()
        };
        let incoming = ai::AiConfig {
            model: "m".into(),
            api_key: None,
            ..ai::AiConfig::default()
        };
        let (merged, err) = apply_ai_config(&current, incoming);
        assert!(err.is_none());
        assert_eq!(merged.api_key.as_deref(), Some("sk-existing"));
        assert_eq!(merged.model, "m");
    }

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
            closed: vec![SavedClosed {
                url: "https://closed.example".into(),
                title: "Closed".into(),
                host: "closed.example".into(),
                closed_at: 1,
            }],
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
        shell.tabs.push(Tab::with_details(1, None, 0, false));
        // Space 1: Tab 2 (pinned), Tab 3
        shell.tabs.push(Tab::with_details(2, None, 1, true));
        shell.tabs.push(Tab::with_details(3, None, 1, false));
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

    #[test]
    fn tab_sleeping_idle_and_exclusions() {
        let mut shell = Shell::default();
        // Tab 1: active shown tab
        let mut t1 = Tab::with_details(1, None, 0, false);
        t1.last_active = 0; // old
        shell.tabs.push(t1);
        shell.shown = Some(1);

        // Tab 2: background idle tab (should sleep)
        let mut t2 = Tab::with_details(2, None, 0, false);
        t2.last_active = 0; // old
        shell.tabs.push(t2);

        // Tab 3: pinned tab (never sleep)
        let mut t3 = Tab::with_details(3, None, 0, true);
        t3.last_active = 0;
        shell.tabs.push(t3);

        // Tab 4: audio playing tab (never sleep)
        let mut t4 = Tab::with_details(4, None, 0, false);
        t4.has_audio = true;
        t4.last_active = 0;
        shell.tabs.push(t4);

        // Tab 5: form input tab (never sleep)
        let mut t5 = Tab::with_details(5, None, 0, false);
        t5.has_form_input = true;
        t5.last_active = 0;
        shell.tabs.push(t5);

        // Tab 6: recent tab (not old enough to sleep)
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let mut t6 = Tab::with_details(6, None, 0, false);
        t6.last_active = now - 60; // only 1 min old
        shell.tabs.push(t6);

        // Run check with timeout of 600s
        shell.check_tab_sleeping(&mut CefEngine, 600);

        assert!(!shell.tabs[0].sleeping, "active tab should never sleep");
        assert!(shell.tabs[1].sleeping, "background idle tab should sleep");
        assert!(!shell.tabs[2].sleeping, "pinned tab should never sleep");
        assert!(!shell.tabs[3].sleeping, "audio tab should never sleep");
        assert!(!shell.tabs[4].sleeping, "form input tab should never sleep");
        assert!(!shell.tabs[5].sleeping, "recent tab should not sleep");

        // Now wake tab 2
        shell.show(&mut CefEngine, 2);
        assert!(!shell.tabs[1].sleeping, "showing sleeping tab wakes it");
        assert_eq!(shell.shown, Some(2));
    }
}
