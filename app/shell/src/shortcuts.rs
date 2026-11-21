//! Keyboard shortcuts you can change (AUDIT-41).
//!
//! One table of commands with their default keys. Your changes are stored in settings as
//! `shortcuts: { <command id>: [<combo>, ...] }` (a command you change keeps only your keys; a
//! command you never touched keeps its defaults). The engine turns a key press in a page window
//! into a command id, the UI does the same for the Constellations window, and the shell runs the
//! command, so a shortcut works everywhere with one definition.
//!
//! Rules for what may be bound (so typing and the OS keep working): a combo needs Ctrl or Alt, or
//! is a function key; Ctrl+Alt (which is AltGr on many keyboards) only with a function key; the
//! editing keys (copy, paste, select all, undo, arrows, Backspace, Delete, ...) and the OS's own
//! combos (Alt+F4, Alt+Tab, Ctrl+Shift+Esc ...) are reserved. Two commands never share a combo.

use serde_json::{Map, Value, json};
use std::collections::HashMap;

pub struct Command {
    pub id: &'static str,
    pub label: &'static str,
    pub defaults: &'static [&'static str],
}

pub const COMMANDS: &[Command] = &[
    Command {
        id: "ask",
        label: "Ask the assistant",
        defaults: &["Ctrl+J"],
    },
    Command {
        id: "newTab",
        label: "New tab",
        defaults: &["Ctrl+T"],
    },
    Command {
        id: "privateWindow",
        label: "New private window",
        defaults: &["Ctrl+Shift+N"],
    },
    Command {
        id: "closeTab",
        label: "Close tab",
        defaults: &["Ctrl+W", "Ctrl+F4"],
    },
    Command {
        id: "reopenTab",
        label: "Reopen closed tab",
        defaults: &["Ctrl+Shift+T"],
    },
    Command {
        id: "nextTab",
        label: "Next tab",
        defaults: &["Ctrl+Tab"],
    },
    Command {
        id: "prevTab",
        label: "Previous tab",
        defaults: &["Ctrl+Shift+Tab"],
    },
    Command {
        id: "address",
        label: "Focus the address bar",
        defaults: &["Ctrl+L", "F6"],
    },
    Command {
        id: "find",
        label: "Find in page",
        defaults: &["Ctrl+F"],
    },
    Command {
        id: "history",
        label: "History",
        defaults: &["Ctrl+H"],
    },
    Command {
        id: "settings",
        label: "Settings",
        defaults: &["Ctrl+,"],
    },
    Command {
        id: "reload",
        label: "Reload",
        defaults: &["Ctrl+R", "F5"],
    },
    Command {
        id: "back",
        label: "Back",
        defaults: &["Alt+Left"],
    },
    Command {
        id: "forward",
        label: "Forward",
        defaults: &["Alt+Right"],
    },
    Command {
        id: "zoomIn",
        label: "Zoom in",
        defaults: &["Ctrl+=", "Ctrl+Shift+=", "Ctrl+Numpad+"],
    },
    Command {
        id: "zoomOut",
        label: "Zoom out",
        defaults: &["Ctrl+-", "Ctrl+Numpad-"],
    },
    Command {
        id: "zoomReset",
        label: "Reset zoom",
        defaults: &["Ctrl+0", "Ctrl+Numpad0"],
    },
    Command {
        id: "print",
        label: "Print",
        defaults: &["Ctrl+P"],
    },
    Command {
        id: "devtools",
        label: "Developer tools",
        defaults: &["F12", "Ctrl+Shift+I"],
    },
];

/// A parsed key combination.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Combo {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    /// Canonical key name: `A`..`Z`, `0`..`9`, `F1`..`F12`, `Tab`, `Left`, `,`, `=`, `Numpad+` ...
    pub key: String,
}

