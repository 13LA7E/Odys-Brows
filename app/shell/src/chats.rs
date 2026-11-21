//! Saved assistant conversations (`chats.json` in the profile folder) and the pictures attached to
//! them (`chat-images/`).
//!
//! Plain JSON, written atomically; a damaged or missing file is an empty history. Limits keep it
//! small: 100 chats, 200 messages each, 20 000 characters per message, 4 pictures per message.
//! Private windows never use this. "Clear history on exit" removes both (see `privacy.rs`).

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub const MAX_CHATS: usize = 100;
pub const MAX_MESSAGES: usize = 200;
pub const MAX_IMAGES: usize = 4;
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
    /// File names in `chat-images/` (pictures the user attached).
    #[serde(default)]
    pub images: Vec<String>,
    /// What a vision model said about each picture, so it is not described again every turn.
    #[serde(default)]
    pub notes: Vec<String>,
    /// Seconds since 1970 (0 = unknown, for chats saved before this existed).
    #[serde(default)]
    pub time: i64,
}

impl Message {
    pub fn new(role: &str, text: &str) -> Self {
        Self {
            role: role.into(),
            text: text.into(),
            steps: vec![],
            sources: vec![],
            images: vec![],
            notes: vec![],
            time: 0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Chat {
    pub id: String,
    pub title: String,
    /// Seconds since 1970 of the last message.
    pub updated: i64,
    #[serde(default)]
    pub pinned: bool,
    pub messages: Vec<Message>,
}

/// A message as the UI gets it: pictures as `data:` URLs.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct MessageView {
    pub role: String,
    pub text: String,
    pub steps: Vec<String>,
    pub sources: Vec<Source>,
    pub images: Vec<String>,
    pub time: i64,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ChatView {
    pub id: String,
    pub title: String,
    pub updated: i64,
    pub pinned: bool,
    pub messages: Vec<MessageView>,
}

/// What the list in the UI needs.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Summary {
    pub id: String,
    pub title: String,
    pub updated: i64,
    pub pinned: bool,
    pub count: usize,
    /// For a search: a piece of the matching message.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub snippet: String,
}

#[derive(Default, Serialize, Deserialize)]
struct File {
    #[serde(default)]
    chats: Vec<Chat>,
}

fn path(dir: &Path) -> PathBuf {
    dir.join("chats.json")
}

fn images_dir(dir: &Path) -> PathBuf {
    dir.join("chat-images")
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

fn summary(c: &Chat, snippet: String) -> Summary {
    Summary {
        id: c.id.clone(),
        title: c.title.clone(),
        updated: c.updated,
        pinned: c.pinned,
        count: c.messages.len(),
        snippet,
    }
}

/// Pinned first, then newest first.
fn sorted(mut v: Vec<Summary>) -> Vec<Summary> {
    v.sort_by_key(|s| (std::cmp::Reverse(s.pinned), std::cmp::Reverse(s.updated)));
    v
}

pub fn list(dir: &Path) -> Vec<Summary> {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    sorted(
        load(dir)
            .chats
            .iter()
            .map(|c| summary(c, String::new()))
            .collect(),
    )
}

/// Chats whose title or any message contains `query` (case-insensitive), with a snippet.
pub fn search(dir: &Path, query: &str) -> Vec<Summary> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return list(dir);
    }
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut out = Vec::new();
    for c in &load(dir).chats {
        let mut snippet = None;
        if c.title.to_lowercase().contains(&q) {
            snippet = Some(String::new());
        }
        for m in &c.messages {
            // Work on characters so a match can never split a multi-byte letter.
            let chars: Vec<char> = m.text.chars().collect();
            let lower: Vec<char> = m.text.to_lowercase().chars().collect();
            let qc: Vec<char> = q.chars().collect();
            if lower.len() == chars.len()
                && let Some(at) = lower.windows(qc.len()).position(|w| w == qc.as_slice())
            {
                let start = at.saturating_sub(30);
                let piece: String = chars[start..].iter().take(90).collect();
                snippet = Some(piece.split_whitespace().collect::<Vec<_>>().join(" "));
                break;
            }
        }
        if let Some(s) = snippet {
            out.push(summary(c, s));
        }
    }
    sorted(out)
}

pub fn get(dir: &Path, id: &str) -> Option<Chat> {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    load(dir).chats.into_iter().find(|c| c.id == id)
}

/// The chat for the UI, with its pictures read from disk.
pub fn get_view(dir: &Path, id: &str) -> Option<ChatView> {
    let c = get(dir, id)?;
    Some(ChatView {
        id: c.id,
        title: c.title,
        updated: c.updated,
        pinned: c.pinned,
        messages: c
            .messages
            .iter()
            .map(|m| MessageView {
                role: m.role.clone(),
                text: m.text.clone(),
                steps: m.steps.clone(),
                sources: m.sources.clone(),
                images: m
                    .images
                    .iter()
                    .filter_map(|n| image_data_url(dir, n))
                    .collect(),
                time: m.time,
            })
            .collect(),
    })
}

fn clean_message(msg: Message) -> Message {
    Message {
        role: if msg.role == "assistant" {
            "assistant".into()
        } else {
            "user".into()
        },
        text: clean_text(&msg.text),
        steps: msg
            .steps
            .iter()
            .take(20)
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
        images: msg
            .images
            .into_iter()
            .filter(|n| valid_image_name(n))
            .take(MAX_IMAGES)
            .collect(),
        notes: msg
            .notes
            .iter()
            .take(MAX_IMAGES)
            .map(|n| clean_text(n).chars().take(2000).collect())
            .collect(),
        time: if msg.time > 0 { msg.time } else { now() },
    }
}

/// Adds a message to chat `id` (a new chat when `id` is `None` or unknown). Returns the chat id and
/// the index of the new message.
pub fn append(dir: &Path, id: Option<&str>, msg: Message) -> Result<(String, usize), String> {
    let result = {
        let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut f = load(dir);
        let msg = clean_message(msg);
        let idx = id.and_then(|id| f.chats.iter().position(|c| c.id == id));
        let idx = match idx {
            Some(i) => i,
            None => {
                let id = new_id(&f);
                f.chats.push(Chat {
                    id,
                    title: title_of(&msg.text),
                    updated: now(),
                    pinned: false,
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
        let at = chat.messages.len() - 1;
        if f.chats.len() > MAX_CHATS {
            f.chats
                .sort_by_key(|c| (std::cmp::Reverse(c.pinned), std::cmp::Reverse(c.updated)));
            f.chats.truncate(MAX_CHATS);
        }
        save(dir, &f)?;
        (chat_id, at)
    };
    gc_images(dir);
    Ok(result)
}

fn with_chat<T>(dir: &Path, id: &str, f: impl FnOnce(&mut Chat) -> T) -> Result<T, String> {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut file = load(dir);
    let chat = file
        .chats
        .iter_mut()
        .find(|c| c.id == id)
        .ok_or("No such chat.")?;
    let out = f(chat);
    save(dir, &file)?;
    Ok(out)
}

pub fn rename(dir: &Path, id: &str, title: &str) -> Result<(), String> {
    let t = title_of(&clean_text(title));
    with_chat(dir, id, |c| c.title = t)
}

pub fn set_pinned(dir: &Path, id: &str, pinned: bool) -> Result<(), String> {
    with_chat(dir, id, |c| c.pinned = pinned)
}

/// Keeps only the first `keep` messages (used to edit a question).
pub fn truncate(dir: &Path, id: &str, keep: usize) -> Result<(), String> {
    with_chat(dir, id, |c| c.messages.truncate(keep))?;
    gc_images(dir);
    Ok(())
}

/// Removes the answer(s) after the last question, so it can be asked again.
pub fn drop_trailing_assistant(dir: &Path, id: &str) -> Result<(), String> {
    with_chat(dir, id, |c| {
        while c.messages.last().is_some_and(|m| m.role == "assistant") {
            c.messages.pop();
        }
    })
}

pub fn set_notes(dir: &Path, id: &str, index: usize, notes: Vec<String>) -> Result<(), String> {
    with_chat(dir, id, |c| {
        if let Some(m) = c.messages.get_mut(index) {
            m.notes = notes
                .iter()
                .take(MAX_IMAGES)
                .map(|n| clean_text(n).chars().take(2000).collect())
                .collect();
        }
    })
}

pub fn delete(dir: &Path, id: &str) -> Result<(), String> {
    {
        let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut f = load(dir);
        f.chats.retain(|c| c.id != id);
        save(dir, &f)?;
    }
    gc_images(dir);
    Ok(())
}

pub fn clear(dir: &Path) {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _ = std::fs::remove_file(path(dir));
    let _ = std::fs::remove_dir_all(images_dir(dir));
}

/// The conversation as a Markdown document.
pub fn export_markdown(chat: &Chat) -> String {
    let mut out = format!("# {}\n\n", chat.title);
    for m in &chat.messages {
        out.push_str(if m.role == "user" {
            "**You**\n\n"
        } else {
            "**Assistant**\n\n"
        });
        out.push_str(m.text.trim());
        out.push_str("\n\n");
        if !m.images.is_empty() {
            out.push_str(&format!("_({} picture(s) attached)_\n\n", m.images.len()));
        }
        for s in &m.sources {
            out.push_str(&format!("- [{}]({})\n", s.title, s.url));
        }
        if !m.sources.is_empty() {
            out.push('\n');
        }
    }
    out
}

// ---- pictures ----

fn valid_image_name(n: &str) -> bool {
    let Some((stem, ext)) = n.rsplit_once('.') else {
        return false;
    };
    stem.len() == 17
        && stem.starts_with('i')
        && stem[1..].chars().all(|c| c.is_ascii_hexdigit())
        && matches!(ext, "jpg" | "png" | "webp")
}

/// Checks a picture sent as a `data:` URL (type, size and real file signature, as for the wallpaper)
/// and saves it. Returns its file name.
pub fn save_image(dir: &Path, data_url: &str) -> Result<String, String> {
    let (mime, bytes) = crate::wallpaper::decode(data_url)?;
    let ext = match mime {
        "image/jpeg" => "jpg",
        "image/webp" => "webp",
        _ => "png",
    };
    let d = images_dir(dir);
    std::fs::create_dir_all(&d).map_err(|e| format!("Couldn't save the picture: {e}"))?;
    let mut n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|t| t.as_nanos() as u64)
        .unwrap_or(1);
    loop {
        let name = format!("i{n:016x}.{ext}");
        let p = d.join(&name);
        if !p.exists() {
            std::fs::write(&p, &bytes).map_err(|e| format!("Couldn't save the picture: {e}"))?;
            return Ok(name);
        }
        n = n.wrapping_add(1);
    }
}

pub fn image_data_url(dir: &Path, name: &str) -> Option<String> {
    use base64::Engine as _;
    if !valid_image_name(name) {
        return None;
    }
    let bytes = std::fs::read(images_dir(dir).join(name)).ok()?;
    let mime = match name.rsplit('.').next()? {
        "jpg" => "image/jpeg",
        "webp" => "image/webp",
        _ => "image/png",
    };
    Some(format!(
        "data:{mime};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}

/// Removes pictures no message refers to any more.
fn gc_images(dir: &Path) {
    let used: std::collections::HashSet<String> = {
        let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        load(dir)
            .chats
            .iter()
            .flat_map(|c| c.messages.iter().flat_map(|m| m.images.clone()))
            .collect()
    };
    let Ok(rd) = std::fs::read_dir(images_dir(dir)) else {
        return;
    };
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if !used.contains(&name) {
            let _ = std::fs::remove_file(e.path());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine as _;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("chats-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    fn m(role: &str, text: &str) -> Message {
        Message::new(role, text)
    }

    fn png_url() -> String {
        let bytes: &[u8] = &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 0];
        format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(bytes)
        )
    }

    #[test]
    fn a_conversation_is_saved_listed_reopened_and_deleted() {
        let d = tmp("basic");
        assert!(list(&d).is_empty());
        let (id, _) = append(&d, None, m("user", "  what is   the speed of light? ")).unwrap();
        append(&d, Some(&id), m("assistant", "About 300,000 km/s.")).unwrap();
        let l = list(&d);
        assert_eq!(l.len(), 1);
        assert_eq!(l[0].title, "what is the speed of light?");
        assert_eq!(l[0].count, 2);
        let c = get(&d, &id).unwrap();
        assert_eq!(c.messages[1].role, "assistant");
        assert!(c.messages[1].time > 0);
        std::thread::sleep(std::time::Duration::from_millis(1100));
        let (id2, _) = append(&d, None, m("user", "another")).unwrap();
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
        let (id, _) = append(&d, None, m("user", &long)).unwrap();
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
        let (id2, _) = append(&d, Some("nope"), m("system", "he\u{0}llo")).unwrap();
        let c2 = get(&d, &id2).unwrap();
        assert_eq!(c2.messages[0].text, "hello");
        assert_eq!(c2.messages[0].role, "user");
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

    #[test]
    fn rename_pin_truncate_and_search() {
        let d = tmp("ops");
        let (a, _) = append(&d, None, m("user", "how do rockets work")).unwrap();
        append(
            &d,
            Some(&a),
            m("assistant", "They push gas out of a nozzle very fast."),
        )
        .unwrap();
        append(&d, Some(&a), m("user", "and orbits?")).unwrap();
        let (b, _) = append(&d, None, m("user", "pasta recipe")).unwrap();
        rename(&d, &b, "  Dinner\nideas ").unwrap();
        assert_eq!(get(&d, &b).unwrap().title, "Dinner ideas");
        set_pinned(&d, &b, true).unwrap();
        assert_eq!(list(&d)[0].id, b, "pinned chats come first");
        let hits = search(&d, "NOZZLE");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, a);
        assert!(
            hits[0].snippet.to_lowercase().contains("nozzle"),
            "{:?}",
            hits[0].snippet
        );
        assert_eq!(search(&d, "dinner").len(), 1, "titles match too");
        assert!(search(&d, "zebra").is_empty());
        // a non-English letter before the match cannot break the snippet
        append(&d, Some(&b), m("user", "Käse Käse Käse Käse käse und Brot")).unwrap();
        assert_eq!(search(&d, "brot").len(), 1);
        truncate(&d, &a, 1).unwrap();
        assert_eq!(get(&d, &a).unwrap().messages.len(), 1);
        append(&d, Some(&a), m("assistant", "x")).unwrap();
        drop_trailing_assistant(&d, &a).unwrap();
        assert_eq!(get(&d, &a).unwrap().messages.len(), 1);
        assert!(rename(&d, "nope", "x").is_err());
        let md = export_markdown(&get(&d, &a).unwrap());
        assert!(md.starts_with("# how do rockets work") && md.contains("**You**"));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn pictures_are_checked_saved_shown_and_cleaned_up() {
        let d = tmp("pics");
        let name = save_image(&d, &png_url()).unwrap();
        assert!(valid_image_name(&name), "{name}");
        assert!(save_image(&d, "data:text/html;base64,PGI+").is_err());
        let mut msg = m("user", "look at this");
        msg.images = vec![name.clone(), "../../evil.png".into(), "nope".into()];
        let (id, at) = append(&d, None, msg).unwrap();
        let c = get(&d, &id).unwrap();
        assert_eq!(
            c.messages[at].images,
            vec![name.clone()],
            "bad names are dropped"
        );
        let v = get_view(&d, &id).unwrap();
        assert!(v.messages[0].images[0].starts_with("data:image/png;base64,"));
        assert!(image_data_url(&d, "../chats.json").is_none());
        // a picture nobody refers to is removed; one in use stays
        let orphan = save_image(&d, &png_url()).unwrap();
        append(&d, Some(&id), m("assistant", "ok")).unwrap();
        assert!(!images_dir(&d).join(&orphan).exists());
        assert!(images_dir(&d).join(&name).exists());
        set_notes(&d, &id, at, vec!["a red square".into()]).unwrap();
        assert_eq!(
            get(&d, &id).unwrap().messages[at].notes,
            vec!["a red square".to_string()]
        );
        delete(&d, &id).unwrap();
        assert!(
            !images_dir(&d).join(&name).exists(),
            "deleting the chat deletes its pictures"
        );
        let _ = std::fs::remove_dir_all(&d);
    }
}
