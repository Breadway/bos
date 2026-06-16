//! Non-destructive config editing.
//!
//! Every bread* app owns a TOML config that may contain keys, sections, and
//! comments this settings app does not model (e.g. breadpad's calendar
//! credentials, breadcrumbs' saved-network passwords). To edit safely we parse
//! the file into a `toml_edit::DocumentMut`, mutate only the specific keys the
//! UI exposes, and write the document back — preserving everything else,
//! formatting and comments included.

use std::error::Error;
use std::path::{Path, PathBuf};

use toml_edit::{value, Array, DocumentMut, Item, Table, Value};

/// Load a TOML file into an editable document. A missing or unparseable file
/// yields an empty document so the UI still renders (with defaults).
pub fn load_doc(path: &Path) -> DocumentMut {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| s.parse::<DocumentMut>().ok())
        .unwrap_or_default()
}

/// Write the document back to disk, creating parent dirs as needed.
pub fn save_doc(path: &Path, doc: &DocumentMut) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, doc.to_string())?;
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

// --- typed readers (walk a dotted path, return None if absent/wrong type) ---

fn get<'a>(doc: &'a DocumentMut, path: &[&str]) -> Option<&'a Item> {
    let mut tbl = doc.as_table();
    let (last, parents) = path.split_last()?;
    for key in parents {
        tbl = tbl.get(key)?.as_table()?;
    }
    tbl.get(last)
}

pub fn get_bool(doc: &DocumentMut, path: &[&str]) -> Option<bool> {
    get(doc, path)?.as_bool()
}
pub fn get_str(doc: &DocumentMut, path: &[&str]) -> Option<String> {
    get(doc, path)?.as_str().map(str::to_string)
}
pub fn get_i64(doc: &DocumentMut, path: &[&str]) -> Option<i64> {
    get(doc, path)?.as_integer()
}
pub fn get_f64(doc: &DocumentMut, path: &[&str]) -> Option<f64> {
    let item = get(doc, path)?;
    item.as_float().or_else(|| item.as_integer().map(|i| i as f64))
}
/// Read an array of strings (e.g. modules.disable, contexts[].priority).
pub fn get_str_list(doc: &DocumentMut, path: &[&str]) -> Vec<String> {
    match get(doc, path).and_then(Item::as_array) {
        Some(arr) => arr
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect(),
        None => Vec::new(),
    }
}

// --- setters (auto-create intermediate tables, replace only the leaf) ---

fn table_at_mut<'a>(doc: &'a mut DocumentMut, parents: &[&str]) -> &'a mut Table {
    let mut tbl = doc.as_table_mut();
    for key in parents {
        let entry = tbl.entry(key).or_insert_with(|| Item::Table(Table::new()));
        if !entry.is_table() {
            *entry = Item::Table(Table::new());
        }
        tbl = entry.as_table_mut().expect("just ensured table");
    }
    tbl
}

fn set_item(doc: &mut DocumentMut, path: &[&str], item: Item) {
    let Some((last, parents)) = path.split_last() else {
        return;
    };
    table_at_mut(doc, parents).insert(last, item);
}

pub fn set_bool(doc: &mut DocumentMut, path: &[&str], v: bool) {
    set_item(doc, path, value(v));
}
pub fn set_str(doc: &mut DocumentMut, path: &[&str], v: &str) {
    set_item(doc, path, value(v));
}
pub fn set_i64(doc: &mut DocumentMut, path: &[&str], v: i64) {
    set_item(doc, path, value(v));
}
pub fn set_f64(doc: &mut DocumentMut, path: &[&str], v: f64) {
    set_item(doc, path, value(v));
}
pub fn set_str_list(doc: &mut DocumentMut, path: &[&str], items: &[String]) {
    let mut arr = Array::new();
    for s in items {
        arr.push(s.as_str());
    }
    set_item(doc, path, Item::Value(Value::Array(arr)));
}

/// Set a string key, or remove it entirely when the value is empty — keeps
/// optional fields out of the file rather than persisting `key = ""`.
pub fn set_str_or_remove(doc: &mut DocumentMut, path: &[&str], v: &str) {
    if v.is_empty() {
        remove(doc, path);
    } else {
        set_str(doc, path, v);
    }
}

pub fn remove(doc: &mut DocumentMut, path: &[&str]) {
    if let Some((last, parents)) = path.split_last() {
        let mut tbl = doc.as_table_mut();
        for key in parents {
            match tbl.get_mut(key).and_then(Item::as_table_mut) {
                Some(t) => tbl = t,
                None => return,
            }
        }
        tbl.remove(last);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_preserve_unmodelled_keys_and_comments() {
        let src = "\
# a leading comment
[daemon]
log_level = \"info\"

[calendar]
password = \"secret\"  # keep me
";
        let mut doc: DocumentMut = src.parse().unwrap();
        // Modify a single modelled key.
        set_str(&mut doc, &["daemon", "log_level"], "debug");
        // A key/section the UI never touches must survive untouched.
        let out = doc.to_string();
        assert!(out.contains("log_level = \"debug\""));
        assert!(out.contains("password = \"secret\""));
        assert!(out.contains("# keep me"));
        assert!(out.contains("# a leading comment"));
    }

    #[test]
    fn setters_create_missing_tables() {
        let mut doc = DocumentMut::new();
        set_bool(&mut doc, &["adapters", "power", "enabled"], false);
        set_i64(&mut doc, &["adapters", "power", "poll_interval_secs"], 45);
        assert_eq!(get_bool(&doc, &["adapters", "power", "enabled"]), Some(false));
        assert_eq!(
            get_i64(&doc, &["adapters", "power", "poll_interval_secs"]),
            Some(45)
        );
    }

    #[test]
    fn empty_string_removes_key() {
        let mut doc: DocumentMut = "[calendar]\nurl = \"x\"\n".parse().unwrap();
        set_str_or_remove(&mut doc, &["calendar", "url"], "");
        assert_eq!(get_str(&doc, &["calendar", "url"]), None);
    }

    #[test]
    fn str_list_roundtrips() {
        let mut doc = DocumentMut::new();
        let items = vec!["a".to_string(), "b".to_string()];
        set_str_list(&mut doc, &["modules", "disable"], &items);
        assert_eq!(get_str_list(&doc, &["modules", "disable"]), items);
    }
}