impl Combo {
    /// `Ctrl+Shift+T`: modifiers in a fixed order, then the key.
    pub fn canonical(&self) -> String {
        let mut s = String::new();
        if self.ctrl {
            s.push_str("Ctrl+");
        }
        if self.alt {
            s.push_str("Alt+");
        }
        if self.shift {
            s.push_str("Shift+");
        }
        s.push_str(&self.key);
        s
    }
}

/// Windows virtual-key code of a key name (CEF reports the same codes on every platform).
pub fn vk_of(key: &str) -> Option<i32> {
    let k = key;
    if k.len() == 1 {
        let c = k.chars().next()?;
        return match c {
            'A'..='Z' | '0'..='9' => Some(c as i32),
            ',' => Some(0xBC),
            '.' => Some(0xBE),
            '/' => Some(0xBF),
            ';' => Some(0xBA),
            '\'' => Some(0xDE),
            '[' => Some(0xDB),
            ']' => Some(0xDD),
            '\\' => Some(0xDC),
            '`' => Some(0xC0),
            '-' => Some(0xBD),
            '=' => Some(0xBB),
            _ => None,
        };
    }
    if let Some(n) = k.strip_prefix('F').and_then(|n| n.parse::<i32>().ok())
        && (1..=12).contains(&n)
    {
        return Some(0x6F + n);
    }
    if let Some(n) = k.strip_prefix("Numpad") {
        return match n {
            "+" => Some(0x6B),
            "-" => Some(0x6D),
            "*" => Some(0x6A),
            "/" => Some(0x6F),
            "." => Some(0x6E),
            d if d.len() == 1 && d.chars().all(|c| c.is_ascii_digit()) => {
                Some(0x60 + d.parse::<i32>().ok()?)
            }
            _ => None,
        };
    }
    match k {
        "Tab" => Some(0x09),
        "Enter" => Some(0x0D),
        "Space" => Some(0x20),
        "Esc" => Some(0x1B),
        "Backspace" => Some(0x08),
        "Delete" => Some(0x2E),
        "Insert" => Some(0x2D),
        "Home" => Some(0x24),
        "End" => Some(0x23),
        "PageUp" => Some(0x21),
        "PageDown" => Some(0x22),
        "Left" => Some(0x25),
        "Up" => Some(0x26),
        "Right" => Some(0x27),
        "Down" => Some(0x28),
        _ => None,
    }
}

/// Parses `Ctrl+Shift+T` (any order, any case for modifiers). `None` if it isn't a known combo.
pub fn parse_combo(text: &str) -> Option<Combo> {
    let mut c = Combo {
        ctrl: false,
        shift: false,
        alt: false,
        key: String::new(),
    };
    let parts: Vec<&str> = text.split('+').collect();
    // "Ctrl++" and "Ctrl+Numpad+": the last part may itself be "+"; "Ctrl+" alone is invalid.
    let mut i = 0;
    while i < parts.len() {
        let p = parts[i];
        let last = i == parts.len() - 1;
        match p.to_ascii_lowercase().as_str() {
            "ctrl" | "control" if !last => c.ctrl = true,
            "alt" if !last => c.alt = true,
            "shift" if !last => c.shift = true,
            _ => {
                if !c.key.is_empty() {
                    return None;
                }
                // everything from here on is the key (it may contain a '+')
                let key = parts[i..].join("+");
                let key = if key.len() == 1 {
                    key.to_ascii_uppercase()
                } else {
                    key
                };
                c.key = normalise_key_name(&key)?;
                i = parts.len();
                continue;
            }
        }
        i += 1;
    }
    vk_of(&c.key)?;
    Some(c)
}

