//! The one schema for every user setting (Stage F, "STORE").
//!
//! Every option lives here once: its key, its allowed values and its default. Loading and saving
//! both go through [`normalize`], so whatever is in `settings.json` (an old version, a hand edit,
//! a half-written file) becomes a complete, valid settings object: unknown keys are dropped and an
//! invalid or missing value falls back to its default. It never fails and never panics.
//!
//! Adding an option = add one `Field` below, add its default to `DEFAULT_SETTINGS` in
//! `ui/settings.js` (a test checks the two agree), and use `state.settings.<key>` in the UI.

use serde_json::{Map, Value, json};

/// What values a setting may hold. `FloatRange` and `Text` are used by the later Stage F options
/// (sliders, names); they are tested here already.
#[allow(dead_code)]
pub enum Kind {
    /// One of these strings.
    Choice(&'static [&'static str]),
    Bool,
    /// One of these whole numbers.
    IntChoice(&'static [i64]),
    /// A whole number in this range (inclusive).
    IntRange(i64, i64),
    /// A number in this range (inclusive).
    FloatRange(f64, f64),
    /// Text up to this many characters.
    Text(usize),
    /// Anything else: returns the cleaned value, or `None` to use the default.
    Custom(fn(&Value) -> Option<Value>),
}

pub struct Field {
    pub key: &'static str,
    pub kind: Kind,
    pub default: fn() -> Value,
}

/// Search engines, by the name shown in Settings (and used as the value).
pub const SEARCH_ENGINES: &[&str] = &[
    "DuckDuckGo",
    "Google",
    "Brave",
    "Bing",
    "Startpage",
    "Ecosia",
    "Custom",
];

/// A custom engine: `{ name, url }`. The URL must be http(s) and contain `%s` or `{query}` where
/// the search words go; otherwise the whole thing is reset (never run anything from settings).
fn clean_custom_engine(v: &Value) -> Option<Value> {
    let o = v.as_object()?;
    let name: String = o
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("")
        .chars()
        .take(40)
        .collect();
    let url = o.get("url").and_then(Value::as_str).unwrap_or("").trim();
    if url.is_empty() {
        return Some(json!({ "name": name.trim(), "url": "" }));
    }
    let scheme_ok = url.starts_with("https://") || url.starts_with("http://");
    let placeholder = url.contains("%s") || url.contains("{query}");
    if url.len() > 400 || !scheme_ok || !placeholder || url.chars().any(char::is_control) {
        return None;
    }
    Some(json!({ "name": name.trim(), "url": url }))
}

/// Colours of a theme, in the order the editor shows them.
pub const THEME_COLOURS: &[&str] = &["bg", "surface", "panel", "text", "accent", "star"];

fn is_hex6(s: &str) -> bool {
    s.len() == 7 && s.starts_with('#') && s[1..].chars().all(|c| c.is_ascii_hexdigit())
}

/// Your own themes: `{ id: { name, bg, surface, panel, text, accent, star } }` with `#rrggbb`
/// colours. At most 12; an entry with a bad id, name or colour is dropped (never trust a pasted file).
fn clean_custom_themes(v: &Value) -> Option<Value> {
    let o = v.as_object()?;
    let mut out = Map::new();
    for (id, t) in o {
        if out.len() >= 12 {
            break;
        }
        let id_ok = !id.is_empty()
            && id.len() <= 24
            && id
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
        let Some(t) = t.as_object().filter(|_| id_ok) else {
            continue;
        };
        let name: String = t
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("")
            .chars()
            .filter(|c| !c.is_control())
            .take(30)
            .collect();
        let name = name.trim().to_string();
        if name.is_empty() {
            continue;
        }
        let mut clean = Map::new();
        clean.insert("name".into(), json!(name));
        let mut ok = true;
        for k in THEME_COLOURS {
            match t.get(*k).and_then(Value::as_str) {
                Some(c) if is_hex6(c) => {
                    clean.insert((*k).into(), json!(c.to_ascii_lowercase()));
                }
                _ => ok = false,
            }
        }
        if ok {
            out.insert(id.clone(), Value::Object(clean));
        }
    }
    Some(Value::Object(out))
}

