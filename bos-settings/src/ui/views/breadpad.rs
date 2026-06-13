use gtk4::prelude::*;
use gtk4::{Box as GBox, Button, Entry, Label, Orientation, Switch};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::rc::Rc;

use crate::config;

#[derive(Deserialize, Serialize, Clone)]
pub struct BreadpadConfig {
    #[serde(default)]
    pub model: String,
    #[serde(default = "default_true")]
    pub reminders: bool,
    #[serde(default = "default_true")]
    pub calendar: bool,
}

fn default_true() -> bool { true }

impl Default for BreadpadConfig {
    fn default() -> Self {
        Self { model: String::new(), reminders: true, calendar: true }
    }
}

fn config_path() -> std::path::PathBuf {
    config::config_dir().join("breadpad/breadpad.toml")
}

pub fn build() -> GBox {
    let path = config_path();
    let cfg: BreadpadConfig = config::load(&path).unwrap_or_default();
    let cfg = Rc::new(RefCell::new(cfg));

    let vbox = GBox::new(Orientation::Vertical, 12);
    vbox.add_css_class("view-content");

    let title = Label::new(Some("breadpad"));
    title.add_css_class("title");
    title.set_xalign(0.0);
    vbox.append(&title);

    // Model entry
    let row = GBox::new(Orientation::Horizontal, 16);
    let lbl = Label::new(Some("Model"));
    lbl.set_hexpand(true);
    lbl.set_xalign(0.0);
    let model_entry = Entry::new();
    model_entry.set_text(&cfg.borrow().model);
    model_entry.set_placeholder_text(Some("e.g. claude-sonnet-4-6"));
    {
        let cfg = cfg.clone();
        model_entry.connect_changed(move |e| {
            cfg.borrow_mut().model = e.text().to_string();
        });
    }
    row.append(&lbl);
    row.append(&model_entry);
    vbox.append(&row);

    // Reminders
    let row = GBox::new(Orientation::Horizontal, 16);
    let lbl = Label::new(Some("Reminders"));
    lbl.set_hexpand(true);
    lbl.set_xalign(0.0);
    let sw = Switch::new();
    sw.set_active(cfg.borrow().reminders);
    {
        let cfg = cfg.clone();
        sw.connect_active_notify(move |s| { cfg.borrow_mut().reminders = s.is_active(); });
    }
    row.append(&lbl);
    row.append(&sw);
    vbox.append(&row);

    // Calendar
    let row = GBox::new(Orientation::Horizontal, 16);
    let lbl = Label::new(Some("Calendar integration"));
    lbl.set_hexpand(true);
    lbl.set_xalign(0.0);
    let sw = Switch::new();
    sw.set_active(cfg.borrow().calendar);
    {
        let cfg = cfg.clone();
        sw.connect_active_notify(move |s| { cfg.borrow_mut().calendar = s.is_active(); });
    }
    row.append(&lbl);
    row.append(&sw);
    vbox.append(&row);

    let btn_row = GBox::new(Orientation::Horizontal, 12);
    btn_row.set_margin_top(16);

    let save_btn = Button::with_label("Save");
    let status_lbl = Label::new(None);
    status_lbl.add_css_class("dim-label");

    {
        let cfg = cfg.clone();
        let status_lbl = status_lbl.clone();
        save_btn.connect_clicked(move |_| {
            match config::save(&path, &*cfg.borrow()) {
                Ok(()) => {
                    status_lbl.set_text("Saved");
                    let lbl = status_lbl.clone();
                    glib::timeout_add_seconds_local(3, move || {
                        lbl.set_text("");
                        glib::ControlFlow::Break
                    });
                }
                Err(e) => status_lbl.set_text(&format!("Error: {e}")),
            }
        });
    }

    btn_row.append(&save_btn);
    btn_row.append(&status_lbl);
    vbox.append(&btn_row);

    vbox
}
