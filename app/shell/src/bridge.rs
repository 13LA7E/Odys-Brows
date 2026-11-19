//! The message bridge between our UI and Rust (P1-03). JSON objects with a `type` field.
//! Only our UI can send these (see `engine-cef`: the bridge exists only in our UI's frame, and
//! only our UI's browser listens). Anything malformed is logged and ignored, never a crash.

use engine_api::{Event, NewWindowKind, ViewId};
use serde::{Deserialize, Serialize};

/// Largest message we accept from the UI (EDGE_CASES §1: huge input).
pub const MAX_MESSAGE_BYTES: usize = 256 * 1024;

/// Messages our UI sends to Rust.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "kebab-case", deny_unknown_fields)]
pub enum FromUi {
    /// Open a new page view with this URL and show it.
    Open {
        url: String,
    },
    Navigate {
        view: ViewId,
        url: String,
    },
    Back {
        view: ViewId,
    },
    Forward {
        view: ViewId,
    },
    Reload {
        view: ViewId,
    },
    Stop {
        view: ViewId,
    },
    /// Show this view and hide the others (a tab switch).
    Show {
        view: ViewId,
    },
    Close {
        view: ViewId,
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
                url: "https://example.com".into()
            })
        );
        assert_eq!(
            parse(r#"{"type":"back","view":3}"#),
            Ok(FromUi::Back { view: 3 })
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