/// Ids of the extensions pinned to the notch: at most 24 short plain strings.
fn clean_pinned(v: &Value) -> Option<Value> {
    let list = v.as_array()?;
    let mut out: Vec<Value> = Vec::new();
    for item in list {
        let Some(id) = item.as_str() else { continue };
        if id.is_empty() || id.chars().count() > 64 || id.chars().any(char::is_control) {
            continue;
        }
        if out.len() < 24 && !out.iter().any(|x| x == id) {
            out.push(json!(id));
        }
    }
    Some(Value::Array(out))
}

fn fields() -> Vec<Field> {
    vec![
        // --- Sky (AUDIT-36 adds more)
        Field {
            key: "stars",
            kind: Kind::Choice(&["Off", "Still", "Drifting"]),
            default: || json!("Still"),
        },
        Field {
            key: "shootingStars",
            kind: Kind::Bool,
            default: || json!(false),
        },
        Field {
            key: "starsFollowCursor",
            kind: Kind::Bool,
            default: || json!(true),
        },
        // --- Updates
        Field {
            key: "autoUpdateCheck",
            kind: Kind::Bool,
            default: || json!(true),
        },
        Field {
            key: "updateUrl",
            kind: Kind::Text(300),
            default: || json!(""),
        },
        // --- Assistant
        Field {
            key: "aiTools",
            kind: Kind::Bool,
            default: || json!(true),
        },
        Field {
            key: "aiControl",
            kind: Kind::Bool,
            default: || json!(true),
        },
        // --- Extensions pinned to the notch
        Field {
            key: "pinnedExtensions",
            kind: Kind::Custom(clean_pinned),
            default: || json!([]),
        },
        // --- Motion (AUDIT-42)
        Field {
            key: "reduceMotion",
            kind: Kind::Choice(&["Auto", "On", "Off"]),
            default: || json!("Auto"),
        },
        // --- Interface options (AUDIT-39)
        Field {
            key: "radius",
            kind: Kind::Choice(&["Sharp", "Default", "Round"]),
            default: || json!("Default"),
        },
        Field {
            key: "uiScale",
            kind: Kind::Choice(&["90%", "100%", "110%", "125%"]),
            default: || json!("100%"),
        },
        Field {
            key: "showNavButtons",
            kind: Kind::Bool,
            default: || json!(true),
        },
        Field {
            key: "showReload",
            kind: Kind::Bool,
            default: || json!(true),
        },
        Field {
            key: "showGuideBar",
            kind: Kind::Bool,
            default: || json!(true),
        },
        Field {
            key: "answerFont",
            kind: Kind::Choice(&["Sans", "Serif", "Mono"]),
            default: || json!("Sans"),
        },
        Field {
            key: "answerSize",
            kind: Kind::Choice(&["Small", "Default", "Large"]),
            default: || json!("Default"),
        },
        Field {
            key: "answerWidth",
            kind: Kind::Choice(&["Narrow", "Default", "Wide"]),
            default: || json!("Default"),
        },
        // --- Sky options (AUDIT-36)
        Field {
            key: "skyDensity",
            kind: Kind::Choice(&["25%", "50%", "75%", "100%"]),
            default: || json!("100%"),
        },
        Field {
            key: "skySpeed",
            kind: Kind::Choice(&["Slow", "Normal", "Fast"]),
            default: || json!("Normal"),
        },
        Field {
            key: "cometRate",
            kind: Kind::Choice(&["Rare", "Normal", "Often"]),
            default: || json!("Normal"),
        },
        Field {
            key: "lowPower",
            kind: Kind::Bool,
            default: || json!(false),
        },
        // --- Appearance
        Field {
            key: "accent",
            kind: Kind::Choice(&["Sea", "Mint", "Lilac", "Amber", "Rose"]),
            default: || json!("Sea"),
        },
        Field {
            key: "trail",
            kind: Kind::Choice(&["Narrow", "Wide"]),
            default: || json!("Narrow"),
        },
        // --- Themes (AUDIT-35): the preset name or the id of one of your own themes
        Field {
            key: "theme",
            kind: Kind::Text(40),
            default: || json!("Default"),
        },
        Field {
            key: "customThemes",
            kind: Kind::Custom(clean_custom_themes),
            default: || json!({}),
        },
        // --- New tab (AUDIT-37)
        Field {
            key: "showClock",
            kind: Kind::Bool,
            default: || json!(true),
        },
        Field {
            key: "clockFormat",
            kind: Kind::Choice(&["24-hour", "12-hour"]),
            default: || json!("24-hour"),
        },
        Field {
            key: "showGreeting",
            kind: Kind::Bool,
            default: || json!(true),
        },
        Field {
            key: "greetingName",
            kind: Kind::Text(30),
            default: || json!(""),
        },
        Field {
            key: "showTrailHint",
            kind: Kind::Bool,
            default: || json!(true),
        },
        Field {
            key: "wallpaperDim",
            kind: Kind::Choice(&["0%", "20%", "40%", "60%", "80%"]),
            default: || json!("40%"),
        },
        Field {
            key: "wallpaperBlur",
            kind: Kind::Choice(&["Off", "Light", "Medium", "Strong"]),
            default: || json!("Off"),
        },
        // --- General (AUDIT-38)
        Field {
            key: "searchEngine",
            kind: Kind::Choice(SEARCH_ENGINES),
            default: || json!("DuckDuckGo"),
        },
        Field {
            key: "customEngine",
            kind: Kind::Custom(clean_custom_engine),
            default: || json!({ "name": "", "url": "" }),
        },
        Field {
            key: "startup",
            kind: Kind::Choice(&["Restore tabs", "New tab"]),
            default: || json!("Restore tabs"),
        },
        Field {
            key: "linkTarget",
            kind: Kind::Choice(&["New tab", "Same tab"]),
            default: || json!("New tab"),
        },
        Field {
            key: "connections",
            kind: Kind::IntChoice(&[1, 4, 8, 16]),
            default: || json!(8),
        },
        // --- Tabs
        Field {
            key: "sleepMinutes",
            kind: Kind::IntRange(0, 1440),
            default: || json!(10),
        },
        // --- Keyboard shortcuts (AUDIT-41): { command id: [combo, ...] }, only the ones you changed
        Field {
            key: "shortcuts",
            kind: Kind::Custom(crate::shortcuts::sanitize_custom),
            default: || json!({}),
        },
        // --- Privacy (AUDIT-40)
        Field {
            key: "globalPrivacyControl",
            kind: Kind::Bool,
            default: || json!(true),
        },
        Field {
            key: "clearHistoryOnExit",
            kind: Kind::Bool,
            default: || json!(false),
        },
        Field {
            key: "clearSiteDataOnExit",
            kind: Kind::Bool,
            default: || json!(false),
        },
        // --- Passwords
        Field {
            key: "offerSavePasswords",
            kind: Kind::Bool,
            default: || json!(true),
        },
        Field {
            key: "autofillPasswords",
            kind: Kind::Bool,
            default: || json!(true),
        },
    ]
}