fn normalise_key_name(k: &str) -> Option<String> {
    let named = match k.to_ascii_lowercase().as_str() {
        "arrowleft" => "Left",
        "arrowright" => "Right",
        "arrowup" => "Up",
        "arrowdown" => "Down",
        "escape" => "Esc",
        _ => "",
    };
    if !named.is_empty() {
        return Some(named.to_string());
    }
    // keep canonical spelling of the multi-letter names
    for n in [
        "Tab",
        "Enter",
        "Space",
        "Esc",
        "Backspace",
        "Delete",
        "Insert",
        "Home",
        "End",
        "PageUp",
        "PageDown",
        "Left",
        "Up",
        "Right",
        "Down",
    ] {
        if k.eq_ignore_ascii_case(n) {
            return Some(n.to_string());
        }
    }
    if k.len() > 6 && k[..6].eq_ignore_ascii_case("numpad") {
        return Some(format!("Numpad{}", &k[6..]));
    }
    if k.len() >= 2 && k.starts_with(['f', 'F']) && k[1..].chars().all(|c| c.is_ascii_digit()) {
        return Some(k.to_ascii_uppercase());
    }
    Some(k.to_string())
}

/// Combos that may never be bound (editing keys and the OS's own).
fn reserved(c: &Combo) -> bool {
    let k = c.key.as_str();
    let ctrl_only = c.ctrl && !c.alt;
    if c.alt && !c.ctrl && !c.shift && matches!(k, "F4" | "Tab" | "Space" | "Esc") {
        return true;
    }
    if c.ctrl && c.shift && !c.alt && k == "Esc" {
        return true;
    }
    if c.ctrl && k == "Esc" {
        return true;
    }
    if ctrl_only && !c.shift && matches!(k, "C" | "V" | "X" | "A" | "Z" | "Y") {
        return true;
    }
    if ctrl_only && c.shift && matches!(k, "Z") {
        return true;
    }
    // text editing keys (word and line movement, selection, deleting) with Ctrl or Shift only
    if !c.alt
        && matches!(
            k,
            "Left"
                | "Right"
                | "Up"
                | "Down"
                | "Home"
                | "End"
                | "PageUp"
                | "PageDown"
                | "Backspace"
                | "Delete"
                | "Insert"
                | "Enter"
                | "Space"
                | "Esc"
        )
    {
        return true;
    }
    false
}

/// Why this combo can't be a shortcut, or `Ok(())`.
pub fn check_binding(c: &Combo) -> Result<(), &'static str> {
    let is_f = c.key.starts_with('F')
        && c.key.len() >= 2
        && c.key[1..].chars().all(|d| d.is_ascii_digit());
    if c.ctrl && c.alt && !is_f {
        return Err(
            "Ctrl+Alt is used to type characters on many keyboards. Use Ctrl or Alt, or a function key.",
        );
    }
    if !(c.ctrl || c.alt || is_f) {
        return Err("Use Ctrl or Alt with a key, or a function key.");
    }
    if reserved(c) {
        return Err(
            "That combination is used for editing or by Windows, so it can't be a shortcut.",
        );
    }
    Ok(())
}

/// The combos in effect for every command, after your changes. Invalid, reserved or conflicting
/// custom combos are ignored (the command keeps what it can); defaults always win a conflict.
pub fn effective(custom: &Value) -> Vec<(&'static Command, Vec<Combo>)> {
    let mut taken: HashMap<Combo, &'static str> = HashMap::new();
    let mut out = Vec::new();
    // Untouched commands first, so their defaults can't be stolen by a custom binding.
    let customised: Vec<&str> = COMMANDS
        .iter()
        .filter(|c| custom.get(c.id).is_some_and(Value::is_array))
        .map(|c| c.id)
        .collect();
    let mut chosen: HashMap<&'static str, Vec<Combo>> = HashMap::new();
    for cmd in COMMANDS.iter().filter(|c| !customised.contains(&c.id)) {
        let combos: Vec<Combo> = cmd.defaults.iter().filter_map(|d| parse_combo(d)).collect();
        for c in &combos {
            taken.insert(c.clone(), cmd.id);
        }
        chosen.insert(cmd.id, combos);
    }
    for cmd in COMMANDS.iter().filter(|c| customised.contains(&c.id)) {
        let mut combos = Vec::new();
        for v in custom[cmd.id].as_array().into_iter().flatten().take(4) {
            let Some(c) = v.as_str().and_then(parse_combo) else {
                continue;
            };
            if check_binding(&c).is_err() || taken.contains_key(&c) {
                continue;
            }
            taken.insert(c.clone(), cmd.id);
            combos.push(c);
        }
        chosen.insert(cmd.id, combos);
    }
    for cmd in COMMANDS {
        out.push((cmd, chosen.remove(cmd.id).unwrap_or_default()));
    }
    out
}

