//! Saved assistant conversations (`chats.json` in the profile folder).
//!
//! Plain JSON, written atomically; a damaged or missing file is an empty history. Limits keep it
//! small: 100 chats, 200 messages each, 20 000 characters per message. Private windows never use
//! this. "Clear history on exit" removes the file (see `privacy.rs`).

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub const MAX_CHATS: usize = 100;
pub const MAX_MESSAGES: usize = 200;
const MAX_TEXT: usize = 20_000;

/// One process-wide lock: the reply is saved from a worker thread while the UI may list or delete.
static LOCK: Mutex<()> = Mutex::new(());

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Source {
    pub title: String,
    pub url: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Message {
    /// "user" or "assistant".
    pub role: String,
    pub text: String,
    /// What the assistant did on the way ("Searching the web for ...").
    #[serde(default)]
    pub steps: Vec<String>,
    #[serde(default)]
    pub sources: Vec<Source>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Chat {
    pub id: String,
    pub title: String,
    /// Seconds since 1970 of the last message.
    pub updated: i64,
    pub messages: Vec<Message>,
}

/// What the list in the UI needs.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Summary {
    pub id: String,
    pub title: String,
    pub updated: i64,
    pub count: usize,
}

#[derive(Default, Serialize, Deserialize)]
struct File {
    #[serde(default)]
    chats: Vec<Chat>,
}

fn path(dir: &Path) -> PathBuf {
    dir.join("chats.json")
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn load(dir: &Path) -> File {
    std::fs::read_to_string(path(dir))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn save(dir: &Path, f: &File) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let tmp = dir.join("chats.json.tmp");
    let text = serde_json::to_string(f).map_err(|e| e.to_string())?;
    std::fs::write(&tmp, text).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path(dir)).map_err(|e| e.to_string())
}

fn clean_text(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
        .take(MAX_TEXT)
        .collect()
}

fn title_of(text: &str) -> String {
    let t: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let t: String = t.chars().take(60).collect();
    if t.is_empty() { "New chat".into() } else { t }
}

fn new_id(f: &File) -> String {
    let base = format!("c{:x}", now());
    let mut id = base.clone();
    let mut n = 1;
    while f.chats.iter().any(|c| c.id == id) {
        id = format!("{base}-{n}");
        n += 1;
    }
    id
}

/// Newest first.
pub fn list(dir: &Path) -> Vec<Summary> {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut v: Vec<Summary> = load(dir)
        .chats
        .into_iter()
        .map(|c| Summary {
            id: c.id,
            title: c.title,
            updated: c.updated,
            count: c.messages.len(),
        })
        .collect();
    v.sort_by_key(|s| std::cmp::Reverse(s.updated));
    v
}

pub fn get(dir: &Path, id: &str) -> Option<Chat> {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    load(dir).chats.into_iter().find(|c| c.id == id)
}

/// Adds a message to chat `id` (a new chat when `id` is `None` or unknown). Returns the chat id.
pub fn append(dir: &Path, id: Option<&str>, msg: Message) -> Result<String, String> {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut f = load(dir);
    let msg = Message {
        role: if msg.role == "assistant" {
            "assistant".into()
        } else {
            "user".into()
        },
        text: clean_text(&msg.text),
        steps: msg
            .steps
            .iter()
            .take(10)
            .map(|s| clean_text(s).chars().take(200).collect())
            .collect(),
        sources: msg
            .sources
            .iter()
            .take(8)
            .map(|s| Source {
                title: clean_text(&s.title).chars().take(150).collect(),
                url: s.url.chars().take(500).collect(),
            })
            .collect(),
    };
    let idx = id.and_then(|id| f.chats.iter().position(|c| c.id == id));
    let idx = match idx {
        Some(i) => i,
        None => {
            let id = new_id(&f);
            f.chats.push(Chat {
                id,
                title: title_of(&msg.text),
                updated: now(),
                messages: Vec::new(),
            });
            f.chats.len() - 1
        }
    };
    let chat = &mut f.chats[idx];
    chat.messages.push(msg);
    if chat.messages.len() > MAX_MESSAGES {
        let extra = chat.messages.len() - MAX_MESSAGES;
        chat.messages.drain(..extra);
    }
    chat.updated = now();
    let chat_id = chat.id.clone();
    if f.chats.len() > MAX_CHATS {
        f.chats.sort_by_key(|c| std::cmp::Reverse(c.updated));
        f.chats.truncate(MAX_CHATS);
    }
    save(dir, &f)?;
    Ok(chat_id)
}

pub fn delete(dir: &Path, id: &str) -> Result<(), String> {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut f = load(dir);
    f.chats.retain(|c| c.id != id);
    save(dir, &f)
}

pub fn clear(dir: &Path) {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _ = std::fs::remove_file(path(dir));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("chats-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    fn m(role: &str, text: &str) -> Message {
        Message {
            role: role.into(),
            text: text.into(),
            steps: vec![],
            sources: vec![],
        }
    }

    #[test]
    fn a_conversation_is_saved_listed_reopened_and_deleted() {
        let d = tmp("basic");
        assert!(list(&d).is_empty());
        let id = append(&d, None, m("user", "  what is   the speed of light? ")).unwrap();
        append(&d, Some(&id), m("assistant", "About 300,000 km/s.")).unwrap();
        let l = list(&d);
        assert_eq!(l.len(), 1);
        assert_eq!(l[0].title, "what is the speed of light?");
        assert_eq!(l[0].count, 2);
        let c = get(&d, &id).unwrap();
        assert_eq!(c.messages[1].role, "assistant");
        // a second chat; the newest is listed first
        std::thread::sleep(std::time::Duration::from_millis(1100));
        let id2 = append(&d, None, m("user", "another")).unwrap();
        assert_ne!(id, id2);
        assert_eq!(list(&d)[0].id, id2);
        delete(&d, &id).unwrap();
        assert!(get(&d, &id).is_none());
        assert_eq!(list(&d).len(), 1);
        clear(&d);
        assert!(list(&d).is_empty());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn limits_and_bad_input_are_handled() {
        let d = tmp("limits");
        let long = "x".repeat(MAX_TEXT + 500);
        let id = append(&d, None, m("user", &long)).unwrap();
        assert_eq!(
            get(&d, &id).unwrap().messages[0].text.chars().count(),
            MAX_TEXT
        );
        for i in 0..(MAX_MESSAGES + 5) {
            append(&d, Some(&id), m("assistant", &format!("a{i}"))).unwrap();
        }
        let c = get(&d, &id).unwrap();
        assert_eq!(c.messages.len(), MAX_MESSAGES);
        assert_eq!(
            c.messages.last().unwrap().text,
            format!("a{}", MAX_MESSAGES + 4)
        );
        // an unknown id starts a new chat; control characters and odd roles are cleaned
        let id2 = append(&d, Some("nope"), m("system", "he\u{0}llo")).unwrap();
        let c2 = get(&d, &id2).unwrap();
        assert_eq!(c2.messages[0].text, "hello");
        assert_eq!(c2.messages[0].role, "user");
        // a damaged file is an empty history, not a crash
        std::fs::write(d.join("chats.json"), "{ not json").unwrap();
        assert!(list(&d).is_empty());
        assert!(append(&d, None, m("user", "again")).is_ok());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn only_the_newest_hundred_chats_are_kept() {
        let d = tmp("many");
        for i in 0..(MAX_CHATS + 3) {
            append(&d, None, m("user", &format!("q{i}"))).unwrap();
        }
        assert_eq!(list(&d).len(), MAX_CHATS);
        let _ = std::fs::remove_dir_all(&d);
    }
}
