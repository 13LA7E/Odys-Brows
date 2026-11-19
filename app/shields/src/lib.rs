//! Built-in ad and tracker blocking (Shields), powered by `adblock-rust`.
//!
//! Handles filter list caching and updating, network request interception,
//! per-site allowlist exceptions, and cosmetic filtering (P5-01 / EDGE_CASES §8).

use adblock::Engine;
use adblock::lists::{FilterSet, ParseOptions};
use adblock::request::Request;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, SystemTime};

pub const DEFAULT_FILTER_RULES: &str = r#"
! Built-in baseline rules for immediate startup protection
||google-analytics.com^$third-party
||googletagmanager.com^$third-party
||doubleclick.net^$third-party
||adservice.google.com^$third-party
||adnxs.com^$third-party
||advertising.com^$third-party
||criteo.com^$third-party
||facebook.net^$third-party
||scorecardresearch.com^$third-party
||quantserve.com^$third-party
||adroll.com^$third-party
||outbrain.com^$third-party
||taboola.com^$third-party
||moatads.com^$third-party
||popads.net^$third-party
||smartadserver.com^$third-party
||rubiconproject.com^$third-party
||pubmatic.com^$third-party
||amazon-adsystem.com^$third-party
||adcolony.com^$third-party
||unityads.unity3d.com^$third-party
||applovin.com^$third-party
||vungle.com^$third-party
||chartboost.com^$third-party
||inmobi.com^$third-party
||ironsrc.com^$third-party
"#;

const LIST_SOURCES: &[(&str, &str)] = &[
    ("easylist", "https://easylist.to/easylist/easylist.txt"),
    (
        "easyprivacy",
        "https://easylist.to/easylist/easyprivacy.txt",
    ),
    (
        "ubo-filters",
        "https://ublockorigin.github.io/uAssets/filters/filters.txt",
    ),
    (
        "peter-lowe",
        "https://pgl.yoyo.org/adservers/serverlist.php?hostformat=adblockplus&showintro=0&mimetype=plaintext",
    ),
];

const REFRESH_AFTER: Duration = Duration::from_secs(4 * 24 * 3600);

