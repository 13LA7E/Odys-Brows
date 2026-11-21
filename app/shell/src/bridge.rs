//! The message bridge between our UI and Rust (P1-03). JSON objects with a `type` field.
//! Only our UI can send these (see `engine-cef`: the bridge exists only in our UI's frame, and
//! only our UI's browser listens). Anything malformed is logged and ignored, never a crash.

use engine_api::{Event, NewWindowKind, ViewId};
use serde::{Deserialize, Serialize};

/// Largest message we accept from the UI (EDGE_CASES §1: huge input).
pub const MAX_MESSAGE_BYTES: usize = 256 * 1024;
/// The one message that carries a file: the new-tab wallpaper (4 MB image as text). wallpaper.rs checks it.
pub const MAX_WALLPAPER_MESSAGE_BYTES: usize = 6 * 1024 * 1024;

fn deserialize_url<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let val = serde_json::Value::deserialize(deserializer)?;
    match val {
        serde_json::Value::String(s) => Ok(s),
        serde_json::Value::Object(map) => {
            if let Some(serde_json::Value::String(u)) = map.get("url") {
                Ok(u.clone())
            } else {
                Ok(String::new())
            }
        }
        _ => Ok(String::new()),
    }
}

/// One saved login offered for autofill (no password).
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct FillChoice {
    pub id: String,
    pub username: String,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct OverlayRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    #[serde(default)]
    pub radius: f64,
}

