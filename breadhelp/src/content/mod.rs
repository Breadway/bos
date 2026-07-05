//! Guide content: `/usr/share/breadhelp/content/` (system, shipped by the
//! package) merged with `~/.local/share/breadhelp/content/` (user overrides/
//! additions). A guide directory name colliding between the two roots means
//! the user copy wins wholesale — no field-level merge, same "replace the
//! whole doc" discipline `bos-settings/src/config/mod.rs` uses for TOML, just
//! at the file-tree level.

pub mod keybinds;
pub mod markdown;
pub mod meta;
pub mod troubleshoot;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use meta::GuideMeta;

const SYSTEM_ROOT: &str = "/usr/share/breadhelp/content";

/// Categories are just top-level content directories — this only fixes the
/// *display order* of ones we know about; unrecognized directories still
/// show up (sorted alphabetically after these) instead of being hidden.
const CATEGORY_ORDER: &[&str] = &[
    "getting-started",
    "daily-use",
    "customization",
    "apps",
    "troubleshooting",
    "advanced",
];

pub struct Guide {
    pub id: String,
    pub category: String,
    pub meta: GuideMeta,
    pub dir: PathBuf,
}

pub struct ContentStore {
    guides: Vec<Guide>,
}

fn user_content_root() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        let p = PathBuf::from(xdg);
        if p.is_absolute() {
            return p.join("breadhelp/content");
        }
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    PathBuf::from(home).join(".local/share/breadhelp/content")
}

fn scan_root(root: &Path, out: &mut BTreeMap<(String, String), Guide>) {
    let Ok(categories) = std::fs::read_dir(root) else {
        return;
    };
    for cat_entry in categories.flatten() {
        let cat_path = cat_entry.path();
        if !cat_path.is_dir() {
            continue;
        }
        let category = cat_entry.file_name().to_string_lossy().into_owned();
        let Ok(guide_dirs) = std::fs::read_dir(&cat_path) else {
            continue;
        };
        for guide_entry in guide_dirs.flatten() {
            let guide_dir = guide_entry.path();
            if !guide_dir.is_dir() {
                continue;
            }
            let id = guide_entry.file_name().to_string_lossy().into_owned();
            // troubleshooting/_symptoms holds wizard data, not guides.
            if id == "_symptoms" {
                continue;
            }
            let meta_path = guide_dir.join("meta.toml");
            let Ok(meta_text) = std::fs::read_to_string(&meta_path) else {
                continue;
            };
            let Ok(meta) = toml::from_str::<GuideMeta>(&meta_text) else {
                eprintln!("breadhelp: {} failed to parse, skipping", meta_path.display());
                continue;
            };
            let effective_category = meta.category.clone().unwrap_or_else(|| category.clone());
            out.insert(
                (effective_category.clone(), id.clone()),
                Guide { id, category: effective_category, meta, dir: guide_dir },
            );
        }
    }
}

fn category_rank(category: &str) -> usize {
    CATEGORY_ORDER.iter().position(|c| *c == category).unwrap_or(CATEGORY_ORDER.len())
}

impl ContentStore {
    pub fn load() -> Self {
        let mut by_key = BTreeMap::new();
        scan_root(Path::new(SYSTEM_ROOT), &mut by_key);
        scan_root(&user_content_root(), &mut by_key);

        let mut guides: Vec<Guide> = by_key.into_values().collect();
        guides.sort_by(|a, b| {
            (category_rank(&a.category), &a.category, &a.id).cmp(&(category_rank(&b.category), &b.category, &b.id))
        });
        Self { guides }
    }

    pub fn categories(&self) -> Vec<String> {
        let mut seen = Vec::new();
        for g in &self.guides {
            if !seen.contains(&g.category) {
                seen.push(g.category.clone());
            }
        }
        seen
    }

    pub fn guides_in(&self, category: &str) -> Vec<&Guide> {
        self.guides.iter().filter(|g| g.category == category).collect()
    }

    pub fn guide(&self, category: &str, id: &str) -> Option<&Guide> {
        self.guides.iter().find(|g| g.category == category && g.id == id)
    }

    pub fn all(&self) -> &[Guide] {
        &self.guides
    }

    /// The guide body for the given mode — `content.beginner.md` when in a
    /// simplified mode and present, otherwise `content.md`. Never errors: a
    /// missing file just renders as an empty guide rather than crashing.
    pub fn body(&self, guide: &Guide, simplified: bool) -> String {
        if simplified {
            let beginner = guide.dir.join("content.beginner.md");
            if let Ok(text) = std::fs::read_to_string(&beginner) {
                return text;
            }
        }
        std::fs::read_to_string(guide.dir.join("content.md")).unwrap_or_default()
    }
}
