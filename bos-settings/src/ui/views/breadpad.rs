//! breadpad.toml — the breadpad notes/reminders config.
//! Schema mirrors breadpad-shared/src/config.rs (settings, model + model.ollama,
//! reminders, calendar). Edited non-destructively (the calendar password and
//! model paths are preserved across saves).

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::Box as GBox;

use crate::config;
use crate::ui::widgets as w;

fn config_path() -> std::path::PathBuf {
    config::config_dir().join("breadpad/breadpad.toml")
}

pub fn build() -> GBox {
    let path = config_path();
    let doc = Rc::new(RefCell::new(config::load_doc(&path)));

    let (outer, c) = w::view_scaffold("breadpad");

    c.append(&w::section("Capture"));
    c.append(&w::dropdown_row(
        "Default note type",
        &doc,
        &["settings", "default_type"],
        &["note", "reminder", "task"],
        "note",
    ));
    c.append(&w::switch_row(
        "Tag with active workspace",
        &doc,
        &["settings", "workspace_tag"],
        true,
    ));
    c.append(&w::csv_row(
        "Snooze options",
        &doc,
        &["settings", "snooze_options"],
        "15m, 1h, tomorrow_morning",
    ));
    c.append(&w::spin_row(
        "Archive after (days)",
        &doc,
        &["settings", "archive_after_days"],
        0.0,
        3650.0,
        1.0,
        30,
    ));

    c.append(&w::section("Classifier model"));
    c.append(&w::entry_row(
        "ONNX model path",
        &doc,
        &["model", "path"],
        "~/.local/share/breadpad/model/classifier.onnx",
        "",
    ));
    c.append(&w::entry_row(
        "Tokenizer path",
        &doc,
        &["model", "tokenizer"],
        "~/.local/share/breadpad/model/tokenizer.json",
        "",
    ));

    c.append(&w::section("Ollama (LLM classifier)"));
    c.append(&w::switch_row(
        "Use Ollama",
        &doc,
        &["model", "ollama", "enabled"],
        true,
    ));
    c.append(&w::entry_row(
        "Endpoint",
        &doc,
        &["model", "ollama", "endpoint"],
        "http://localhost:11434",
        "",
    ));
    c.append(&w::entry_row(
        "Model",
        &doc,
        &["model", "ollama", "model"],
        "e.g. fastflowlm",
        "",
    ));
    c.append(&w::spin_f64_row(
        "Confidence threshold",
        &doc,
        &["model", "ollama", "confidence_threshold"],
        0.0,
        1.0,
        0.05,
        2,
        0.6,
    ));

    c.append(&w::section("Reminders"));
    c.append(&w::entry_row(
        "Default morning time",
        &doc,
        &["reminders", "default_morning"],
        "7:00",
        "",
    ));
    c.append(&w::spin_row(
        "Missed grace (minutes)",
        &doc,
        &["reminders", "missed_grace_minutes"],
        0.0,
        1440.0,
        5.0,
        60,
    ));

    c.append(&w::section("Calendar (CalDAV)"));
    c.append(&w::switch_row(
        "Sync to calendar",
        &doc,
        &["calendar", "enabled"],
        false,
    ));
    c.append(&w::entry_row(
        "CalDAV URL",
        &doc,
        &["calendar", "url"],
        "https://host/remote.php/dav/calendars/...",
        "",
    ));
    c.append(&w::entry_row(
        "Username",
        &doc,
        &["calendar", "username"],
        "",
        "",
    ));
    c.append(&w::password_row("Password", &doc, &["calendar", "password"]));

    outer.append(&w::save_button(&doc, path));
    outer
}
