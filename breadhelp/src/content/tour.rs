//! Live tour step loader — same shape of problem as `troubleshoot.rs`'s
//! symptom trees: TOML data, not hardcoded Rust, so the tour can be edited
//! without a rebuild. Fields are flat optionals rather than a tagged enum in
//! the TOML itself (mirroring `SymptomOption::goto`'s plain-string encoding)
//! to keep hand-authored `onboarding.toml` simple; `target()`/`success()`
//! turn that into the richer shape `ui::tour` actually wants to match on.

use std::path::Path;

#[derive(Clone)]
pub enum Target {
    /// A `gtk4-layer-shell` surface identified by namespace, e.g. "breadbox".
    Namespace(String),
    /// A plain toplevel window identified by its Wayland app-id/class.
    WindowClass(String),
    /// Pure-concept step — full-screen dim, centered callout, no spotlight.
    None,
}

#[derive(Clone)]
pub enum Success {
    /// Auto-advance when a `--tour-event <id>` matching this arrives.
    Event(String),
    /// Auto-advance after this many seconds if no event arrives first.
    Timeout(u64),
    /// Next button only — no event can signal this step's completion.
    Manual,
}

#[derive(serde::Deserialize, Clone)]
pub struct Step {
    pub id: String,
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub target_namespace: Option<String>,
    #[serde(default)]
    pub target_window_class: Option<String>,
    /// Shell command run when the step starts, e.g. launching breadbox so
    /// there's something to highlight. Skipped if the target is already
    /// resolvable — several bread apps toggle-close on a second invocation
    /// of the same command, so firing this unconditionally could close an
    /// already-open instance instead of opening one.
    #[serde(default)]
    pub launch: Option<String>,
    #[serde(default)]
    pub success_event: Option<String>,
    #[serde(default)]
    pub success_timeout_seconds: Option<u64>,
    /// A keybind combo (e.g. "super + shift + s") to temporarily rebind so
    /// its original action still runs but is chained with a
    /// `--tour-event <success_event>` ping — for steps whose action has no
    /// compositor-observable signal at all (e.g. taking a screenshot).
    /// Reverted the moment this step is left, and self-healed on the next
    /// launch if breadhelp crashes mid-step — see `ui::tour`.
    #[serde(default)]
    pub rebind_combo: Option<String>,
}

impl Step {
    pub fn target(&self) -> Target {
        if let Some(ns) = &self.target_namespace {
            Target::Namespace(ns.clone())
        } else if let Some(class) = &self.target_window_class {
            Target::WindowClass(class.clone())
        } else {
            Target::None
        }
    }

    pub fn success(&self) -> Success {
        if let Some(id) = &self.success_event {
            Success::Event(id.clone())
        } else if let Some(secs) = self.success_timeout_seconds {
            Success::Timeout(secs)
        } else {
            Success::Manual
        }
    }
}

#[derive(serde::Deserialize)]
struct TourFile {
    #[serde(rename = "step", default)]
    steps: Vec<Step>,
}

const SYSTEM_TOUR_PATH: &str = "/usr/share/breadhelp/content/tours/onboarding.toml";

pub fn load() -> Vec<Step> {
    let Ok(text) = std::fs::read_to_string(Path::new(SYSTEM_TOUR_PATH)) else {
        return Vec::new();
    };
    match toml::from_str::<TourFile>(&text) {
        Ok(f) => f.steps,
        Err(e) => {
            eprintln!("breadhelp: {SYSTEM_TOUR_PATH} failed to parse: {e}");
            Vec::new()
        }
    }
}
