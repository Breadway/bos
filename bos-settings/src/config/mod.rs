use std::error::Error;
use std::path::Path;

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

pub fn config_dir() -> std::path::PathBuf {
    dirs_path()
}

fn dirs_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    std::path::PathBuf::from(home).join(".config")
}
