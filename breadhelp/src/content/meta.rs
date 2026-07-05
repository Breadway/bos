//! `meta.toml` schema — one per guide directory.

#[derive(serde::Deserialize, Clone, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Difficulty {
    #[default]
    Beginner,
    Intermediate,
    Advanced,
}

#[derive(serde::Deserialize, Clone, Default)]
pub struct GuideMeta {
    pub title: String,
    #[serde(default)]
    pub difficulty: Difficulty,
    #[serde(default)]
    pub tags: Vec<String>,
    /// Guide dir-names in the same or another category. Unresolvable entries
    /// are silently dropped when rendering "related guides" — never a crash.
    #[serde(default)]
    pub related: Vec<String>,
    #[serde(default)]
    pub estimated_minutes: u32,
    /// Overrides the parent directory name as the guide's category, if set.
    #[serde(default)]
    pub category: Option<String>,
}
