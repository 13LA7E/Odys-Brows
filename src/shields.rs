//! Shields: ad and tracker blocking built on Brave's adblock-rust engine.
//!
//! * Filter lists (EasyList, EasyPrivacy, uBlock Origin lists, Peter Lowe) are
//!   downloaded on first run, cached, and refreshed every few days.
//! * Network requests from every tab are checked inside WebView2 before they
//!   leave the machine; blocked ones get an empty 403 response.
//! * Cosmetic filters hide leftover ad boxes, and scriptlets (when Brave's
//!   resource bundle is available) are injected before page scripts run.

use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex, RwLock,
    },
    time::{Duration, SystemTime},
};

use adblock::{
    lists::{FilterSet, ParseOptions},
    resources::Resource,
    Engine,
};
use serde_json::json;
use tauri::{AppHandle, Emitter, Manager};

const LISTS: &[(&str, &str)] = &[
    ("easylist", "https://easylist.to/easylist/easylist.txt"),
    ("easyprivacy", "https://easylist.to/easylist/easyprivacy.txt"),
    ("ubo-filters", "https://ublockorigin.github.io/uAssets/filters/filters.txt"),
    ("ubo-privacy", "https://ublockorigin.github.io/uAssets/filters/privacy.txt"),
    ("ubo-unbreak", "https://ublockorigin.github.io/uAssets/filters/unbreak.txt"),
    ("ubo-quick-fixes", "https://ublockorigin.github.io/uAssets/filters/quick-fixes.txt"),
    (
        "peter-lowe",
        "https://pgl.yoyo.org/adservers/serverlist.php?hostformat=adblockplus&showintro=0&mimetype=plaintext",
    ),
];
const COOKIE_LIST: (&str, &str) = ("cookie-banners", "https://secure.fanboy.co.nz/fanboy-cookiemonster.txt");
const RESOURCES_URL: &str = "https://raw.githubusercontent.com/brave/adblock-resources/master/dist/resources.json";
const REFRESH_AFTER: Duration = Duration::from_secs(4 * 24 * 3600);

pub struct Shields {
    pub engine: RwLock<Option<Arc<Engine>>>,
    pub enabled: AtomicBool,
    pub cookie_banners: AtomicBool,
    pub allow: RwLock<HashSet<String>>,
    pub total_blocked: AtomicU64,
    pub per_tab: Mutex<HashMap<String, u64>>,
    pub status: Mutex<String>,
    pub rules: AtomicU64,
    pub building: AtomicBool,
}

impl Default for Shields {
    fn default() -> Self {
        Shields {
            engine: RwLock::new(None),
            enabled: AtomicBool::new(true),
            cookie_banners: AtomicBool::new(false),
            allow: RwLock::new(HashSet::new()),
            total_blocked: AtomicU64::new(0),
            per_tab: Mutex::new(HashMap::new()),
            status: Mutex::new("Starting".into()),
            rules: AtomicU64::new(0),
            building: AtomicBool::new(false),
        }
    }
}

pub fn host_of(url: &str) -> String {
    let rest = url.split("://").nth(1).unwrap_or("");
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    let host = host.rsplit('@').next().unwrap_or("");
    let host = host.split(':').next().unwrap_or("");
    host.trim_start_matches("www.").to_lowercase()
}

impl Shields {
    /// Should requests made by a page on `page_url` be filtered?
    pub fn active_for(&self, page_url: &str) -> bool {
        self.enabled.load(Ordering::Relaxed) && !self.allow.read().unwrap().contains(&host_of(page_url))
    }

    pub fn engine(&self) -> Option<Arc<Engine>> {
        self.engine.read().unwrap().clone()
    }

    pub fn count_block(&self, app: &AppHandle, tab: &str) {
        self.total_blocked.fetch_add(1, Ordering::Relaxed);
        let n = {
            let mut m = self.per_tab.lock().unwrap();
            let c = m.entry(tab.to_string()).or_insert(0);
            *c += 1;
            *c
        };
        let _ = app.emit_to("ui", "shield-count", json!({ "id": tab, "count": n }));
    }

    pub fn reset_tab(&self, app: &AppHandle, tab: &str) {
        self.per_tab.lock().unwrap().insert(tab.to_string(), 0);
        let _ = app.emit_to("ui", "shield-count", json!({ "id": tab, "count": 0 }));
    }
}

