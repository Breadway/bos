//! Loads `~/.config/hypr/binds.json` — the SAME structured data
//! `hyprland.lua` itself loads (via `scripts/input/{binds,keybinds}.lua`) to
//! generate the real Hyprland binds. Reading this directly, rather than a
//! separately-maintained cheatsheet, means the keybind viewer can never
//! drift from what's actually bound.

#[derive(Clone)]
pub struct Keybind {
    pub combo: String,
    pub action: String,
    #[allow(dead_code)] // reserved for Ask-tab filtering/future grouping
    pub category: String,
    /// What the onboarding tour's "Try it" button runs for this bind.
    pub demo_cmd: Option<String>,
    /// The `<dispatcher>,<args>` half of the real `hyprctl keyword bind`
    /// value (e.g. `"exec,breadbox"`) — only synthesized for `action: "exec"`
    /// entries, since binds.json's other actions ("close", "fullscreen", ...)
    /// are this app's own loader abstraction, not real Hyprland dispatcher
    /// names, and there's no safe generic way to reconstruct one. Binds
    /// without this simply don't offer a rebind button.
    pub raw: Option<String>,
}

#[derive(serde::Deserialize)]
struct RawEntry {
    action: String,
    #[serde(default)]
    command: Option<String>,
    key: String,
    #[serde(default)]
    mods: Option<Vec<String>>,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    category: String,
    #[serde(default)]
    demo_cmd: Option<String>,
}

#[derive(serde::Deserialize)]
struct BindsFile {
    #[serde(default)]
    default_mods: Vec<String>,
    #[serde(default)]
    bindings: Vec<RawEntry>,
}

fn binds_json_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    std::path::PathBuf::from(home).join(".config/hypr/binds.json")
}

pub fn load() -> Vec<Keybind> {
    let Ok(text) = std::fs::read_to_string(binds_json_path()) else {
        return Vec::new();
    };
    let file: BindsFile = match serde_json::from_str(&text) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("breadhelp: binds.json failed to parse: {e}");
            return Vec::new();
        }
    };
    let default_mods = if file.default_mods.is_empty() { vec!["SUPER".to_string()] } else { file.default_mods };

    file.bindings
        .into_iter()
        .map(|entry| {
            // An explicit "mods": [] (media keys) must NOT fall back to
            // default_mods — only an omitted `mods` field should, matching
            // scripts/input/keybinds.lua's own normalize_mods semantics.
            let mods = entry.mods.unwrap_or_else(|| default_mods.clone());
            let combo = if mods.is_empty() {
                entry.key.to_lowercase()
            } else {
                format!("{} + {}", mods.join(" + ").to_lowercase(), entry.key.to_lowercase())
            };
            let raw = (entry.action == "exec").then(|| entry.command.clone()).flatten().map(|c| format!("exec,{c}"));
            Keybind {
                combo,
                action: entry.label.unwrap_or(entry.action),
                category: entry.category,
                demo_cmd: entry.demo_cmd,
                raw,
            }
        })
        .collect()
}

/// Case-insensitive lookup by combo (e.g. "super + space") for `[Show
/// Keybind]` tags — falls back to `None` (rendered as raw text) rather than
/// panicking when a guide references a combo that doesn't exist.
pub fn find<'a>(binds: &'a [Keybind], combo: &str) -> Option<&'a Keybind> {
    let norm = normalize(combo);
    binds.iter().find(|b| normalize(&b.combo) == norm)
}

fn normalize(combo: &str) -> String {
    combo.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_lowercase()
}

/// Best-effort conversion of a human combo ("super + shift + u") into
/// hyprctl's `<mods>_<mods>,<key>` form ("SUPER_SHIFT,U") for the temporary
/// rebind feature — not a full Hyprland key-name parser, just enough for the
/// common single-letter/named-key binds this viewer lists.
pub fn to_hypr_mods_key(combo: &str) -> String {
    let parts: Vec<String> = combo
        .split(|c: char| c == '+' || c.is_whitespace())
        .filter(|s| !s.is_empty())
        .map(normalize_token)
        .collect();
    match parts.split_last() {
        None => String::new(),
        Some((key, [])) => key.clone(),
        Some((key, mods)) => format!("{},{}", mods.join("_"), key),
    }
}

fn normalize_token(tok: &str) -> String {
    match tok.to_lowercase().as_str() {
        "space" => "SPACE".to_string(),
        "return" | "enter" => "RETURN".to_string(),
        "backspace" => "BACKSPACE".to_string(),
        other => other.to_uppercase(),
    }
}