/// The default value of every setting.
pub fn defaults() -> Value {
    let mut m = Map::new();
    for f in fields() {
        m.insert(f.key.to_string(), (f.default)());
    }
    Value::Object(m)
}

/// The keys of the schema (for the test that compares them with the UI's defaults).
#[cfg(test)]
pub fn keys() -> Vec<&'static str> {
    fields().iter().map(|f| f.key).collect()
}

fn valid(kind: &Kind, v: &Value) -> Option<Value> {
    match kind {
        Kind::Choice(opts) => v.as_str().filter(|s| opts.contains(s)).map(|s| json!(s)),
        Kind::Bool => v.as_bool().map(|b| json!(b)),
        Kind::IntChoice(opts) => v.as_i64().filter(|n| opts.contains(n)).map(|n| json!(n)),
        Kind::IntRange(lo, hi) => v
            .as_i64()
            .filter(|n| (lo..=hi).contains(&n))
            .map(|n| json!(n)),
        Kind::FloatRange(lo, hi) => v
            .as_f64()
            .filter(|n| n.is_finite() && *n >= *lo && *n <= *hi)
            .map(|n| json!(n)),
        Kind::Text(max) => v
            .as_str()
            .filter(|s| s.chars().count() <= *max && !s.chars().any(char::is_control))
            .map(|s| json!(s)),
        Kind::Custom(f) => f(v),
    }
}