fn lists_dir(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_data_dir().ok().map(|d| d.join("shields"))
}

fn is_fresh(path: &PathBuf) -> bool {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| SystemTime::now().duration_since(t).ok())
        .map(|age| age < REFRESH_AFTER)
        .unwrap_or(false)
}

async fn fetch_cached(client: &reqwest::Client, dir: &PathBuf, name: &str, url: &str, force: bool) -> Option<String> {
    let path = dir.join(format!("{name}.txt"));
    if !force && is_fresh(&path) {
        if let Ok(s) = std::fs::read_to_string(&path) {
            return Some(s);
        }
    }
    let fresh = async {
        let r = client.get(url).timeout(Duration::from_secs(40)).send().await.ok()?;
        if !r.status().is_success() {
            return None;
        }
        r.text().await.ok()
    }
    .await;
    match fresh {
        Some(text) if text.len() > 200 => {
            let _ = std::fs::write(&path, &text);
            Some(text)
        }
        // Offline or the server failed: fall back to whatever we cached before.
        _ => std::fs::read_to_string(&path).ok(),
    }
}

/// Download (or load cached) lists and build a new engine in the background.
pub fn rebuild(app: AppHandle, force_download: bool) {
    tauri::async_runtime::spawn(async move {
        let shields = app.state::<Shields>();
        if shields.building.swap(true, Ordering::SeqCst) {
            return;
        }
        *shields.status.lock().unwrap() = "Updating filter lists".into();
        let _ = app.emit_to("ui", "shield-status", ());
        let Some(dir) = lists_dir(&app) else { return };
        let _ = std::fs::create_dir_all(&dir);
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) Odyssey/0.2")
            .build()
            .unwrap_or_default();

        let mut texts = Vec::new();
        let mut wanted: Vec<(&str, &str)> = LISTS.to_vec();
        if shields.cookie_banners.load(Ordering::Relaxed) {
            wanted.push(COOKIE_LIST);
        }
        for (name, url) in wanted {
            if let Some(t) = fetch_cached(&client, &dir, name, url, force_download).await {
                texts.push(t);
            }
        }
        let resources_json = fetch_cached(&client, &dir, "resources", RESOURCES_URL, force_download).await;
        let list_count = texts.len();

        let built = tauri::async_runtime::spawn_blocking(move || {
            let mut set = FilterSet::new(false);
            let mut rules = 0usize;
            for t in texts {
                rules += t.lines().filter(|l| !l.is_empty() && !l.starts_with('!')).count();
                set.add_filter_list(t, ParseOptions::default());
            }
            let mut engine = Engine::new_with_filter_set(set);
            if let Some(js) = resources_json {
                if let Ok(res) = serde_json::from_str::<Vec<Resource>>(&js) {
                    engine.use_resources(res);
                }
            }
            (engine, rules)
        })
        .await;

        match built {
            Ok((engine, rules)) if list_count > 0 => {
                *shields.engine.write().unwrap() = Some(Arc::new(engine));
                shields.rules.store(rules as u64, Ordering::Relaxed);
                *shields.status.lock().unwrap() = format!("Active with {list_count} filter lists");
            }
            _ => {
                *shields.status.lock().unwrap() =
                    "Couldn't download filter lists. Check your connection and click Update lists.".into();
            }
        }
        shields.building.store(false, Ordering::SeqCst);
        let _ = app.emit_to("ui", "shield-status", ());
    });
}

/// CSS that hides the given selectors, in chunks so one bad selector can't void the rest.
pub fn hide_css(selectors: impl IntoIterator<Item = String>) -> String {
    let all: Vec<String> = selectors.into_iter().collect();
    all.chunks(200)
        .map(|c| format!("{}{{display:none!important}}", c.join(",")))
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn inject_css_js(css: &str) -> String {
    format!(
        "(function(){{try{{var s=document.createElement('style');s.setAttribute('data-odyssey','shields');s.textContent={};(document.head||document.documentElement).appendChild(s);}}catch(e){{}}}})();",
        serde_json::to_string(css).unwrap_or_else(|_| "''".into())
    )
}

/// Collects class names and ids present on the page (for generic cosmetic rules).
pub const COLLECT_JS: &str = r#"(function(){
  var c = new Set(), i = new Set();
  var els = document.querySelectorAll('[class],[id]');
  for (var k = 0; k < els.length && k < 6000; k++) {
    var e = els[k];
    if (e.id) i.add(e.id);
    if (e.classList) for (var j = 0; j < e.classList.length; j++) c.add(e.classList[j]);
  }
  return { c: Array.from(c).slice(0, 4000), i: Array.from(i).slice(0, 2000) };
})()"#;

