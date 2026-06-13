use std::error::Error;
use std::path::{Path, PathBuf};

pub fn load<T: for<'de> serde::Deserialize<'de>>(path: &Path) -> Result<T, Box<dyn Error>> {
    let text = std::fs::read_to_string(path)?;
    Ok(toml::from_str(&text)?)
}

pub fn save<T: serde::Serialize>(path: &Path, val: &T) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, toml::to_string_pretty(val)?)?;
    Ok(())
}

pub fn config_dir() -> PathBuf {
    // Honour XDG_CONFIG_HOME if set; otherwise fall back to $HOME/.config.
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        let p = PathBuf::from(xdg);
        if p.is_absolute() {
            return p;
        }
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    PathBuf::from(home).join(".config")
}