/// Turns anything into a complete, valid settings object.
pub fn normalize(input: &Value) -> Value {
    let given = input.as_object();
    let mut out = Map::new();
    for f in fields() {
        let v = given
            .and_then(|o| o.get(f.key))
            .and_then(|v| valid(&f.kind, v))
            .unwrap_or_else(|| (f.default)());
        out.insert(f.key.to_string(), v);
    }
    // A custom engine without a usable URL can't be the chosen engine.
    let custom_ok = out["customEngine"]["url"]
        .as_str()
        .is_some_and(|u| !u.is_empty());
    if out["searchEngine"] == "Custom" && !custom_ok {
        out.insert("searchEngine".into(), json!("DuckDuckGo"));
    }
    Value::Object(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_valid_and_complete() {
        let d = defaults();
        assert_eq!(normalize(&d), d, "defaults must normalise to themselves");
        for k in keys() {
            assert!(d.get(k).is_some(), "{k} has a default");
        }
    }

    #[test]
    fn anything_becomes_valid_settings() {
        for junk in [json!(null), json!(42), json!("x"), json!([1, 2]), json!({})] {
            assert_eq!(normalize(&junk), defaults());
        }
    }

    #[test]
    fn every_value_round_trips_through_save_and_reload() {
        let custom = json!({
            "stars": "Drifting", "shootingStars": true, "starsFollowCursor": false,
            "accent": "Rose", "trail": "Wide", "searchEngine": "Custom",
            "customEngine": { "name": "Mine", "url": "https://example.com/?q=%s" },
            "startup": "New tab", "linkTarget": "Same tab", "connections": 16,
            "sleepMinutes": 0, "globalPrivacyControl": false, "clearHistoryOnExit": true,
            "clearSiteDataOnExit": true, "showClock": false, "clockFormat": "12-hour", "showGreeting": false, "greetingName": "Sam", "showTrailHint": false, "wallpaperDim": "60%", "wallpaperBlur": "Light", "reduceMotion": "On", "pinnedExtensions": ["abc"], "autoUpdateCheck": false, "updateUrl": "https://example.com/latest.json", "aiTools": false, "aiControl": false, "radius": "Round", "uiScale": "110%", "showNavButtons": false, "showReload": false, "showGuideBar": false, "answerFont": "Mono", "answerSize": "Large", "answerWidth": "Wide", "skyDensity": "50%", "skySpeed": "Fast", "cometRate": "Rare", "lowPower": true, "theme": "Void", "customThemes": { "mine": { "name": "Mine", "bg": "#000000", "surface": "#111111", "panel": "#0a0a0a", "text": "#eeeeee", "accent": "#ff8800", "star": "#ffffff" } }, "shortcuts": { "reload": ["Ctrl+K"] }, "offerSavePasswords": false, "autofillPasswords": false
        });
        let once = normalize(&custom);
        assert_eq!(once, custom);
        let text = serde_json::to_string(&once).unwrap();
        assert_eq!(normalize(&serde_json::from_str(&text).unwrap()), custom);
    }

    #[test]
    fn invalid_values_fall_back_to_the_default_one_by_one() {
        let bad = json!({
            "stars": "Sparkly", "shootingStars": "yes", "accent": 7, "trail": null,
            "searchEngine": "Nope", "startup": "NewTab", "linkTarget": "Both",
            "connections": 3, "sleepMinutes": 99999, "offerSavePasswords": 1,
            "starsFollowCursor": false, "unknownKey": "dropped"
        });
        let n = normalize(&bad);
        let d = defaults();
        for k in [
            "stars",
            "shootingStars",
            "accent",
            "trail",
            "searchEngine",
            "startup",
            "linkTarget",
            "connections",
            "sleepMinutes",
            "offerSavePasswords",
        ] {
            assert_eq!(n[k], d[k], "{k} falls back");
        }
        assert_eq!(
            n["starsFollowCursor"],
            json!(false),
            "a valid value next to bad ones is kept"
        );
        assert!(n.get("unknownKey").is_none(), "unknown keys are dropped");
    }

    #[test]
    fn custom_search_engine_is_checked() {
        let ok = |url: &str| {
            normalize(
                &json!({ "searchEngine": "Custom", "customEngine": { "name": "X", "url": url } }),
            )
        };
        assert_eq!(ok("https://e.test/search?q=%s")["searchEngine"], "Custom");
        assert_eq!(ok("http://e.test/?q={query}")["searchEngine"], "Custom");
        for bad in [
            "javascript:alert(1)%s",
            "ftp://e.test/?q=%s",
            "https://e.test/?q=",
            "file:///c:/%s",
            "https://e.test/\n%s",
        ] {
            let n = ok(bad);
            assert_eq!(n["searchEngine"], "DuckDuckGo", "{bad} is rejected");
            assert_eq!(n["customEngine"]["url"], "", "{bad} is cleared");
        }
        // Custom chosen but no URL at all
        assert_eq!(
            normalize(&json!({ "searchEngine": "Custom" }))["searchEngine"],
            "DuckDuckGo"
        );
    }

    #[test]
    fn custom_themes_are_checked() {
        let good = json!({ "name": " Dusk ", "bg": "#000000", "surface": "#111111", "panel": "#0A0A0A",
            "text": "#EEEEEE", "accent": "#ff8800", "star": "#ffffff" });
        let mut bad_colour = good.clone();
        bad_colour["accent"] = json!("red; background:url(x)");
        let mut missing = good.clone();
        missing.as_object_mut().unwrap().remove("star");
        let mut no_name = good.clone();
        no_name["name"] = json!("  ");
        let n = normalize(&json!({ "customThemes": {
            "dusk": good, "Bad Id!": good, "c1": bad_colour, "c2": missing, "c3": no_name, "c4": 5 } }));
        let t = n["customThemes"].as_object().unwrap();
        assert_eq!(t.len(), 1, "{t:?}");
        assert_eq!(t["dusk"]["name"], "Dusk");
        assert_eq!(t["dusk"]["panel"], "#0a0a0a", "colours are lower-cased");
        let many: serde_json::Map<String, Value> =
            (0..20).map(|i| (format!("t{i}"), good.clone())).collect();
        assert_eq!(
            normalize(&json!({ "customThemes": many }))["customThemes"]
                .as_object()
                .unwrap()
                .len(),
            12
        );
        assert_eq!(
            normalize(&json!({ "customThemes": "x" }))["customThemes"],
            json!({})
        );
    }

    #[test]
    fn the_ui_defaults_match_the_schema() {
        // ui/settings.js DEFAULT_SETTINGS must list every key with the same simple default.
        let ui = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../ui/settings.js");
        let src = std::fs::read_to_string(ui).unwrap();
        let start = src
            .find("export const DEFAULT_SETTINGS = {")
            .expect("DEFAULT_SETTINGS");
        let block = &src[start..start + src[start..].find("};").unwrap()];
        let d = defaults();
        for k in keys() {
            let line = block
                .lines()
                .find(|l| l.trim_start().starts_with(&format!("{k}:")))
                .unwrap_or_else(|| panic!("{k} missing from DEFAULT_SETTINGS in ui/settings.js"));
            if let Some(expected) = d[k].as_str() {
                assert!(
                    line.contains(&format!("'{expected}'")),
                    "{k}: UI default differs ({line})"
                );
            } else if d[k].is_boolean() || d[k].is_number() {
                let lit = d[k].to_string();
                assert!(
                    line.split(':')
                        .nth(1)
                        .unwrap()
                        .trim_start()
                        .starts_with(&lit),
                    "{k}: UI default differs ({line})"
                );
            }
        }
    }
}
