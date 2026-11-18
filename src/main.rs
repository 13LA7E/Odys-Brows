// Odyssey - an AI-first browser that talks to your own Odysseus server.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod shields;
mod vault;

use std::{collections::HashMap, path::PathBuf, sync::{atomic::Ordering, Mutex}, time::Duration};

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::{
    webview::{DownloadEvent, NewWindowResponse, PageLoadEvent, WebviewBuilder},
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, State, Url, WebviewUrl,
};
use tauri_plugin_single_instance::init as single_instance_init;
use tokio::sync::oneshot;

use shields::Shields;
use vault::{vault_add, vault_delete, vault_list, vault_list_for_origin, vault_reveal, vault_update};

const OFFSCREEN: f64 = -30000.0;
const UI_LABEL: &str = "ui";

/// Wrapper for scripts Odyssey runs inside a tab. The result travels back
/// through document.title in small chunks, so web pages never get access to
/// Odyssey's internal commands. `__BODY__` must be an expression (or promise).
const RUN_TEMPLATE: &str = r#"
(async function () {
  var original = document.title;
  var value;
  try { value = await (__BODY__); } catch (e) { value = { error: String(e) }; }
  var payload = encodeURIComponent(JSON.stringify(value === undefined ? null : value));
  var CH = 1800, n = Math.max(1, Math.ceil(payload.length / CH)), i = 0;
  function step() {
    if (i < n) {
      document.title = 'ODY~' + i + '~' + n + '~' + payload.substr(i * CH, CH);
      i++;
      setTimeout(step, 25);
    } else {
      document.title = original;
    }
  }
  step();
})();
"#;

/// Reads the visible text of a page.
const EXTRACT_BODY: &str = r#"(function () {
  var text = (document.body ? document.body.innerText : '') || '';
  text = text.replace(/[ \t]+\n/g, '\n').replace(/\n{3,}/g, '\n\n').slice(0, 12000);
  var m = document.querySelector('meta[name="description"]');
  return { u: location.href, t: document.title, d: m ? (m.getAttribute('content') || '') : '', x: text };
})()"#;

/// Find in page. Uses the CSS Custom Highlight API so every match is painted, with the current one stronger.
const FIND_BODY: &str = r#"(function () {
  var q = __Q__, fwd = __FWD__, restart = __RESTART__;
  var st = window.__odyFind;
  if (!q) { if (window.CSS && CSS.highlights) { CSS.highlights.delete('ody-find'); CSS.highlights.delete('ody-find-cur'); } window.__odyFind = null; return { count: 0, cur: 0 }; }
  if (!window.CSS || !CSS.highlights || typeof Highlight === 'undefined') {
    var n = 0, text = (document.body ? document.body.innerText : '').toLowerCase(), i0 = 0, lq = q.toLowerCase();
    while ((i0 = text.indexOf(lq, i0)) !== -1) { n++; i0 += lq.length; }
    if (n) window.find(q, false, !fwd, true);
    return { count: n, cur: 0 };
  }
  if (!st || st.q !== q || restart) {
    var ranges = [], lq2 = q.toLowerCase();
    var w = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT, { acceptNode: function (nd) {
      var p = nd.parentElement;
      if (!p || /^(SCRIPT|STYLE|NOSCRIPT|TEXTAREA)$/.test(p.tagName) || !p.getClientRects().length) return NodeFilter.FILTER_SKIP;
      return NodeFilter.FILTER_ACCEPT;
    } });
    var node;
    while ((node = w.nextNode()) && ranges.length < 3000) {
      var low = node.nodeValue.toLowerCase(), at = 0;
      while ((at = low.indexOf(lq2, at)) !== -1) {
        var r = document.createRange(); r.setStart(node, at); r.setEnd(node, at + q.length);
        ranges.push(r); at += q.length;
      }
    }
    st = window.__odyFind = { q: q, ranges: ranges, i: -1 };
    restart = true;
  }
  var rs = st.ranges;
  if (!document.getElementById('ody-find-style')) {
    var s = document.createElement('style'); s.id = 'ody-find-style';
    s.textContent = '::highlight(ody-find){background-color:#f6d365;color:#111}::highlight(ody-find-cur){background-color:#ff8a3d;color:#111}';
    document.documentElement.appendChild(s);
  }
  if (!rs.length) { CSS.highlights.delete('ody-find'); CSS.highlights.delete('ody-find-cur'); return { count: 0, cur: 0 }; }
  st.i = restart ? 0 : (st.i + (fwd ? 1 : -1) + rs.length) % rs.length;
  CSS.highlights.set('ody-find', new Highlight(...rs));
  CSS.highlights.set('ody-find-cur', new Highlight(rs[st.i]));
  var el = rs[st.i].startContainer.parentElement;
  if (el && el.scrollIntoView) el.scrollIntoView({ block: 'center', inline: 'nearest' });
  return { count: rs.length, cur: st.i + 1 };
})()"#;

const FIND_CLEAR: &str = r#"(function () { if (window.CSS && CSS.highlights) { CSS.highlights.delete('ody-find'); CSS.highlights.delete('ody-find-cur'); } window.__odyFind = null; })()"#;