/// Selectors for generic rules matching the classes/ids reported by COLLECT_JS.
pub fn generic_selectors(engine: &Engine, page_url: &str, collected_json: &str) -> Vec<String> {
    let v: serde_json::Value = match serde_json::from_str(collected_json) {
        Ok(v) => v,
        Err(_) => return vec![],
    };
    let strs = |k: &str| -> Vec<String> {
        v.get(k)
            .and_then(|a| a.as_array())
            .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
            .unwrap_or_default()
    };
    let res = engine.url_cosmetic_resources(page_url);
    if res.generichide {
        return vec![];
    }
    engine.hidden_class_id_selectors(strs("c"), strs("i"), &res.exceptions)
}

// ---------------------------------------------------------------- WebView2 hooks (Windows)

#[cfg(windows)]
pub mod win {
    use super::*;
    use std::{cell::RefCell, rc::Rc};
    use webview2_com::{
        take_pwstr, AcceleratorKeyPressedEventHandler, AddScriptToExecuteOnDocumentCreatedCompletedHandler,
        DOMContentLoadedEventHandler,
        ExecuteScriptCompletedHandler, Microsoft::Web::WebView2::Win32::*, NavigationStartingEventHandler,
        WebResourceRequestedEventHandler,
    };
    use windows::core::{Interface, HSTRING, PWSTR};
    use windows::Win32::UI::Input::KeyboardAndMouse::GetKeyState;

    fn down(vk: i32) -> bool {
        unsafe { (GetKeyState(vk) as u16 & 0x8000) != 0 }
    }

    /// Browser shortcuts (Ctrl+T, Ctrl+W, F5, Alt+Left…) while a web page has focus.
    unsafe fn hook_shortcuts(controller: &ICoreWebView2Controller, app: AppHandle) {
        let mut token = 0i64;
        let _ = controller.add_AcceleratorKeyPressed(
            &AcceleratorKeyPressedEventHandler::create(Box::new(move |_sender, args| {
                let Some(args) = args else { return Ok(()) };
                let mut kind = COREWEBVIEW2_KEY_EVENT_KIND_KEY_DOWN;
                args.KeyEventKind(&mut kind)?;
                if kind != COREWEBVIEW2_KEY_EVENT_KIND_KEY_DOWN && kind != COREWEBVIEW2_KEY_EVENT_KIND_SYSTEM_KEY_DOWN {
                    return Ok(());
                }
                let mut vk = 0u32;
                args.VirtualKey(&mut vk)?;
                let (ctrl, shift, alt) = (down(0x11), down(0x10), down(0x12));
                let key = match vk {
                    0x74 => "F5",
                    0x7B => "F12",
                    0x25 if alt => "ArrowLeft",
                    0x27 if alt => "ArrowRight",
                    0x09 if ctrl => "Tab",
                    0xBC if ctrl => ",",
                    0xBB if ctrl => "=",
                    0xBD if ctrl => "-",
                    0x30 if ctrl => "0",
                    v if ctrl && (0x41..=0x5A).contains(&v) => {
                        let c = (v as u8 as char).to_ascii_lowercase();
                        match c {
                            't' | 'w' | 'l' | 'j' | 'r' | 'd' | 'h' | 'p' | 'k' => {
                                return {
                                    let _ = app.emit_to("ui", "accel", json!({ "key": c.to_string(), "ctrl": ctrl, "shift": shift, "alt": alt }));
                                    args.SetHandled(true)
                                };
                            }
                            _ => return Ok(()),
                        }
                    }
                    _ => return Ok(()),
                };
                let _ = app.emit_to("ui", "accel", json!({ "key": key, "ctrl": ctrl, "shift": shift, "alt": alt }));
                args.SetHandled(true)
            })),
            &mut token,
        );
    }

    unsafe fn source_of(core: &ICoreWebView2) -> String {
        let mut p = PWSTR::null();
        if core.Source(&mut p).is_ok() {
            take_pwstr(p)
        } else {
            String::new()
        }
    }

