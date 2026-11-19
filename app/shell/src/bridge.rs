//! The message bridge between our UI and Rust (P1-03). JSON objects with a `type` field.
//! Only our UI can send these (see `engine-cef`: the bridge exists only in our UI's frame, and
//! only our UI's browser listens). Anything malformed is logged and ignored, never a crash.

use engine_api::{Event, NewWindowKind, ViewId};
use serde::{Deserialize, Serialize};

/// Largest message we accept from the UI (EDGE_CASES §1: huge input).
pub const MAX_MESSAGE_BYTES: usize = 256 * 1024;

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
    /// Relight a closed tab (P4-04).
    RelightClosedTab {
        id: u64,
    },
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
        #[serde(skip_serializing_if = "Option::is_none")]
        opener: Option<ViewId>,
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
    if json.len() > MAX_MESSAGE_BYTES {
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
        Event::Closed { view } => ToUi::Closed { view },
        Event::Download(info) => ToUi::DownloadProgress {
            id: info.id,
            url: info.url,
            file_name: info.file_name,
            received: info.received_bytes,
            total: info.total_bytes,
            speed: info.bytes_per_second,
            complete: info.complete,
            cancelled: info.cancelled,
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
}