/// Pulls the main readable content out of a page for Reader mode.
const READER_BODY: &str = r#"(function () {
  var root = document.querySelector('article') || document.querySelector('main') || document.querySelector('[role="main"]');
  if (!root) {
    var best = null, bestLen = 0;
    document.querySelectorAll('div,section').forEach(function (d) {
      var len = 0;
      d.querySelectorAll(':scope > p').forEach(function (p) { len += (p.innerText || '').length; });
      if (len > bestLen) { bestLen = len; best = d; }
    });
    root = best || document.body;
  }
  var blocks = [], total = 0;
  root.querySelectorAll('h1,h2,h3,h4,p,li,blockquote,pre').forEach(function (el) {
    if (total > 80000 || el.closest('nav,aside,footer,header,form,script,style,noscript')) return;
    var t = (el.innerText || '').replace(/\s+/g, ' ').trim();
    var k = el.tagName.toLowerCase();
    if (t.length < 2 || (k === 'p' && t.length < 20)) return;
    blocks.push({ k: k, t: t });
    total += t.length;
  });
  var h1 = document.querySelector('h1');
  return { u: location.href, title: (h1 && h1.innerText.trim()) || document.title, site: location.hostname, blocks: blocks.slice(0, 500) };
})()"#;

/// Asks the page's biggest playing (or first) video to enter picture-in-picture.
const PIP_BODY: &str = r#"(async function () {
  var vids = Array.prototype.slice.call(document.querySelectorAll('video'));
  if (!vids.length) return { ok: false, error: 'No video on this page.' };
  vids.sort(function (a, b) { return (b.videoWidth * b.videoHeight) - (a.videoWidth * a.videoHeight); });
  var v = vids.filter(function (x) { return !x.paused; })[0] || vids[0];
  try {
    if (document.pictureInPictureElement) { await document.exitPictureInPicture(); return { ok: true, closed: true }; }
    await v.requestPictureInPicture();
    return { ok: true };
  } catch (e) { return { ok: false, error: String(e && e.message || e) }; }
})()"#;

/// Asks the signed-in Odysseus page to mint a chat-only token for Odyssey.
const PAIR_BODY: &str = r#"(async function () {
  var fd = new FormData();
  fd.append('name', 'Odyssey browser');
  fd.append('scopes', 'chat');
  var r = await fetch('/api/tokens', { method: 'POST', body: fd, credentials: 'same-origin' });
  if (!r.ok) return { error: r.status };
  var j = await r.json();
  return { token: j.token || '' };
})()"#;

#[derive(Clone, Copy, Default, Deserialize, Serialize, Debug)]
struct Rect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(default)]
struct Config {
    base_url: String,
    token: String,
    model: String,
    endpoint_id: String,
    home_search: String,
    shields_enabled: bool,
    shield_allow: Vec<String>,
    cookie_banners: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            base_url: "http://127.0.0.1:7000".into(),
            token: String::new(),
            model: "qwen3.5:9b".into(),
            endpoint_id: String::new(),
            home_search: "https://duckduckgo.com/?q=".into(),
            shields_enabled: true,
            shield_allow: Vec::new(),
            cookie_banners: false,
        }
    }
}

struct Extract {
    parts: Vec<Option<String>>,
    tx: Option<oneshot::Sender<String>>,
}

#[derive(Default)]
struct AppState {
    rect: Mutex<Rect>,
    active: Mutex<Option<String>>,
    extracts: Mutex<HashMap<String, Extract>>,
    sessions: Mutex<HashMap<String, String>>,
    config: Mutex<Config>,
    http: reqwest::Client,
}

fn tab_label(id: &str) -> String {
    format!("tab-{id}")
}

fn id_of(label: &str) -> String {
    label.trim_start_matches("tab-").to_string()
}

fn config_path(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_config_dir().ok().map(|d| d.join("config.json"))
}

fn load_config(app: &AppHandle) -> Config {
    config_path(app)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_config(app: &AppHandle, cfg: &Config) -> Result<(), String> {
    let path = config_path(app).ok_or("no config dir")?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let s = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    std::fs::write(path, s).map_err(|e| e.to_string())
}

/// Show one tab's webview inside the content area and park every other tab off-screen.
fn arrange(app: &AppHandle, state: &AppState) {
    let r = *state.rect.lock().unwrap();
    let active = state.active.lock().unwrap().clone();
    for (label, wv) in app.webviews() {
        if !label.starts_with("tab-") {
            continue;
        }
        let show = active.as_ref().map(|a| tab_label(a) == label).unwrap_or(false);
        let x = if show { r.x } else { OFFSCREEN };
        let _ = wv.set_position(LogicalPosition::new(x, r.y));
        let _ = wv.set_size(LogicalSize::new(r.w.max(1.0), r.h.max(1.0)));
    }
}

fn handle_chunk(app: &AppHandle, id: &str, rest: &str) {
    let mut it = rest.splitn(3, '~');
    let (i, n, data) = match (it.next(), it.next(), it.next()) {
        (Some(a), Some(b), Some(c)) => (
            a.parse::<usize>().unwrap_or(0),
            b.parse::<usize>().unwrap_or(1).max(1),
            c.to_string(),
        ),
        _ => return,
    };
    let state = app.state::<AppState>();
    let mut map = state.extracts.lock().unwrap();
    let done = if let Some(ex) = map.get_mut(id) {
        if ex.parts.len() != n {
            ex.parts = vec![None; n];
        }
        if i < n {
            ex.parts[i] = Some(data);
        }
        ex.parts.iter().all(|p| p.is_some())
    } else {
        false
    };
    if done {
        if let Some(mut ex) = map.remove(id) {
            let joined: String = ex.parts.iter().map(|p| p.clone().unwrap_or_default()).collect();
            let decoded = percent_encoding::percent_decode_str(&joined)
                .decode_utf8_lossy()
                .to_string();
            if let Some(tx) = ex.tx.take() {
                let _ = tx.send(decoded);
            }
        }
    }
}

// ---------------------------------------------------------------- tabs

/// Hosts whose sign-in popups must stay real popups (they talk to window.opener).
fn is_oauth_host(url: &Url) -> bool {
    let host = url.host_str().unwrap_or("");
    let path = url.path();
    host == "accounts.google.com"
        || host == "login.microsoftonline.com"
        || host == "login.live.com"
        || host == "appleid.apple.com"
        || (host == "github.com" && path.starts_with("/login"))
}

/// `dir/name`, or `dir/name (2).ext` etc. when the file already exists.
fn unique_path(dir: &std::path::Path, name: &str) -> PathBuf {
    let first = dir.join(name);
    if !first.exists() {
        return first;
    }
    let p = std::path::Path::new(name);
    let stem = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "download".into());
    let ext = p.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    for i in 2..1000 {
        let c = dir.join(format!("{stem} ({i}){ext}"));
        if !c.exists() {
            return c;
        }
    }
    first
}