pub fn extract_host(url: &str) -> String {
    let rest = url.split("://").nth(1).unwrap_or(url);
    let host_part = rest.split(['/', '?', '#']).next().unwrap_or("");
    let host_no_user = host_part.rsplit('@').next().unwrap_or("");
    let host_no_port = host_no_user.split(':').next().unwrap_or("");
    host_no_port.trim_start_matches("www.").to_lowercase()
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ShieldsData {
    pub enabled: bool,
    pub allowlist: HashSet<String>,
}

pub struct Shields {
    engine: RwLock<Option<Arc<Engine>>>,
    enabled: AtomicBool,
    allowlist: RwLock<HashSet<String>>,
    total_blocked: AtomicU64,
    per_tab: Mutex<HashMap<u64, u64>>,
    cache_dir: PathBuf,
}

impl Default for Shields {
    fn default() -> Self {
        Self::new(std::env::temp_dir().join("constellations-shields"))
    }
}

impl Shields {
    pub fn new(cache_dir: PathBuf) -> Self {
        let mut filter_set = FilterSet::new(false);
        filter_set.add_filter_list(DEFAULT_FILTER_RULES.to_string(), ParseOptions::default());
        let engine = Engine::new_with_filter_set(filter_set);

        let shields = Self {
            engine: RwLock::new(Some(Arc::new(engine))),
            enabled: AtomicBool::new(true),
            allowlist: RwLock::new(HashSet::new()),
            total_blocked: AtomicU64::new(0),
            per_tab: Mutex::new(HashMap::new()),
            cache_dir,
        };
        shields.load_allowlist();
        shields
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    pub fn set_enabled(&self, val: bool) {
        self.enabled.store(val, Ordering::Relaxed);
    }

    pub fn is_enabled_for_url(&self, url: &str) -> bool {
        if !self.is_enabled() {
            return false;
        }
        let host = extract_host(url);
        if host.is_empty() {
            return true;
        }
        !self.allowlist.read().unwrap().contains(&host)
    }

    pub fn toggle_exception(&self, host: &str) -> bool {
        let clean = host.trim_start_matches("www.").to_lowercase();
        let active = {
            let mut list = self.allowlist.write().unwrap();
            if list.contains(&clean) {
                list.remove(&clean);
                true // Shields now ON for this host
            } else {
                list.insert(clean);
                false // Shields now OFF for this host
            }
        };
        self.save_allowlist();
        active
    }

    pub fn is_host_exempt(&self, host: &str) -> bool {
        let clean = host.trim_start_matches("www.").to_lowercase();
        self.allowlist.read().unwrap().contains(&clean)
    }

    pub fn check_network_request(
        &self,
        url: &str,
        source_url: &str,
        resource_type: &str,
        method: &str,
        tab_id: u64,
    ) -> bool {
        if !self.is_enabled_for_url(source_url) {
            return false;
        }

        let engine = match self.engine.read().unwrap().clone() {
            Some(e) => e,
            None => return false,
        };

        let req = match Request::new(url, source_url, resource_type, method) {
            Ok(r) => r,
            Err(_) => return false,
        };

        let blocker = engine.check_network_request(&req);
        if blocker.should_block() {
            self.total_blocked.fetch_add(1, Ordering::Relaxed);
            let mut per_tab = self.per_tab.lock().unwrap();
            let count = per_tab.entry(tab_id).or_insert(0);
            *count += 1;
            true
        } else {
            false
        }
    }

    pub fn blocked_count_for_tab(&self, tab_id: u64) -> u64 {
        self.per_tab
            .lock()
            .unwrap()
            .get(&tab_id)
            .copied()
            .unwrap_or(0)
    }

    pub fn reset_tab(&self, tab_id: u64) {
        self.per_tab.lock().unwrap().remove(&tab_id);
    }

    pub fn total_blocked(&self) -> u64 {
        self.total_blocked.load(Ordering::Relaxed)
    }

    pub fn cosmetic_css(&self, page_url: &str) -> String {
        if !self.is_enabled_for_url(page_url) {
            return String::new();
        }
        let engine = match self.engine.read().unwrap().clone() {
            Some(e) => e,
            None => return String::new(),
        };
        let resources = engine.url_cosmetic_resources(page_url);
        let selectors: Vec<String> = resources.hide_selectors.into_iter().collect();
        if selectors.is_empty() {
            return String::new();
        }
        selectors
            .chunks(100)
            .map(|chunk| format!("{}{{display:none!important}}", chunk.join(",")))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn allowlist_file(&self) -> PathBuf {
        self.cache_dir.join("shields_allowlist.json")
    }

    fn load_allowlist(&self) {
        let path = self.allowlist_file();
        let Ok(data) = std::fs::read_to_string(&path) else {
            return;
        };
        let Ok(parsed) = serde_json::from_str::<ShieldsData>(&data) else {
            return;
        };
        self.enabled.store(parsed.enabled, Ordering::Relaxed);
        *self.allowlist.write().unwrap() = parsed.allowlist;
    }

    fn save_allowlist(&self) {
        let _ = std::fs::create_dir_all(&self.cache_dir);
        let data = ShieldsData {
            enabled: self.enabled.load(Ordering::Relaxed),
            allowlist: self.allowlist.read().unwrap().clone(),
        };
        if let Ok(json) = serde_json::to_string_pretty(&data) {
            let tmp = self.cache_dir.join("shields_allowlist.tmp");
            if std::fs::write(&tmp, json).is_ok() {
                let _ = std::fs::rename(tmp, self.allowlist_file());
            }
        }
    }

    /// Asynchronously fetch updated filter lists and re-initialize engine.
    pub fn update_filters_background(self: Arc<Self>) {
        std::thread::spawn(move || {
            let Ok(rt) = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            else {
                return;
            };
            rt.block_on(async move {
                let _ = std::fs::create_dir_all(&self.cache_dir);
                let client = match reqwest::Client::builder()
                    .timeout(Duration::from_secs(30))
                    .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) Constellations/0.1")
                    .build()
                {
                    Ok(c) => c,
                    Err(_) => return,
                };

                let mut filter_texts = Vec::new();
                for (name, url) in LIST_SOURCES {
                    let cache_file = self.cache_dir.join(format!("{name}.txt"));
                    let is_fresh = std::fs::metadata(&cache_file)
                        .and_then(|m| m.modified())
                        .ok()
                        .and_then(|t| SystemTime::now().duration_since(t).ok())
                        .map(|age| age < REFRESH_AFTER)
                        .unwrap_or(false);

                    if is_fresh && cache_file.exists() {
                        let text = std::fs::read_to_string(&cache_file).unwrap_or_default();
                        if !text.is_empty() {
                            filter_texts.push(text);
                            continue;
                        }
                    }

                    let maybe_resp = match client.get(*url).send().await {
                        Ok(r) if r.status().is_success() => Some(r),
                        _ => None,
                    };
                    if let Some(resp) = maybe_resp {
                        let text = resp.text().await.unwrap_or_default();
                        if text.len() > 500 {
                            let _ = std::fs::write(&cache_file, &text);
                            filter_texts.push(text);
                            continue;
                        }
                    }

                    // Fallback to existing file if offline
                    if let Ok(s) = std::fs::read_to_string(&cache_file) {
                        filter_texts.push(s);
                    }
                }

                if !filter_texts.is_empty() {
                    let mut set = FilterSet::new(false);
                    set.add_filter_list(DEFAULT_FILTER_RULES.to_string(), ParseOptions::default());
                    for t in filter_texts {
                        set.add_filter_list(t, ParseOptions::default());
                    }
                    let new_engine = Engine::new_with_filter_set(set);
                    *self.engine.write().unwrap() = Some(Arc::new(new_engine));
                }
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_host_correctly() {
        assert_eq!(
            extract_host("https://www.example.com/path?query"),
            "example.com"
        );
        assert_eq!(
            extract_host("http://ads.doubleclick.net:8080/foo"),
            "ads.doubleclick.net"
        );
        assert_eq!(extract_host("example.org"), "example.org");
    }

    #[test]
    fn baseline_rules_block_known_trackers() {
        let shields = Shields::default();
        let blocked = shields.check_network_request(
            "https://www.google-analytics.com/analytics.js",
            "https://news.ycombinator.com/",
            "script",
            "GET",
            1,
        );
        assert!(
            blocked,
            "Google Analytics should be blocked as a third-party tracker"
        );
        assert_eq!(shields.blocked_count_for_tab(1), 1);
        assert_eq!(shields.total_blocked(), 1);

        let allowed = shields.check_network_request(
            "https://news.ycombinator.com/hn.js",
            "https://news.ycombinator.com/",
            "script",
            "GET",
            1,
        );
        assert!(!allowed, "First-party script should not be blocked");
        assert_eq!(shields.blocked_count_for_tab(1), 1);
    }

    #[test]
    fn site_exception_toggles_and_persists() {
        let tmp = std::env::temp_dir().join(format!("shields-test-{}", std::process::id()));
        let shields = Shields::new(tmp.clone());

        assert!(shields.is_enabled_for_url("https://example.com/"));
        assert!(!shields.is_host_exempt("example.com"));

        // Toggle off
        let active = shields.toggle_exception("example.com");
        assert!(!active);
        assert!(shields.is_host_exempt("example.com"));
        assert!(!shields.is_enabled_for_url("https://example.com/page"));

        // Requests from this site are now allowed even if tracking
        let blocked = shields.check_network_request(
            "https://www.google-analytics.com/analytics.js",
            "https://example.com/",
            "script",
            "GET",
            1,
        );
        assert!(!blocked);

        // Toggle back on
        let active2 = shields.toggle_exception("example.com");
        assert!(active2);
        assert!(!shields.is_host_exempt("example.com"));
        assert!(shields.is_enabled_for_url("https://example.com/page"));

        let _ = std::fs::remove_dir_all(tmp);
    }

    #[test]
    fn tab_counts_reset_cleanly() {
        let shields = Shields::default();
        shields.check_network_request(
            "https://doubleclick.net/ad.js",
            "https://example.com/",
            "script",
            "GET",
            42,
        );
        assert_eq!(shields.blocked_count_for_tab(42), 1);
        shields.reset_tab(42);
        assert_eq!(shields.blocked_count_for_tab(42), 0);
    }
}