/// Messages our UI sends to Rust.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "kebab-case", deny_unknown_fields)]
pub enum FromUi {
    /// Open a new page view with this URL and show it.
    Open {
        #[serde(deserialize_with = "deserialize_url", alias = "target")]
        url: String,
        #[serde(default)]
        opener: Option<ViewId>,
    },
    Navigate {
        #[serde(default)]
        view: Option<ViewId>,
        #[serde(deserialize_with = "deserialize_url", alias = "target")]
        url: String,
    },
    Back {
        #[serde(default)]
        view: Option<ViewId>,
    },
    Forward {
        #[serde(default)]
        view: Option<ViewId>,
    },
    Reload {
        #[serde(default)]
        view: Option<ViewId>,
    },
    Stop {
        #[serde(default)]
        view: Option<ViewId>,
    },
    /// Show this view and hide the others (a tab switch).
    Show {
        #[serde(default)]
        view: Option<ViewId>,
    },
    Close {
        #[serde(default)]
        view: Option<ViewId>,
    },
    /// Window controls from the notch (P2-02).
    MinimizeWindow,
    MaximizeWindow,
    /// Open a private window (a second process with `--private`, AUDIT-26).
    OpenPrivateWindow,
    /// Export every saved login to a CSV file in the Downloads folder.
    VaultExport,
    /// Import logins from the text of a CSV file (Chrome, Brave, Edge, Firefox layouts).
    VaultImport {
        csv: String,
    },
    /// Offer to save passwords for this site again.
    VaultRemoveNever {
        host: String,
    },
    /// The user's answer to "Save password?": `save`, `never` or `dismiss`.
    AnswerSaveLogin {
        request: u64,
        action: String,
    },
    /// The user's choice among several saved logins to fill (`None` = none of them).
    AnswerFill {
        request: u64,
        #[serde(default)]
        id: Option<String>,
    },
    /// The picked new-tab wallpaper as a `data:` URL (checked in Rust, saved in the profile).
    SetWallpaper {
        data: String,
    },
    /// Remove the wallpaper.
    ClearWallpaper,
    /// Bind a shortcut: `combo` like `Ctrl+Shift+K` to command `id` (checked in Rust).
    SetShortcut {
        id: String,
        combo: String,
    },
    /// Put one command, or every command (no id), back to its default keys.
    ResetShortcuts {
        #[serde(default)]
        id: Option<String>,
    },
    /// A keyboard shortcut was pressed in our own window: run this command (see `shortcuts.rs`).
    RunCommand {
        id: String,
    },
    /// Bring our window forward and give it keyboard focus (a click while another app is on top).
    FocusWindow,
    /// How much of the notch is showing, in DIP from the top (it slides away over web pages).
    SetNotchHeight {
        height: f64,
        /// How wide the notch is (it grows with pinned extensions); older UIs leave it out.
        #[serde(default)]
        width: Option<f64>,
    },
    /// Where the UI draws over the page area, so the page windows leave holes there (AUDIT-06).
    SetOverlayRects {
        rects: Vec<OverlayRect>,
    },
    /// The user's answer to a permission prompt (AUDIT-21).
    AnswerPermission {
        request: u64,
        allow: bool,
    },
    /// The user's answer to a certificate error (AUDIT-21).
    AnswerCertificate {
        request: u64,
        proceed: bool,
    },
    CloseWindow,
    /// Rail width change from collapse/expand (P2-05).
    SetRailWidth {
        width: i32,
    },
    /// Save settings (P2-07).
    SaveSettings {
        settings: serde_json::Value,
    },
    /// Load settings (P2-07).
    LoadSettings,
    /// Pause, resume or cancel a download (P4-01).
    DownloadControl {
        id: String,
        action: String,
    },
    /// Open a completed download with the default app (P4-01).
    OpenDownload {
        path: String,
    },
    /// Show a downloaded file in the system file manager (P4-01).
    ShowInFolder {
        path: String,
    },
    /// Query history day groups (P4-02).
    QueryHistory,
    /// Search history (P4-02).
    SearchHistory {
        query: String,
    },
    /// Forget a specific visit (P4-02).
    ForgetVisit {
        id: i64,
    },
    /// Clear a day of history (P4-02).
    ClearHistoryDay {
        day_start_secs: i64,
    },
    /// Clear all history (P4-02).
    ClearAllHistory,
    /// Relight a closed tab (P4-04, None relights most recent).
    RelightClosedTab {
        #[serde(default)]
        id: Option<u64>,
    },
    /// Switch to next tab.
    NextTab,
    /// Switch to prev tab.
    PrevTab,
    /// Forget a closed tab from the closed earlier list (P4-04).
    ForgetClosedTab {
        id: u64,
    },
    /// Set page zoom factor (P4-06).
    SetZoom {
        view: ViewId,
        factor: f64,
    },
    /// Find text in page (P4-06).
    FindInPage {
        view: ViewId,
        text: String,
        forward: bool,
        match_case: bool,
        next: bool,
    },
    /// Stop finding in page (P4-06).
    StopFinding {
        view: ViewId,
        clear_selection: bool,
    },
    /// Print page (P4-06).
    PrintPage {
        view: ViewId,
    },
    /// Open devtools (P4-06).
    OpenDevTools {
        view: ViewId,
    },
    /// Switch active space (P4-07).
    SwitchSpace {
        space: usize,
    },
    /// Open or focus a pinned site (P4-07).
    OpenPinnedSite {
        name: String,
    },
    /// Toggle ad/tracker blocking exception for host (P5-01).
    ToggleShieldsException {
        host: String,
    },
    /// Request current shields status for a view or active tab (P5-01).
    GetShieldsStatus {
        #[serde(default)]
        view: Option<ViewId>,
    },
    /// Explicitly sleep or wake a tab (P5-02).
    SetTabSleeping {
        view: ViewId,
        sleeping: bool,
    },
    /// Request installed extensions list (P6-01).
    GetExtensions,
    /// Toggle an extension on/off (P6-01).
    ToggleExtension {
        id: String,
        enabled: bool,
    },
    /// Remove an extension (P6-01).
    RemoveExtension {
        id: String,
    },
    /// Load an unpacked extension folder (P6-01).
    LoadUnpackedExtension {
        path: String,
    },
    /// Ask a question in Helm ask mode (P7-01, P7-03).
    AiAsk {
        prompt: String,
        #[serde(default)]
        context: Option<String>,
        /// The conversation this belongs to (none = start a new one).
        #[serde(default)]
        chat: Option<String>,
    },
    /// The list of saved assistant conversations.
    ChatList,
    /// Open one saved conversation.
    ChatOpen {
        id: String,
    },
    /// Delete one saved conversation.
    ChatDelete {
        id: String,
    },
    /// Delete every saved conversation.
    ChatClearAll,
    /// Cancel active AI request (P7-01, P7-03).
    AiCancel,
    /// Test an AI provider connection from Settings (P7-01, P7-02).
    AiTestConnection {
        config: ai::AiConfig,
    },
    /// Save AI config from Settings (P7-02).
    AiSaveConfig {
        config: ai::AiConfig,
    },
    /// Fetch current AI configuration (P7-02).
    AiGetConfig,
    /// List saved logins (P8-02).
    VaultList,
    /// List saved logins for origin (P8-02).
    VaultListForOrigin {
        origin: String,
    },
    /// Add a new login (P8-02).
    VaultAdd {
        origin: String,
        username: String,
        password: String,
    },
    /// Update a login (P8-02).
    VaultUpdate {
        id: String,
        #[serde(default)]
        origin: Option<String>,
        #[serde(default)]
        username: Option<String>,
        #[serde(default)]
        password: Option<String>,
    },
    /// Delete a login (P8-02).
    VaultDelete {
        id: String,
    },
    /// Reveal login password (P8-02).
    VaultReveal {
        id: String,
    },
    /// Only used by `--ipc-selftest`.
    SelftestResult {
        ok: bool,
        log: Vec<String>,
    },
}