/// Reveal a downloaded file in File Explorer.
#[tauri::command]
async fn show_in_folder(path: String) -> Result<(), String> {
    #[cfg(windows)]
    {
        let p = PathBuf::from(&path);
        if !p.exists() {
            return Err("That file is gone.".into());
        }
        std::process::Command::new("explorer")
            .arg(format!("/select,{}", p.display()))
            .spawn()
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        Err("Only available on Windows.".into())
    }
}

/// Open a downloaded file with its default app.
#[tauri::command]
async fn open_download(path: String) -> Result<(), String> {
    #[cfg(windows)]
    {
        let p = PathBuf::from(&path);
        if !p.exists() {
            return Err("That file is gone.".into());
        }
        std::process::Command::new("cmd")
            .args(["/C", "start", "", &path])
            .spawn()
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        Err("Only available on Windows.".into())
    }
}

/// Give keyboard focus back to Odyssey's own chrome (find bar, address bar, palette).
#[tauri::command]
async fn focus_ui(app: AppHandle) -> Result<(), String> {
    if let Some(wv) = app.get_webview(UI_LABEL) {
        wv.set_focus().map_err(|e| e.to_string())?;
        shields::win::focus(&wv);
    }
    Ok(())
}

/// Mute or unmute the media elements on a page. Covers <video>/<audio>, not Web Audio.
#[tauri::command]
async fn tab_mute(app: AppHandle, id: String, muted: bool) -> Result<(), String> {
    let wv = app.get_webview(&tab_label(&id)).ok_or("tab not found")?;
    let js = format!(
        "(function(){{ window.__odyMuted = {muted}; document.querySelectorAll('video,audio').forEach(function(m){{ m.muted = {muted}; }}); \
         if (!window.__odyMuteHook) {{ window.__odyMuteHook = true; document.addEventListener('play', function(e){{ if (window.__odyMuted && e.target && e.target.muted === false) e.target.muted = true; }}, true); }} }})()"
    );
    wv.eval(js).map_err(|e| e.to_string())
}