/// Binds `combo` to command `id` (replacing that command's keys). The error is a sentence for the
/// user: not a combo, not allowed, or already used by another command (which is named).
pub fn set_binding(custom: &Value, id: &str, combo: &str) -> Result<Value, String> {
    let cmd = COMMANDS
        .iter()
        .find(|c| c.id == id)
        .ok_or("Unknown command.")?;
    let c = parse_combo(combo).ok_or("That isn't a key combination.")?;
    check_binding(&c).map_err(str::to_string)?;
    let sane = sanitize_custom(custom).unwrap_or_else(|| json!({}));
    for (other, combos) in effective(&sane) {
        if other.id != id && combos.contains(&c) {
            return Err(format!(
                "{} already does \u{201c}{}\u{201d}. Change that one first.",
                c.canonical(),
                other.label
            ));
        }
    }
    let mut o = sane.as_object().cloned().unwrap_or_default();
    o.insert(cmd.id.to_string(), json!([c.canonical()]));
    Ok(Value::Object(o))
}

/// Puts one command (or every command) back to its default keys.
pub fn reset(custom: &Value, id: Option<&str>) -> Value {
    let mut o = sanitize_custom(custom)
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default();
    match id {
        Some(id) => {
            o.remove(id);
        }
        None => o.clear(),
    }
    Value::Object(o)
}

/// For the settings schema: keeps only known commands with valid, non-conflicting combos.
pub fn sanitize_custom(v: &Value) -> Option<Value> {
    let o = v.as_object()?;
    let eff = effective(v);
    let mut out = Map::new();
    for (cmd, combos) in eff {
        if o.get(cmd.id).is_some_and(Value::is_array) {
            let list: Vec<Value> = combos.iter().map(|c| json!(c.canonical())).collect();
            // a command whose every custom combo was invalid goes back to its defaults
            if !list.is_empty() {
                out.insert(cmd.id.to_string(), Value::Array(list));
            }
        }
    }
    Some(Value::Object(out))
}

/// What the UI shows and uses for matching: id, label, current keys, and whether customised.
pub fn table(custom: &Value) -> Value {
    let sane = sanitize_custom(custom).unwrap_or_else(|| json!({}));
    Value::Array(
        effective(&sane)
            .into_iter()
            .map(|(cmd, combos)| {
                json!({
                    "id": cmd.id,
                    "label": cmd.label,
                    "keys": combos.iter().map(Combo::canonical).collect::<Vec<_>>(),
                    "custom": sane.get(cmd.id).is_some(),
                })
            })
            .collect(),
    )
}

/// Default combinations that no command uses now (their command was moved). The engine swallows
/// them, so a key you reassigned doesn't keep working through Chromium's own handling.
pub fn unbound_defaults(custom: &Value) -> Vec<Combo> {
    let sane = sanitize_custom(custom).unwrap_or_else(|| json!({}));
    let used: Vec<Combo> = effective(&sane).into_iter().flat_map(|(_, c)| c).collect();
    COMMANDS
        .iter()
        .flat_map(|c| c.defaults.iter().filter_map(|k| parse_combo(k)))
        .filter(|c| !used.contains(c))
        .collect()
}