/// Messages Rust sends to our UI.
#[derive(Debug, Serialize, PartialEq)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ToUi {
    ViewOpened {
        view: ViewId,
        /// Which space (Home, Work, Personal) the tab lives in.
        space: usize,
        #[serde(skip_serializing_if = "Option::is_none")]
        opener: Option<ViewId>,
        #[serde(skip_serializing_if = "Option::is_none")]
        url: Option<String>,
    },
    ViewShown {
        view: ViewId,
    },
    HelmFocus,
    /// Switch the Helm between the address bar and asking the assistant (Ctrl+J).
    ToggleAsk,
    FindInPagePrompt {
        view: ViewId,
    },
    Url {
        view: ViewId,
        url: String,
    },
    Title {
        view: ViewId,
        title: String,
    },
    Loading {
        view: ViewId,
        loading: bool,
        can_go_back: bool,
        can_go_forward: bool,
    },
    LoadFailed {
        view: ViewId,
        url: String,
        code: i32,
        text: String,
    },
    PermissionRequest {
        view: ViewId,
        request: u64,
        origin: String,
        kind: String,
    },
    CertificateError {
        view: ViewId,
        request: u64,
        url: String,
        error: String,
    },
    NewWindow {
        opener: ViewId,
        url: String,
        background: bool,
        user_gesture: bool,
    },
    Closed {
        view: ViewId,
    },
    SettingsLoaded {
        settings: serde_json::Value,
    },
    DownloadProgress {
        id: u32,
        url: String,
        file_name: String,
        received: i64,
        #[serde(skip_serializing_if = "Option::is_none")]
        total: Option<i64>,
        speed: i64,
        complete: bool,
        cancelled: bool,
        path: String,
    },
    HistoryLoaded {
        groups: Vec<history::DayGroup>,
    },
    ClosedTabsUpdated {
        tabs: Vec<ClosedTabInfo>,
    },
    SpaceChanged {
        space: usize,
    },
    /// Shields status update (P5-01).
    ShieldsStatus {
        view: ViewId,
        count: u64,
        enabled: bool,
    },
    /// Tab sleeping state update (P5-02).
    TabSleeping {
        view: ViewId,
        sleeping: bool,
    },
    /// Extensions list update (P6-01, P6-03).
    ExtensionsLoaded {
        extensions: Vec<crate::extensions::ExtensionInfo>,
    },
    /// Streamed AI event or answer delta (P7-01, P7-03).
    AiEvent {
        event: ai::AiEvent,
    },
    /// Loaded AI settings (keys stripped, P7-02).
    AiConfigLoaded {
        config: ai::AiSettingsUi,
    },
    /// Test connection response (P7-01, P7-02).
    AiTestResult {
        ok: bool,
        message: String,
    },
    /// Vault entries loaded (P8-02).
    /// The commands with their current keys (after the user's changes).
    Shortcuts {
        commands: serde_json::Value,
    },
    /// The new-tab wallpaper (a `data:` URL, or none) and, if a change was refused, why.
    Wallpaper {
        data: Option<String>,
        error: Option<String>,
    },
    /// Saved conversations, newest first.
    ChatList {
        chats: Vec<crate::chats::Summary>,
    },
    /// One saved conversation (none when it does not exist).
    ChatData {
        chat: Option<crate::chats::Chat>,
    },
    /// A question started (or continued) this conversation.
    ChatStarted {
        id: String,
    },
    /// The assistant's reply was saved.
    ChatSaved {
        id: String,
    },
    /// The answer to `set-shortcut` / `reset-shortcuts`; `shortcuts` is what is now saved.
    ShortcutResult {
        ok: bool,
        message: String,
        shortcuts: serde_json::Value,
    },
    SaveLoginPrompt {
        view: ViewId,
        request: u64,
        host: String,
        username: String,
        update: bool,
    },
    FillChoices {
        view: ViewId,
        request: u64,
        host: String,
        logins: Vec<FillChoice>,
    },
    /// Windows verification failed or was cancelled, so the password was not sent.
    VaultRevealDenied {
        id: String,
        message: String,
    },
    VaultExported {
        ok: bool,
        message: String,
        path: String,
    },
    VaultImported {
        added: usize,
        skipped: usize,
        error: Option<String>,
    },
    VaultLoaded {
        entries: Vec<vault::VaultEntrySummary>,
        /// Sites the user chose "Never" for.
        never: Vec<String>,
        /// Set when the saved-logins file on disk was damaged (it was kept aside).
        #[serde(skip_serializing_if = "Option::is_none")]
        notice: Option<String>,
    },
    /// Revealed login credentials (P8-02).
    VaultCredentials {
        id: String,
        username: String,
        password: String,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClosedTabInfo {
    pub id: u64,
    pub title: String,
    pub url: String,
    pub host: String,
    pub closed_ago: String,
}

/// Parse one message from the UI. `Err` carries a short reason for the log.
pub fn parse(json: &str) -> Result<FromUi, String> {
    let limit = if json.starts_with(r#"{"type":"set-wallpaper""#) {
        MAX_WALLPAPER_MESSAGE_BYTES
    } else {
        MAX_MESSAGE_BYTES
    };
    if json.len() > limit {
        return Err(format!("message too large ({} bytes)", json.len()));
    }
    serde_json::from_str(json).map_err(|e| e.to_string())
}

/// What the UI needs to hear about an engine event (`None`: nothing).
pub fn event_to_ui(e: &Event) -> Option<ToUi> {
    Some(match e.clone() {
        Event::UrlChanged { view, url } => ToUi::Url { view, url },
        Event::TitleChanged { view, title } => ToUi::Title { view, title },
        Event::LoadingChanged {
            view,
            loading,
            can_go_back,
            can_go_forward,
        } => ToUi::Loading {
            view,
            loading,
            can_go_back,
            can_go_forward,
        },
        Event::LoadFailed {
            view,
            url,
            code,
            text,
        } => ToUi::LoadFailed {
            view,
            url,
            code,
            text,
        },
        Event::NewWindowRequested {
            opener,
            url,
            kind,
            user_gesture,
        } => ToUi::NewWindow {
            opener,
            url,
            background: kind == NewWindowKind::BackgroundTab,
            user_gesture,
        },
        Event::PermissionRequested {
            view,
            request,
            origin,
            kind,
        } => ToUi::PermissionRequest {
            view,
            request,
            origin,
            kind: format!("{kind:?}").to_lowercase(),
        },
        Event::CertificateError {
            view,
            request,
            url,
            error,
        } => ToUi::CertificateError {
            view,
            request,
            url,
            error,
        },
        Event::Closed { view } => ToUi::Closed { view },
        Event::ShieldsCountChanged { view, count } => ToUi::ShieldsStatus {
            view,
            count,
            enabled: true,
        },
        Event::Download(info) => ToUi::DownloadProgress {
            id: info.id,
            url: info.url,
            file_name: info.file_name,
            received: info.received_bytes,
            total: info.total_bytes,
            speed: info.bytes_per_second,
            complete: info.complete,
            cancelled: info.cancelled,
            path: info.path,
        },
        _ => return None,
    })
}

pub fn to_json(m: &ToUi) -> String {
    serde_json::to_string(m).unwrap_or_else(|_| "{\"type\":\"error\"}".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_known_messages() {
        assert_eq!(
            parse(r#"{"type":"open","url":"https://example.com"}"#),
            Ok(FromUi::Open {
                url: "https://example.com".into(),
                opener: None,
            })
        );
        assert_eq!(
            parse(r#"{"type":"open","url":"https://example.com","opener":42}"#),
            Ok(FromUi::Open {
                url: "https://example.com".into(),
                opener: Some(42),
            })
        );
        assert_eq!(
            parse(r#"{"type":"back","view":3}"#),
            Ok(FromUi::Back { view: Some(3) })
        );
        assert_eq!(parse(r#"{"type":"back"}"#), Ok(FromUi::Back { view: None }));
        assert_eq!(
            parse(r#"{"type":"navigate","target":"https://example.com"}"#),
            Ok(FromUi::Navigate {
                view: None,
                url: "https://example.com".into()
            })
        );
        assert_eq!(
            parse(
                r#"{"type":"navigate","target":{"kind":"search","url":"https://duckduckgo.com/?q=test"}}"#
            ),
            Ok(FromUi::Navigate {
                view: None,
                url: "https://duckduckgo.com/?q=test".into()
            })
        );
        assert_eq!(
            parse(r#"{"type":"minimize-window"}"#),
            Ok(FromUi::MinimizeWindow)
        );
        assert_eq!(
            parse(r#"{"type":"maximize-window"}"#),
            Ok(FromUi::MaximizeWindow)
        );
        assert_eq!(parse(r#"{"type":"close-window"}"#), Ok(FromUi::CloseWindow));
        assert_eq!(
            parse(r#"{"type":"set-rail-width","width":264}"#),
            Ok(FromUi::SetRailWidth { width: 264 })
        );
        assert_eq!(
            parse(r#"{"type":"load-settings"}"#),
            Ok(FromUi::LoadSettings)
        );
        assert_eq!(
            parse(r#"{"type":"save-settings","settings":{"accent":"Sea"}}"#),
            Ok(FromUi::SaveSettings {
                settings: serde_json::json!({"accent": "Sea"})
            })
        );
        assert_eq!(
            parse(r#"{"type":"download-control","id":"C:/Downloads/foo.pdf","action":"pause"}"#),
            Ok(FromUi::DownloadControl {
                id: "C:/Downloads/foo.pdf".into(),
                action: "pause".into()
            })
        );
        assert_eq!(
            parse(r#"{"type":"open-download","path":"C:/Downloads/foo.pdf"}"#),
            Ok(FromUi::OpenDownload {
                path: "C:/Downloads/foo.pdf".into()
            })
        );
        assert_eq!(
            parse(r#"{"type":"show-in-folder","path":"C:/Downloads/foo.pdf"}"#),
            Ok(FromUi::ShowInFolder {
                path: "C:/Downloads/foo.pdf".into()
            })
        );
        assert_eq!(
            parse(r#"{"type":"switch-space","space":1}"#),
            Ok(FromUi::SwitchSpace { space: 1 })
        );
        assert_eq!(
            parse(r#"{"type":"open-pinned-site","name":"Mail"}"#),
            Ok(FromUi::OpenPinnedSite {
                name: "Mail".into()
            })
        );
    }

    #[test]
    fn rejects_bad_messages_without_panicking() {
        for bad in [
            "",
            "null",
            "[]",
            "{}",
            r#"{"type":"open"}"#,
            r#"{"type":"nope"}"#,
            r#"{"type":"open","url":"x","extra":1}"#,
            r#"{"type":"back","view":-1}"#,
            "{\"type\":\"open\",\"url\":\"\u{0}\"",
        ] {
            assert!(parse(bad).is_err(), "should reject {bad:?}");
        }
        let huge = format!(
            r#"{{"type":"open","url":"{}"}}"#,
            "a".repeat(MAX_MESSAGE_BYTES)
        );
        assert!(parse(&huge).unwrap_err().contains("too large"));
        // only the wallpaper may be big, and not without limit
        let wp = |n: usize| format!(r#"{{"type":"set-wallpaper","data":"{}"}}"#, "a".repeat(n));
        assert!(parse(&wp(MAX_MESSAGE_BYTES * 4)).is_ok());
        assert!(
            parse(&wp(MAX_WALLPAPER_MESSAGE_BYTES))
                .unwrap_err()
                .contains("too large")
        );
    }

    #[test]
    fn unicode_and_long_titles_survive_round_trip() {
        let title = "Звёзды 🌌 ".repeat(500) + "\u{2028}end";
        let json = to_json(&ToUi::Title {
            view: 1,
            title: title.clone(),
        });
        let back: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(back["title"], title);
        assert_eq!(back["type"], "title");
    }

    #[test]
    fn engine_events_map_to_ui_messages() {
        let e = Event::NewWindowRequested {
            opener: 2,
            url: "https://a".into(),
            kind: NewWindowKind::BackgroundTab,
            user_gesture: true,
        };
        assert_eq!(
            event_to_ui(&e),
            Some(ToUi::NewWindow {
                opener: 2,
                url: "https://a".into(),
                background: true,
                user_gesture: true
            })
        );
        assert_eq!(
            event_to_ui(&Event::AudioChanged {
                view: 1,
                playing: true
            }),
            None
        );
    }

    /// Every message the UI source sends must be accepted by `FromUi`: known type, known field
    /// names. (A dash/underscore mix-up once silently broke find-in-page.)
    #[test]
    fn ui_messages_match_the_bridge_contract() {
        let ui = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../ui");
        let mut checked = 0;
        for entry in std::fs::read_dir(&ui).unwrap().filter_map(Result::ok) {
            let path = entry.path();
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            if path.extension().and_then(|e| e.to_str()) != Some("js")
                || name == "fixtures.js"
                || name == "selftest.js"
            {
                continue;
            }
            let src = std::fs::read_to_string(&path).unwrap();
            for (ty, keys) in sent_messages(&src) {
                checked += 1;
                let bare = format!(r#"{{"type":"{ty}"}}"#);
                if let Err(e) = parse(&bare) {
                    assert!(
                        !e.contains("unknown variant"),
                        "{name}: unknown message type '{ty}'"
                    );
                }
                for key in keys {
                    let msg = format!(r#"{{"type":"{ty}","{key}":null}}"#);
                    if let Err(e) = parse(&msg) {
                        assert!(
                            !e.contains("unknown field"),
                            "{name}: message '{ty}' sends field '{key}' that Rust does not accept ({e})"
                        );
                    }
                }
            }
        }
        assert!(
            checked > 20,
            "expected to find UI messages, found {checked}"
        );
    }

    /// Finds `{ type: 'x', a, b: 1 }` objects passed to `odysseyBridge(JSON.stringify(` or `send(`.
    fn sent_messages(src: &str) -> Vec<(String, Vec<String>)> {
        let bytes = src.as_bytes();
        let mut out = Vec::new();
        let mut from = 0;
        while let Some(rel) = src[from..].find("type: '") {
            let at = from + rel;
            from = at + 7;
            let end_ty = match src[from..].find('\'') {
                Some(e) => from + e,
                None => break,
            };
            let ty = src[from..end_ty].to_string();
            // Walk back to the unmatched `{`.
            let mut depth = 0i32;
            let mut start = None;
            for i in (0..at).rev() {
                match bytes[i] {
                    b'}' => depth += 1,
                    b'{' if depth == 0 => {
                        start = Some(i);
                        break;
                    }
                    b'{' => depth -= 1,
                    _ => {}
                }
            }
            let Some(start) = start else { continue };
            let before = &src[start.saturating_sub(40)..start];
            if !(before.contains("JSON.stringify(") || before.trim_end().ends_with("send(")) {
                continue;
            }
            // Walk forward to the matching `}` collecting top-level keys.
            let mut depth = 0i32;
            let mut keys = Vec::new();
            let mut token = String::new();
            let mut quote: Option<u8> = None;
            let mut expecting_key = true;
            for &b in &bytes[start + 1..] {
                if let Some(q) = quote {
                    if b == q {
                        quote = None;
                        if depth == 0 && expecting_key && q != b'`' {
                            // quoted key such as 'match-case'
                        }
                    } else if depth == 0 && expecting_key {
                        token.push(b as char);
                    }
                    continue;
                }
                match b {
                    b'\'' | b'"' | b'`' => quote = Some(b),
                    b'{' | b'(' | b'[' => depth += 1,
                    b'}' | b')' | b']' => {
                        if depth == 0 {
                            let t = token.trim().to_string();
                            if expecting_key && !t.is_empty() {
                                keys.push(t);
                            }
                            break;
                        }
                        depth -= 1;
                    }
                    b':' if depth == 0 && expecting_key => {
                        keys.push(token.trim().to_string());
                        token.clear();
                        expecting_key = false;
                    }
                    b',' if depth == 0 => {
                        let t = token.trim().to_string();
                        if expecting_key && !t.is_empty() {
                            keys.push(t); // shorthand `{ id }`
                        }
                        token.clear();
                        expecting_key = true;
                    }
                    _ if depth == 0 && expecting_key => token.push(b as char),
                    _ => {}
                }
            }
            keys.retain(|k| k != "type" && !k.is_empty());
            out.push((ty, keys));
        }
        out
    }
}