/// Picture-in-picture for the page's main video.
#[tauri::command]
async fn tab_pip(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<Value, String> {
    run_in_tab(&app, &state, &id, PIP_BODY).await
}

/// Reader mode: the readable blocks (headings, paragraphs, lists) of the page.
#[tauri::command]
async fn tab_reader(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<Value, String> {
    run_in_tab(&app, &state, &id, READER_BODY).await
}

/// Find in page: highlight every match, mark the current one, scroll to it, report the counts.
#[tauri::command]
async fn tab_find(app: AppHandle, state: State<'_, AppState>, id: String, query: String, forward: bool, restart: bool) -> Result<Value, String> {
    let q = serde_json::to_string(&query).map_err(|e| e.to_string())?;
    let body = FIND_BODY
        .replace("__Q__", &q)
        .replace("__FWD__", if forward { "true" } else { "false" })
        .replace("__RESTART__", if restart { "true" } else { "false" });
    run_in_tab(&app, &state, &id, &body).await
}

/// Clear the find highlight.
#[tauri::command]
async fn tab_find_clear(app: AppHandle, id: String) -> Result<(), String> {
    let wv = app.get_webview(&tab_label(&id)).ok_or("tab not found")?;
    wv.eval(FIND_CLEAR).map_err(|e| e.to_string())
}

#[tauri::command]
async fn set_layout(app: AppHandle, state: State<'_, AppState>, rect: Rect) -> Result<(), String> {
    *state.rect.lock().unwrap() = rect;
    arrange(&app, &state);
    Ok(())
}

#[tauri::command]
async fn tab_create(app: AppHandle, state: State<'_, AppState>, id: String, url: String) -> Result<(), String> {
    let window = app.get_window("main").ok_or("main window missing")?;
    let parsed: Url = url.parse().map_err(|e| format!("bad url: {e}"))?;
    let app_for_popups = app.clone();
    let builder = WebviewBuilder::new(tab_label(&id), WebviewUrl::External(parsed))
        .on_page_load(|wv, payload| {
            let loading = matches!(payload.event(), PageLoadEvent::Started);
            let _ = wv.app_handle().emit_to(
                UI_LABEL,
                "tab-load",
                json!({ "id": id_of(wv.label()), "url": payload.url().to_string(), "loading": loading }),
            );
        })
        .on_document_title_changed(|wv, title| {
            let id = id_of(wv.label());
            let app = wv.app_handle().clone();
            if let Some(rest) = title.strip_prefix("ODY~") {
                handle_chunk(&app, &id, rest);
            } else {
                let _ = app.emit_to(UI_LABEL, "tab-title", json!({ "id": id, "title": title }));
            }
        })
        .on_new_window(move |url, _features| {
            // Sign-in popups need a real window so window.opener keeps working.
            if is_oauth_host(&url) {
                return NewWindowResponse::Allow;
            }
            let _ = app_for_popups.emit_to(UI_LABEL, "open-tab", json!({ "url": url.to_string() }));
            NewWindowResponse::Deny
        })
        .on_download(move |wv, event| {
            let app = wv.app_handle().clone();
            match event {
                DownloadEvent::Requested { url, destination } => {
                    let dir = app.path().download_dir().unwrap_or_else(|_| std::env::temp_dir());
                    let name = destination
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .filter(|n| !n.is_empty())
                        .unwrap_or_else(|| "download".to_string());
                    let target = unique_path(&dir, &name);
                    *destination = target.clone();
                    let _ = app.emit_to(
                        UI_LABEL,
                        "download-started",
                        json!({
                            "id": target.to_string_lossy(),
                            "url": url.to_string(),
                            "name": target.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or(name),
                            "path": target.to_string_lossy(),
                        }),
                    );
                }
                DownloadEvent::Finished { path, success, .. } => {
                    let p = path.map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
                    let _ = app.emit_to(UI_LABEL, "download-finished", json!({ "id": p, "path": p, "success": success }));
                }
                _ => {}
            }
            true
        });
    let r = *state.rect.lock().unwrap();
    let webview = window
        .add_child(builder, LogicalPosition::new(OFFSCREEN, r.y), LogicalSize::new(r.w.max(1.0), r.h.max(1.0)))
        .map_err(|e| e.to_string())?;
    shields::win::attach(&webview, app.clone(), id.clone());
    *state.active.lock().unwrap() = Some(id);
    arrange(&app, &state);
    Ok(())
}

#[tauri::command]
async fn tab_activate(app: AppHandle, state: State<'_, AppState>, id: Option<String>) -> Result<(), String> {
    *state.active.lock().unwrap() = id.clone();
    arrange(&app, &state);
    if let Some(id) = id {
        if let Some(wv) = app.get_webview(&tab_label(&id)) {
            let _ = wv.set_focus();
        }
    }
    Ok(())
}

#[tauri::command]
async fn tab_navigate(app: AppHandle, id: String, url: String) -> Result<(), String> {
    let wv = app.get_webview(&tab_label(&id)).ok_or("tab not found")?;
    let parsed: Url = url.parse().map_err(|e| format!("bad url: {e}"))?;
    wv.navigate(parsed).map_err(|e| e.to_string())
}

#[tauri::command]
async fn tab_action(app: AppHandle, id: String, action: String) -> Result<(), String> {
    let wv = app.get_webview(&tab_label(&id)).ok_or("tab not found")?;
    let js = match action.as_str() {
        "back" => "history.back()",
        "forward" => "history.forward()",
        "reload" => "location.reload()",
        "stop" => "window.stop()",
        _ => return Err("unknown action".into()),
    };
    wv.eval(js).map_err(|e| e.to_string())
}

#[tauri::command]
async fn tab_close(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<(), String> {
    if let Some(wv) = app.get_webview(&tab_label(&id)) {
        wv.close().map_err(|e| e.to_string())?;
    }
    state.extracts.lock().unwrap().remove(&id);
    app.state::<Shields>().per_tab.lock().unwrap().remove(&id);
    let mut active = state.active.lock().unwrap();
    if active.as_deref() == Some(id.as_str()) {
        *active = None;
    }
    Ok(())
}

/// Run a script inside a tab and wait for its JSON result.
async fn run_in_tab(app: &AppHandle, state: &AppState, id: &str, body: &str) -> Result<Value, String> {
    let wv = app.get_webview(&tab_label(id)).ok_or("tab not found")?;
    let (tx, rx) = oneshot::channel();
    state
        .extracts
        .lock()
        .unwrap()
        .insert(id.to_string(), Extract { parts: vec![], tx: Some(tx) });
    wv.eval(RUN_TEMPLATE.replace("__BODY__", body)).map_err(|e| e.to_string())?;
    match tokio::time::timeout(Duration::from_secs(12), rx).await {
        Ok(Ok(s)) => serde_json::from_str(&s).map_err(|e| e.to_string()),
        _ => {
            state.extracts.lock().unwrap().remove(id);
            Err("The page didn't answer (it may block scripts or still be loading).".into())
        }
    }
}

/// Read the current page's title, URL and visible text.
#[tauri::command]
async fn tab_extract(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<Value, String> {
    run_in_tab(&app, &state, &id, EXTRACT_BODY).await
}

/// Fills the saved username/password for `entry_id` into the password
/// field (and, if one is nearby, the username/email field) on the page
/// currently open in tab `id`. The decrypted password only ever leaves
/// Rust as part of this one script run -- it's not returned to the
/// frontend and not logged.
const AUTOFILL_BODY: &str = r#"(function () {
  var user = __USER_JSON__;
  var pass = __PASS_JSON__;
  var pwd = document.querySelector('input[type="password"]');
  if (!pwd) return { filled: false, reason: 'no password field on this page' };
  function setVal(el, val) {
    var proto = Object.getPrototypeOf(el);
    var desc = Object.getOwnPropertyDescriptor(proto, 'value');
    if (desc && desc.set) { desc.set.call(el, val); } else { el.value = val; }
    el.dispatchEvent(new Event('input', { bubbles: true }));
    el.dispatchEvent(new Event('change', { bubbles: true }));
  }
  setVal(pwd, pass);
  var userField = null;
  var scope = pwd.closest('form') || document;
  var candidates = scope.querySelectorAll('input[type="text"], input[type="email"], input:not([type])');
  for (var i = 0; i < candidates.length; i++) {
    if (candidates[i].offsetParent !== null) { userField = candidates[i]; break; }
  }
  if (userField) setVal(userField, user);
  return { filled: true, usedUsername: !!userField };
})()"#;

#[tauri::command]
async fn vault_autofill(app: AppHandle, state: State<'_, AppState>, id: String, entry_id: String) -> Result<Value, String> {
    let (username, password) = vault::get_entry_credentials(&app, &entry_id)?;
    let user_json = serde_json::to_string(&username).map_err(|e| e.to_string())?;
    let pass_json = serde_json::to_string(&password).map_err(|e| e.to_string())?;
    let body = AUTOFILL_BODY.replace("__USER_JSON__", &user_json).replace("__PASS_JSON__", &pass_json);
    run_in_tab(&app, &state, &id, &body).await
}

fn same_server(a: &Url, b: &Url) -> bool {
    let norm = |h: Option<&str>| match h {
        Some("localhost") | Some("[::1]") => "127.0.0.1".to_string(),
        Some(h) => h.to_lowercase(),
        None => String::new(),
    };
    a.scheme() == b.scheme() && norm(a.host_str()) == norm(b.host_str()) && a.port_or_known_default() == b.port_or_known_default()
}

/// Connect Odyssey to Odysseus using a tab where the user is signed in.
/// The new chat-only token is stored by Odyssey and never shown to the page UI.
#[tauri::command]
async fn pair_odysseus(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<String, String> {
    let mut cfg = state.config.lock().unwrap().clone();
    let base: Url = cfg.base_url.parse().map_err(|_| "The Odysseus address in Settings isn't valid.")?;
    let wv = app.get_webview(&tab_label(&id)).ok_or("tab not found")?;
    let here = wv.url().map_err(|e| e.to_string())?;
    if !same_server(&here, &base) {
        return Err("That tab isn't your Odysseus. Open Odysseus, sign in, then click Connect.".into());
    }
    if here.path().starts_with("/login") {
        return Err("Sign in to Odysseus in the tab first, then click Connect.".into());
    }
    let v = run_in_tab(&app, &state, &id, PAIR_BODY).await?;
    if let Some(code) = v.get("error") {
        return Err(match code.as_u64() {
            Some(401) => "Odysseus says you're not signed in. Sign in in the tab, then click Connect.".into(),
            Some(403) => "Only an Odysseus admin account can connect apps.".into(),
            _ => format!("Odysseus refused to create a token ({code})."),
        });
    }
    let token = v.get("token").and_then(|t| t.as_str()).unwrap_or("").to_string();
    if !token.starts_with("ody_") {
        return Err("Odysseus didn't return a token.".into());
    }
    cfg.token = token;
    save_config(&app, &cfg)?;
    *state.config.lock().unwrap() = cfg;
    state.sessions.lock().unwrap().clear();
    Ok("Connected to Odysseus.".into())
}

// ---------------------------------------------------------------- Odysseus

/// Settings as seen by the interface: the token itself never leaves Rust.
#[tauri::command]
async fn config_get(state: State<'_, AppState>) -> Result<Value, String> {
    let c = state.config.lock().unwrap().clone();
    Ok(json!({
        "base_url": c.base_url,
        "connected": !c.token.is_empty(),
        "model": c.model,
        "endpoint_id": c.endpoint_id,
        "home_search": c.home_search,
    }))
}

#[tauri::command]
async fn disconnect(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let mut cfg = state.config.lock().unwrap().clone();
    cfg.token.clear();
    save_config(&app, &cfg)?;
    *state.config.lock().unwrap() = cfg;
    state.sessions.lock().unwrap().clear();
    Ok(())
}

#[derive(Deserialize)]
struct ConfigUpdate {
    base_url: String,
    model: String,
    endpoint_id: String,
    home_search: String,
}

#[tauri::command]
async fn config_set(app: AppHandle, state: State<'_, AppState>, config: ConfigUpdate) -> Result<(), String> {
    let mut cfg = state.config.lock().unwrap().clone();
    let new_base = config.base_url.trim().trim_end_matches('/').to_string();
    if new_base != cfg.base_url {
        cfg.token.clear(); // a token only belongs to the server that issued it
    }
    cfg.base_url = new_base;
    cfg.model = config.model.trim().to_string();
    cfg.endpoint_id = config.endpoint_id.trim().to_string();
    cfg.home_search = config.home_search.trim().to_string();
    save_config(&app, &cfg)?;
    *state.config.lock().unwrap() = cfg;
    state.sessions.lock().unwrap().clear();
    Ok(())
}

#[tauri::command]
async fn ai_test(state: State<'_, AppState>) -> Result<String, String> {
    let cfg = state.config.lock().unwrap().clone();
    if cfg.token.is_empty() {
        return Err("Not connected yet. Click Connect.".into());
    }
    let resp = state
        .http
        .get(format!("{}/api/tokens/self", cfg.base_url))
        .bearer_auth(&cfg.token)
        .timeout(Duration::from_secs(8))
        .send()
        .await
        .map_err(|e| format!("Can't reach Odysseus at {}: {e}", cfg.base_url))?;
    let status = resp.status();
    if status.is_success() {
        Ok("Connected to Odysseus.".into())
    } else if status.as_u16() == 401 {
        Err("Odysseus no longer accepts Odyssey's token. Click Connect again.".into())
    } else {
        Err(format!("Odysseus answered {status}"))
    }
}

async fn ensure_session(state: &AppState, cfg: &Config, conv: &str) -> Result<String, String> {
    let existing = { state.sessions.lock().unwrap().get(conv).cloned() };
    if let Some(s) = existing {
        return Ok(s);
    }
    let mut form = reqwest::multipart::Form::new()
        .text("name", "Odyssey browser")
        .text("model", cfg.model.clone());
    if !cfg.endpoint_id.is_empty() {
        form = form.text("endpoint_id", cfg.endpoint_id.clone());
    }
    let resp = state
        .http
        .post(format!("{}/api/session", cfg.base_url))
        .bearer_auth(&cfg.token)
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("Can't reach Odysseus: {e}"))?;
    let status = resp.status();
    let body: Value = resp.json().await.unwrap_or(Value::Null);
    if !status.is_success() {
        return Err(format!("Couldn't start a chat ({status}): {body}"));
    }
    let id = body["id"].as_str().ok_or("Odysseus returned no session id")?.to_string();
    state.sessions.lock().unwrap().insert(conv.to_string(), id.clone());
    Ok(id)
}

/// Send a message to Odysseus and stream the reply to the UI as `ai-event`s.
#[tauri::command]
async fn ai_ask(
    app: AppHandle,
    state: State<'_, AppState>,
    conv: String,
    message: String,
    use_web: bool,
    context: Option<String>,
) -> Result<(), String> {
    let cfg = state.config.lock().unwrap().clone();
    if cfg.token.is_empty() {
        return Err("Odyssey isn't connected to Odysseus yet. Open Settings and click Connect.".into());
    }
    let session = ensure_session(&state, &cfg, &conv).await?;
    let mut form = reqwest::multipart::Form::new()
        .text("message", message)
        .text("session", session)
        .text("mode", "chat");
    if use_web {
        form = form.text("use_web", "true");
    }
    if let Some(ctx) = context.filter(|c| !c.trim().is_empty()) {
        // Sent as reference material, separate from the question, so a page's
        // text can't steer Odysseus into tool/agent mode.
        form = form.text("search_context", ctx);
    }
    let resp = state
        .http
        .post(format!("{}/api/chat_stream", cfg.base_url))
        .bearer_auth(&cfg.token)
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("Can't reach Odysseus: {e}"))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if status.as_u16() == 404 {
            state.sessions.lock().unwrap().remove(&conv);
        }
        return Err(format!("Odysseus error {status}: {}", text.chars().take(300).collect::<String>()));
    }
    let mut stream = resp.bytes_stream();
    let mut buf: Vec<u8> = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        buf.extend_from_slice(&chunk);
        while let Some(pos) = buf.iter().position(|b| *b == b'\n') {
            let line: Vec<u8> = buf.drain(..=pos).collect();
            let line = String::from_utf8_lossy(&line).trim().to_string();
            let Some(data) = line.strip_prefix("data:") else { continue };
            let data = data.trim();
            if data == "[DONE]" {
                continue;
            }
            if let Ok(ev) = serde_json::from_str::<Value>(data) {
                let _ = app.emit_to(UI_LABEL, "ai-event", json!({ "conv": conv, "ev": ev }));
            }
        }
    }
    Ok(())
}