/// `(command id, combo)` pairs for the engine's key hook.
pub fn bindings(custom: &Value) -> Vec<(String, Combo)> {
    let sane = sanitize_custom(custom).unwrap_or_else(|| json!({}));
    effective(&sane)
        .into_iter()
        .flat_map(|(cmd, combos)| combos.into_iter().map(move |c| (cmd.id.to_string(), c)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_default_is_valid_and_no_two_commands_share_one() {
        let mut seen: HashMap<String, &str> = HashMap::new();
        for cmd in COMMANDS {
            assert!(!cmd.defaults.is_empty(), "{} has a default", cmd.id);
            for d in cmd.defaults {
                let c = parse_combo(d).unwrap_or_else(|| panic!("default {d} parses"));
                assert!(vk_of(&c.key).is_some(), "{d} has a key code");
                assert_eq!(c.canonical(), *d, "{d} is written canonically");
                // the only allowed rule exceptions are the browser's own defaults; none are reserved
                assert!(!reserved(&c), "{d} is not reserved");
                if let Some(other) = seen.insert(c.canonical(), cmd.id) {
                    panic!("{d} is bound to both {other} and {}", cmd.id);
                }
            }
        }
    }

    #[test]
    fn parsing_is_forgiving_about_order_case_and_plus() {
        assert_eq!(
            parse_combo("shift+ctrl+t").unwrap().canonical(),
            "Ctrl+Shift+T"
        );
        assert_eq!(
            parse_combo("Alt+ArrowLeft").unwrap().canonical(),
            "Alt+Left"
        );
        assert_eq!(
            parse_combo("Ctrl+Numpad+").unwrap().canonical(),
            "Ctrl+Numpad+"
        );
        assert_eq!(parse_combo("ctrl+,").unwrap().key, ",");
        assert_eq!(parse_combo("f5").unwrap().canonical(), "F5");
        for bad in [
            "",
            "Ctrl+",
            "Ctrl+Ctrl",
            "Ctrl+Shift",
            "Ctrl+Nope",
            "Ctrl+T+Y",
            "Hyper+T",
            "Ctrl+F13",
        ] {
            assert!(parse_combo(bad).is_none(), "{bad:?} is not a combo");
        }
    }

    #[test]
    fn typing_and_the_os_stay_safe() {
        let ok = |s: &str| check_binding(&parse_combo(s).unwrap());
        assert!(
            ok("Ctrl+K").is_ok()
                && ok("Alt+J").is_ok()
                && ok("F9").is_ok()
                && ok("Ctrl+Shift+K").is_ok()
        );
        for bad in [
            "K",
            "Shift+K",
            "Shift+F",
            "Ctrl+Alt+K",
            "Ctrl+C",
            "Ctrl+V",
            "Ctrl+X",
            "Ctrl+A",
            "Ctrl+Z",
            "Ctrl+Y",
            "Ctrl+Shift+Z",
            "Alt+F4",
            "Alt+Tab",
            "Ctrl+Shift+Esc",
            "Ctrl+Left",
            "Ctrl+Backspace",
            "Ctrl+Delete",
            "Ctrl+Home",
            "Ctrl+Space",
            "Ctrl+Enter",
            "Shift+Insert",
        ] {
            assert!(ok(bad).is_err(), "{bad} must be refused");
        }
        assert!(
            ok("Ctrl+Alt+F9").is_ok(),
            "a function key with Ctrl+Alt is fine"
        );
    }

    #[test]
    fn custom_bindings_replace_defaults_and_conflicts_are_dropped() {
        // "reload" takes Ctrl+K; "find" tries Ctrl+T (newTab's) and Ctrl+C (reserved): both ignored.
        let custom = json!({ "reload": ["Ctrl+K"], "find": ["Ctrl+T", "Ctrl+C"], "bogus": ["Ctrl+Q"], "back": "nope" });
        let t = table(&custom);
        let get = |id: &str| {
            t.as_array()
                .unwrap()
                .iter()
                .find(|c| c["id"] == id)
                .unwrap()
                .clone()
        };
        assert_eq!(
            get("reload")["keys"],
            json!(["Ctrl+K"]),
            "your keys replace the defaults"
        );
        assert_eq!(get("reload")["custom"], json!(true));
        assert_eq!(
            get("newTab")["keys"],
            json!(["Ctrl+T"]),
            "defaults keep their keys"
        );
        assert_eq!(
            get("find")["keys"],
            json!(["Ctrl+F"]),
            "all-invalid custom list falls back to the defaults"
        );
        assert_eq!(get("find")["custom"], json!(false));
        assert_eq!(
            get("back")["custom"],
            json!(false),
            "a non-list value is ignored"
        );
        let clean = sanitize_custom(&custom).unwrap();
        assert!(
            clean.get("bogus").is_none()
                && clean.get("find").is_none()
                && clean["reload"] == json!(["Ctrl+K"])
        );
    }

    #[test]
    fn a_custom_binding_cannot_steal_another_commands_default() {
        let custom = json!({ "reload": ["Ctrl+W"] });
        let t = table(&custom);
        let reload = t
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["id"] == "reload")
            .unwrap();
        assert_eq!(
            reload["keys"],
            json!(["Ctrl+R", "F5"]),
            "Ctrl+W still closes the tab; reload keeps its defaults"
        );
        assert_eq!(sanitize_custom(&custom).unwrap(), json!({}));
    }

    #[test]
    fn setting_and_resetting_a_binding_gives_clear_messages() {
        let none = json!({});
        let after = set_binding(&none, "reload", "ctrl+k").unwrap();
        assert_eq!(after["reload"], json!(["Ctrl+K"]));
        // another command already has it
        let err = set_binding(&after, "find", "Ctrl+K").unwrap_err();
        assert!(err.contains("Ctrl+K") && err.contains("Reload"), "{err}");
        // a default of an untouched command
        assert!(
            set_binding(&none, "find", "Ctrl+T")
                .unwrap_err()
                .contains("New tab")
        );
        // not allowed, not a combo, unknown command
        assert!(
            set_binding(&none, "find", "K")
                .unwrap_err()
                .contains("Ctrl or Alt")
        );
        assert!(
            set_binding(&none, "find", "Ctrl+C")
                .unwrap_err()
                .contains("editing")
        );
        assert!(set_binding(&none, "find", "nonsense").is_err());
        assert!(set_binding(&none, "nope", "Ctrl+K").is_err());
        // rebinding a command to the key it already has is fine
        assert!(set_binding(&none, "find", "Ctrl+F").is_ok());
        // a command's old default is free for others once it has moved
        let moved = set_binding(&none, "newTab", "Ctrl+Alt+F9").unwrap();
        assert!(set_binding(&moved, "find", "Ctrl+T").is_ok());
        assert_eq!(reset(&after, Some("reload")), json!({}));
        assert_eq!(reset(&moved, None), json!({}));
    }

    #[test]
    fn moved_defaults_are_swallowed_not_left_to_chromium() {
        assert!(unbound_defaults(&json!({})).is_empty());
        let moved = json!({ "reload": ["Ctrl+K"] });
        let free = unbound_defaults(&moved);
        assert!(free.iter().any(|c| c.key == "F5"));
        assert!(free.iter().any(|c| c.key == "R" && c.ctrl));
        assert!(!free.iter().any(|c| c.key == "K"));
    }

    #[test]
    fn the_engine_gets_every_combo_with_its_key_code() {
        let b = bindings(&json!({}));
        let find = |id: &str, vk: i32| b.iter().any(|(i, c)| i == id && vk_of(&c.key) == Some(vk));
        assert!(
            find("newTab", 0x54)
                && find("address", 0x75)
                && find("devtools", 0x7B)
                && find("zoomIn", 0xBB)
        );
        assert!(find("back", 0x25) && find("nextTab", 0x09) && find("settings", 0xBC));
        assert!(b.len() >= COMMANDS.len());
    }
}
