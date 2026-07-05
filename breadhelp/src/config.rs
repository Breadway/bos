//! `~/.config/breadhelp/state.toml` — non-destructive TOML editing, same
//! `load_doc`/`save_doc` discipline as `bos-settings/src/config/mod.rs`: a
//! missing file yields defaults, a file that exists but fails to parse is
//! backed up once before falling back, so a bad edit is always recoverable.

use std::path::{Path, PathBuf};
use toml_edit::{value, DocumentMut};

pub fn config_dir() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        let p = PathBuf::from(xdg);
        if p.is_absolute() {
            return p;
        }
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    PathBuf::from(home).join(".config")
}

fn state_path() -> PathBuf {
    config_dir().join("breadhelp").join("state.toml")
}

fn load_doc(path: &Path) -> DocumentMut {
    let Ok(text) = std::fs::read_to_string(path) else {
        return DocumentMut::default();
    };
    match text.parse::<DocumentMut>() {
        Ok(doc) => doc,
        Err(e) => {
            let backup = PathBuf::from(format!("{}.bak", path.display()));
            eprintln!(
                "breadhelp: {} failed to parse ({e}); backed up to {} before falling back to defaults",
                path.display(),
                backup.display()
            );
            let _ = std::fs::write(&backup, &text);
            DocumentMut::default()
        }
    }
}

fn save_doc(path: &Path, doc: &DocumentMut) {
    if let Some(parent) = path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            eprintln!("breadhelp: couldn't create {}: {e}", parent.display());
            return;
        }
    }
    if let Err(e) = std::fs::write(path, doc.to_string()) {
        eprintln!("breadhelp: couldn't write {}: {e}", path.display());
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Normal,
    Beginner,
    Dad,
    Compact,
}

impl Mode {
    pub fn as_str(self) -> &'static str {
        match self {
            Mode::Normal => "normal",
            Mode::Beginner => "beginner",
            Mode::Dad => "dad",
            Mode::Compact => "compact",
        }
    }

    fn from_str(s: &str) -> Self {
        match s {
            "beginner" => Mode::Beginner,
            "dad" => Mode::Dad,
            "compact" => Mode::Compact,
            _ => Mode::Normal,
        }
    }

    /// Beginner/Dad pick `content.beginner.md` over `content.md` when present.
    pub fn is_simplified(self) -> bool {
        matches!(self, Mode::Beginner | Mode::Dad)
    }
}

pub struct State {
    doc: DocumentMut,
    path: PathBuf,
}

impl State {
    pub fn load() -> Self {
        let path = state_path();
        let doc = load_doc(&path);
        Self { doc, path }
    }

    pub fn onboarding_completed(&self) -> bool {
        self.doc
            .get("onboarding")
            .and_then(|t| t.get("completed"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }

    pub fn set_onboarding_completed(&mut self, completed: bool) {
        self.doc["onboarding"]["completed"] = value(completed);
        self.save();
    }

    pub fn onboarding_step(&self) -> i64 {
        self.doc.get("onboarding").and_then(|t| t.get("step")).and_then(|v| v.as_integer()).unwrap_or(0)
    }

    pub fn set_onboarding_step(&mut self, step: i64) {
        self.doc["onboarding"]["step"] = value(step);
        self.save();
    }

    pub fn mode(&self) -> Mode {
        let s = self.doc.get("general").and_then(|t| t.get("mode")).and_then(|v| v.as_str()).unwrap_or("normal");
        Mode::from_str(s)
    }

    pub fn set_mode(&mut self, mode: Mode) {
        self.doc["general"]["mode"] = value(mode.as_str());
        self.save();
    }

    fn save(&self) {
        save_doc(&self.path, &self.doc);
    }
}