#[tauri::command]
async fn ai_reset(state: State<'_, AppState>, conv: String) -> Result<(), String> {
    state.sessions.lock().unwrap().remove(&conv);
    Ok(())
}

// ---------------------------------------------------------------- window chrome

fn main_window(app: &AppHandle) -> Result<tauri::Window, String> {
    app.get_window("main").ok_or_else(|| "main window missing".to_string())
}

#[tauri::command]
async fn win_minimize(app: AppHandle) -> Result<(), String> {
    main_window(&app)?.minimize().map_err(|e| e.to_string())
}

#[tauri::command]
async fn win_toggle_maximize(app: AppHandle) -> Result<bool, String> {
    let w = main_window(&app)?;
    if w.is_maximized().unwrap_or(false) {
        w.unmaximize().map_err(|e| e.to_string())?;
        Ok(false)
    } else {
        w.maximize().map_err(|e| e.to_string())?;
        Ok(true)
    }
}

#[tauri::command]
async fn win_close(app: AppHandle) -> Result<(), String> {
    main_window(&app)?.close().map_err(|e| e.to_string())
}

#[tauri::command]
async fn win_is_maximized(app: AppHandle) -> Result<bool, String> {
    Ok(main_window(&app)?.is_maximized().unwrap_or(false))
}

