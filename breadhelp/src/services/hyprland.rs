use serde::Deserialize;
use std::process::Command;

/// Runtime-only rebind via `hyprctl keyword bind`: takes effect immediately
/// and resets on the next Hyprland reload/login. `hyprland.lua` can't safely
/// be rewritten line-by-line by a program (hand-authored Lua, arbitrary
/// formatting), so this is the only rebind path — callers must label it
/// clearly as temporary in the UI.
///
/// `bind_value` is the full `<mods>,<key>,<dispatcher>,<args>` string, e.g.
/// `"SUPER,U,exec,breadpad"`.
pub fn rebind_temp(bind_value: &str) {
    if let Err(e) = Command::new("hyprctl").args(["keyword", "bind", bind_value]).spawn() {
        eprintln!("breadhelp: hyprctl rebind failed: {e}");
    }
}

#[derive(Debug, Deserialize)]
pub struct Client {
    pub class: String,
    pub at: (i32, i32),
    pub size: (i32, i32),
    /// Numeric monitor id — join against `monitors()` by `Monitor::id` for a name.
    pub monitor: i32,
}

#[derive(Debug, Deserialize)]
pub struct Monitor {
    pub id: i32,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub focused: bool,
}

#[derive(Debug, Clone)]
pub struct LayerClient {
    pub namespace: String,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub monitor: String,
}

fn query_json(cmd: &str) -> Option<serde_json::Value> {
    let output = Command::new("hyprctl").args(["-j", cmd]).output().ok()?;
    output.status.success().then(|| serde_json::from_slice(&output.stdout).ok()).flatten()
}

pub fn clients() -> Vec<Client> {
    query_json("clients").and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default()
}

pub fn monitors() -> Vec<Monitor> {
    query_json("monitors").and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default()
}

/// `hyprctl -j layers` nests surfaces as `{monitor_name: {levels: {level: [surface]}}}`
/// with no fixed schema the `hyprland` crate or any sibling repo has a typed struct
/// for, so this walks the `serde_json::Value` by hand instead of deriving `Deserialize`.
pub fn layers() -> Vec<LayerClient> {
    let Some(serde_json::Value::Object(by_monitor)) = query_json("layers") else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (monitor_name, monitor_val) in by_monitor {
        let Some(levels) = monitor_val.get("levels").and_then(|v| v.as_object()) else {
            continue;
        };
        for entries in levels.values().filter_map(|v| v.as_array()) {
            for entry in entries {
                let fields = (
                    entry.get("namespace").and_then(|v| v.as_str()),
                    entry.get("x").and_then(|v| v.as_i64()),
                    entry.get("y").and_then(|v| v.as_i64()),
                    entry.get("w").and_then(|v| v.as_i64()),
                    entry.get("h").and_then(|v| v.as_i64()),
                );
                if let (Some(namespace), Some(x), Some(y), Some(w), Some(h)) = fields {
                    out.push(LayerClient {
                        namespace: namespace.to_string(),
                        x: x as i32,
                        y: y as i32,
                        w: w as i32,
                        h: h as i32,
                        monitor: monitor_name.clone(),
                    });
                }
            }
        }
    }
    out
}