    fn context_name(ctx: COREWEBVIEW2_WEB_RESOURCE_CONTEXT) -> &'static str {
        if ctx == COREWEBVIEW2_WEB_RESOURCE_CONTEXT_DOCUMENT {
            "document"
        } else if ctx == COREWEBVIEW2_WEB_RESOURCE_CONTEXT_STYLESHEET {
            "stylesheet"
        } else if ctx == COREWEBVIEW2_WEB_RESOURCE_CONTEXT_IMAGE {
            "image"
        } else if ctx == COREWEBVIEW2_WEB_RESOURCE_CONTEXT_MEDIA {
            "media"
        } else if ctx == COREWEBVIEW2_WEB_RESOURCE_CONTEXT_FONT {
            "font"
        } else if ctx == COREWEBVIEW2_WEB_RESOURCE_CONTEXT_SCRIPT {
            "script"
        } else if ctx == COREWEBVIEW2_WEB_RESOURCE_CONTEXT_XML_HTTP_REQUEST
            || ctx == COREWEBVIEW2_WEB_RESOURCE_CONTEXT_FETCH
            || ctx == COREWEBVIEW2_WEB_RESOURCE_CONTEXT_EVENT_SOURCE
        {
            "xhr"
        } else if ctx == COREWEBVIEW2_WEB_RESOURCE_CONTEXT_WEBSOCKET {
            "websocket"
        } else if ctx == COREWEBVIEW2_WEB_RESOURCE_CONTEXT_PING {
            "ping"
        } else {
            "other"
        }
    }

    /// Run the cosmetic pass (specific + generic hiding) on whatever page `core` shows.
    pub unsafe fn cosmetic_pass(app: &AppHandle, core: &ICoreWebView2, specific: bool) {
        let shields = app.state::<Shields>();
        let url = source_of(core);
        if !url.starts_with("http") || !shields.active_for(&url) {
            return;
        }
        let Some(engine) = shields.engine() else { return };
        if specific {
            let res = engine.url_cosmetic_resources(&url);
            if !res.hide_selectors.is_empty() {
                let js = inject_css_js(&hide_css(res.hide_selectors.into_iter()));
                let _ = core.ExecuteScript(&HSTRING::from(js), None);
            }
        }
        let core2 = core.clone();
        let handler = ExecuteScriptCompletedHandler::create(Box::new(move |_res, json| {
            let sels = generic_selectors(&engine, &url, &json);
            if !sels.is_empty() {
                let js = inject_css_js(&hide_css(sels));
                let _ = core2.ExecuteScript(&HSTRING::from(js), None);
            }
            Ok(())
        }));
        let _ = core.ExecuteScript(&HSTRING::from(COLLECT_JS), &handler);
    }

    /// Attach Shields to one tab's WebView2.
    pub fn attach(webview: &tauri::Webview, app: AppHandle, tab: String) {
        let _ = webview.with_webview(move |pw| unsafe {
            let controller = pw.controller();
            hook_shortcuts(&controller, app.clone());
            let Ok(core) = controller.CoreWebView2() else { return };
            let env = pw.environment();
            let mut token = 0i64;

            // 1. Network blocking.
            let _ = core.AddWebResourceRequestedFilter(&HSTRING::from("*"), COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL);
            let (app1, tab1, env1) = (app.clone(), tab.clone(), env.clone());
            let _ = core.add_WebResourceRequested(
                &WebResourceRequestedEventHandler::create(Box::new(move |sender, args| {
                    let Some(args) = args else { return Ok(()) };
                    let shields = app1.state::<Shields>();
                    if !shields.enabled.load(Ordering::Relaxed) {
                        return Ok(());
                    }
                    let Some(engine) = shields.engine() else { return Ok(()) };
                    let req = args.Request()?;
                    let mut p = PWSTR::null();
                    req.Uri(&mut p)?;
                    let uri = take_pwstr(p);
                    if !(uri.starts_with("http://") || uri.starts_with("https://")) {
                        return Ok(());
                    }
                    let target_host = host_of(&uri);
                    if target_host.ends_with(".localhost") || target_host == "localhost" || target_host == "127.0.0.1" {
                        return Ok(());
                    }
                    let page = sender.as_ref().map(|s| source_of(s)).unwrap_or_default();
                    if !page.is_empty() && !shields.active_for(&page) {
                        return Ok(());
                    }
                    let mut ctx = COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL;
                    args.ResourceContext(&mut ctx)?;
                    let mut kind = context_name(ctx);
                    if kind == "document" {
                        // Never block the page the user navigated to; only third-party frames.
                        if page.is_empty() || host_of(&page) == target_host || uri == page {
                            return Ok(());
                        }
                        kind = "sub_frame";
                    }
                    let mut m = PWSTR::null();
                    let method = if req.Method(&mut m).is_ok() { take_pwstr(m) } else { "GET".into() };
                    let source = if page.is_empty() { uri.clone() } else { page };
                    let Ok(r) = adblock::request::Request::new(&uri, &source, kind, &method.to_lowercase()) else {
                        return Ok(());
                    };
                    let verdict = engine.check_network_request(&r);
                    if verdict.should_block() {
                        let resp = env1.CreateWebResourceResponse(None, 403, &HSTRING::from("Blocked"), &HSTRING::from(""))?;
                        args.SetResponse(&resp)?;
                        shields.count_block(&app1, &tab1);
                    }
                    Ok(())
                })),
                &mut token,
            );

            // 2. Per-navigation: reset the counter and pre-inject scriptlets.
            let script_id: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
            let (app2, tab2, sid) = (app.clone(), tab.clone(), script_id.clone());
            let _ = core.add_NavigationStarting(
                &NavigationStartingEventHandler::create(Box::new(move |sender, args| {
                    let (Some(core), Some(args)) = (sender, args) else { return Ok(()) };
                    let shields = app2.state::<Shields>();
                    shields.reset_tab(&app2, &tab2);
                    if let Some(old) = sid.borrow_mut().take() {
                        let _ = core.RemoveScriptToExecuteOnDocumentCreated(&HSTRING::from(old));
                    }
                    let mut p = PWSTR::null();
                    args.Uri(&mut p)?;
                    let uri = take_pwstr(p);
                    if !uri.starts_with("http") || !shields.active_for(&uri) {
                        return Ok(());
                    }
                    let Some(engine) = shields.engine() else { return Ok(()) };
                    let script = engine.url_cosmetic_resources(&uri).injected_script;
                    if script.trim().is_empty() {
                        return Ok(());
                    }
                    let host = host_of(&uri);
                    let guarded = format!(
                        "(function(){{try{{var h=location.hostname.replace(/^www\\./,'');if(h!=={host:?}&&!h.endsWith('.'+{host:?}))return;}}catch(e){{return;}}\n{script}\n}})();"
                    );
                    let sid2 = sid.clone();
                    let _ = core.AddScriptToExecuteOnDocumentCreated(
                        &HSTRING::from(guarded),
                        &AddScriptToExecuteOnDocumentCreatedCompletedHandler::create(Box::new(move |_res, id| {
                            *sid2.borrow_mut() = Some(id);
                            Ok(())
                        })),
                    );
                    Ok(())
                })),
                &mut token,
            );

            // 3. Cosmetic filtering once the DOM exists, plus a later pass for late-loading ads.
            if let Ok(core2) = core.cast::<ICoreWebView2_2>() {
                let (app3, tab3) = (app.clone(), tab.clone());
                let _ = core2.add_DOMContentLoaded(
                    &DOMContentLoadedEventHandler::create(Box::new(move |sender, _args| {
                        let Some(core) = sender else { return Ok(()) };
                        cosmetic_pass(&app3, &core, true);
                        let (app4, label) = (app3.clone(), format!("tab-{tab3}"));
                        std::thread::spawn(move || {
                            std::thread::sleep(Duration::from_millis(2500));
                            if let Some(wv) = app4.get_webview(&label) {
                                let app5 = app4.clone();
                                let _ = wv.with_webview(move |pw| unsafe {
                                    if let Ok(core) = pw.controller().CoreWebView2() {
                                        cosmetic_pass(&app5, &core, false);
                                    }
                                });
                            }
                        });
                        Ok(())
                    })),
                    &mut token,
                );
            }
        });
    }
}

#[cfg(not(windows))]
pub mod win {
    use super::*;
    pub fn attach(_webview: &tauri::Webview, _app: AppHandle, _tab: String) {}
}