#[tauri::command]
async fn win_drag(app: AppHandle) -> Result<(), String> {
    main_window(&app)?.start_dragging().map_err(|e| e.to_string())
}

#[tauri::command]
async fn win_resize(app: AppHandle, edge: String) -> Result<(), String> {
    use tauri_runtime::ResizeDirection as D;
    let dir = match edge.as_str() {
        "n" => D::North,
        "s" => D::South,
        "e" => D::East,
        "w" => D::West,
        "ne" => D::NorthEast,
        "nw" => D::NorthWest,
        "se" => D::SouthEast,
        "sw" => D::SouthWest,
        _ => return Err("bad edge".into()),
    };
    main_window(&app)?.start_resize_dragging(dir).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------- page tools

#[tauri::command]
async fn tab_zoom(app: AppHandle, id: String, factor: f64) -> Result<(), String> {
    let wv = app.get_webview(&tab_label(&id)).ok_or("tab not found")?;
    wv.set_zoom(factor.clamp(0.25, 5.0)).map_err(|e| e.to_string())
}

#[tauri::command]
async fn tab_print(app: AppHandle, id: String) -> Result<(), String> {
    let wv = app.get_webview(&tab_label(&id)).ok_or("tab not found")?;
    wv.print().map_err(|e| e.to_string())
}

#[tauri::command]
async fn tab_devtools(app: AppHandle, id: String) -> Result<(), String> {
    let wv = app.get_webview(&tab_label(&id)).ok_or("tab not found")?;
    wv.open_devtools();
    Ok(())
}

/// Clears cookies, cache and site storage for every website (Odyssey's own settings stay).
#[tauri::command]
async fn clear_site_data(app: AppHandle) -> Result<(), String> {
    for (label, wv) in app.webviews() {
        if label.starts_with("tab-") {
            return wv.clear_all_browsing_data().map_err(|e| e.to_string());
        }
    }
    // No tab open: make a throwaway one so the shared profile can be cleared.
    Err("Open any website tab first, then clear again.".into())
}

// ---------------------------------------------------------------- shields

fn persist_shields(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let sh = app.state::<Shields>();
    let mut cfg = state.config.lock().unwrap().clone();
    cfg.shields_enabled = sh.enabled.load(Ordering::Relaxed);
    cfg.cookie_banners = sh.cookie_banners.load(Ordering::Relaxed);
    let mut allow: Vec<String> = sh.allow.read().unwrap().iter().cloned().collect();
    allow.sort();
    cfg.shield_allow = allow;
    save_config(app, &cfg)?;
    *state.config.lock().unwrap() = cfg;
    Ok(())
}

#[tauri::command]
async fn shields_info(app: AppHandle, url: Option<String>, id: Option<String>) -> Result<Value, String> {
    let sh = app.state::<Shields>();
    let host = url.as_deref().map(shields::host_of).unwrap_or_default();
    let blocked = id
        .and_then(|i| sh.per_tab.lock().unwrap().get(&i).copied())
        .unwrap_or(0);
    Ok(json!({
        "enabled": sh.enabled.load(Ordering::Relaxed),
        "host": host,
        "site_on": !sh.allow.read().unwrap().contains(&host),
        "blocked": blocked,
        "total": sh.total_blocked.load(Ordering::Relaxed),
        "status": sh.status.lock().unwrap().clone(),
        "rules": sh.rules.load(Ordering::Relaxed),
        "ready": sh.engine().is_some(),
        "cookie_banners": sh.cookie_banners.load(Ordering::Relaxed),
        "allowed_sites": sh.allow.read().unwrap().iter().cloned().collect::<Vec<_>>(),
    }))
}

#[tauri::command]
async fn shields_set_enabled(app: AppHandle, on: bool) -> Result<(), String> {
    app.state::<Shields>().enabled.store(on, Ordering::Relaxed);
    persist_shields(&app)
}

#[tauri::command]
async fn shields_set_site(app: AppHandle, host: String, on: bool) -> Result<(), String> {
    {
        let sh = app.state::<Shields>();
        let mut allow = sh.allow.write().unwrap();
        let h = host.trim().trim_start_matches("www.").to_lowercase();
        if on {
            allow.remove(&h);
        } else if !h.is_empty() {
            allow.insert(h);
        }
    }
    persist_shields(&app)
}

#[tauri::command]
async fn shields_set_cookie_banners(app: AppHandle, on: bool) -> Result<(), String> {
    app.state::<Shields>().cookie_banners.store(on, Ordering::Relaxed);
    persist_shields(&app)?;
    shields::rebuild(app.clone(), false);
    Ok(())
}

#[tauri::command]
async fn shields_update(app: AppHandle) -> Result<(), String> {
    shields::rebuild(app.clone(), true);
    Ok(())
}

// ---------------------------------------------------------------- web results

fn clean_ddg_link(href: &str) -> String {
    if let Some(pos) = href.find("uddg=") {
        let enc = href[pos + 5..].split('&').next().unwrap_or("");
        return percent_encoding::percent_decode_str(enc).decode_utf8_lossy().to_string();
    }
    if href.starts_with("//") {
        return format!("https:{href}");
    }
    href.to_string()
}

fn strip_tags(s: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for ch in s.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out.replace("&amp;", "&").replace("&quot;", "\"").replace("&#x27;", "'").replace("&#39;", "'")
        .replace("&lt;", "<").replace("&gt;", ">").replace("&nbsp;", " ").trim().to_string()
}

/// Minimal DuckDuckGo HTML results parser (fallback when Odysseus search isn't reachable).
async fn ddg_results(client: &reqwest::Client, query: &str) -> Result<Vec<Value>, String> {
    let html = client
        .get("https://html.duckduckgo.com/html/")
        .query(&[("q", query)])
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130 Safari/537.36")
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for block in html.split("class=\"result__a\"").skip(1).take(12) {
        let href = block.split("href=\"").nth(1).and_then(|r| r.split('"').next()).unwrap_or("");
        let title = block.split('>').nth(1).and_then(|r| r.split("</a").next()).unwrap_or("");
        let snippet = block
            .split("result__snippet")
            .nth(1)
            .and_then(|r| r.split_once('>'))
            .and_then(|(_, r)| r.split("</a").next())
            .unwrap_or("");
        let url = clean_ddg_link(&href.replace("&amp;", "&"));
        if url.starts_with("http") && !url.contains("duckduckgo.com/y.js") {
            out.push(json!({ "title": strip_tags(title), "url": url, "snippet": strip_tags(snippet) }));
        }
    }
    Ok(out)
}

#[tauri::command]
async fn web_search(state: State<'_, AppState>, query: String) -> Result<Vec<Value>, String> {
    let cfg = state.config.lock().unwrap().clone();
    if !cfg.token.is_empty() {
        let form = reqwest::multipart::Form::new()
            .text("query", query.clone())
            .text("provider", "duckduckgo")
            .text("count", "10");
        let resp = state
            .http
            .post(format!("{}/api/search/query", cfg.base_url))
            .bearer_auth(&cfg.token)
            .multipart(form)
            .timeout(Duration::from_secs(20))
            .send()
            .await;
        if let Ok(resp) = resp {
            if resp.status().is_success() {
                if let Ok(body) = resp.json::<Value>().await {
                    let list: Vec<Value> = body["results"]
                        .as_array()
                        .cloned()
                        .unwrap_or_default()
                        .into_iter()
                        .filter_map(|r| {
                            let url = r["url"].as_str().or(r["href"].as_str())?.to_string();
                            Some(json!({
                                "title": r["title"].as_str().unwrap_or(&url),
                                "url": url,
                                "snippet": r["snippet"].as_str().or(r["body"].as_str()).unwrap_or(""),
                            }))
                        })
                        .collect();
                    if !list.is_empty() {
                        return Ok(list);
                    }
                }
            }
        }
    }
    ddg_results(&state.http, &query).await
}

fn main() {
    tauri::Builder::default()
        .plugin(single_instance_init(|app, _args, _cwd| {
            let _ = app.emit_to("ui", "single-instance", ());
        }))
        .manage(Shields::default())
        .manage(AppState {
            http: reqwest::Client::builder()
                .connect_timeout(Duration::from_secs(8))
                .build()
                .expect("http client"),
            ..Default::default()
        })
        .setup(|app| {
            let handle = app.handle().clone();
            let cfg = load_config(&handle);
            {
                let sh = app.state::<Shields>();
                sh.enabled.store(cfg.shields_enabled, Ordering::Relaxed);
                sh.cookie_banners.store(cfg.cookie_banners, Ordering::Relaxed);
                *sh.allow.write().unwrap() = cfg.shield_allow.iter().cloned().collect();
            }
            *app.state::<AppState>().config.lock().unwrap() = cfg;
            shields::rebuild(handle.clone(), false);
            let window = tauri::window::WindowBuilder::new(app, "main")
                .title("Odyssey")
                .inner_size(1180.0, 720.0)
                .center()
                .min_inner_size(720.0, 480.0)
                .maximized(true)
                .decorations(false)
                .shadow(true)
                .build()?;
            let h2 = handle.clone();
            let w2 = window.clone();
            window.on_window_event(move |e| {
                if let tauri::WindowEvent::Resized(_) = e {
                    let max = w2.is_maximized().unwrap_or(false);
                    let _ = h2.emit_to(UI_LABEL, "win-state", json!({ "maximized": max }));
                }
            });
            let size = window.inner_size()?.to_logical::<f64>(window.scale_factor()?);
            window.add_child(
                WebviewBuilder::new(UI_LABEL, WebviewUrl::App("index.html".into())).auto_resize(),
                LogicalPosition::new(0.0, 0.0),
                LogicalSize::new(size.width, size.height),
            )?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            focus_ui,
            tab_mute,
            tab_pip,
            tab_reader,
            show_in_folder,
            open_download,
            tab_find,
            tab_find_clear,
            set_layout,
            tab_create,
            tab_activate,
            tab_navigate,
            tab_action,
            tab_close,
            tab_extract,
            vault_list,
            vault_list_for_origin,
            vault_add,
            vault_update,
            vault_delete,
            vault_reveal,
            vault_autofill,
            config_get,
            config_set,
            pair_odysseus,
            disconnect,
            ai_test,
            ai_ask,
            ai_reset,
            win_minimize,
            win_toggle_maximize,
            win_close,
            win_is_maximized,
            win_drag,
            win_resize,
            tab_zoom,
            tab_print,
            tab_devtools,
            clear_site_data,
            shields_info,
            shields_set_enabled,
            shields_set_site,
            shields_set_cookie_banners,
            shields_update,
            web_search
        ])
        .run(tauri::generate_context!())
        .expect("error while running Odyssey");
}
