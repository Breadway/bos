use gtk4::prelude::*;
use gtk4::{Box as GBox, Button, Entry, Label, Orientation, Switch};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::rc::Rc;

use crate::config;

#[derive(Deserialize, Serialize, Clone)]
pub struct BreadpadConfig {
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_true")]
    pub reminders: bool,
    #[serde(default = "default_true")]
    pub calendar: bool,
}

fn default_model() -> String {
    "claude-sonnet-4-6".to_string()
}

fn default_true() -> bool {
    true
}

impl Default for BreadpadConfig {
    fn default() -> Self {
        Self {
            model: default_model(),
            reminders: true,
            calendar: true,
        }
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
    {
        let cfg = cfg.clone();
        model_entry.connect_changed(move |e| {
            cfg.borrow_mut().model = e.text().to_string();
        });
    }
    row.append(&lbl);
    row.append(&model_entry);
    vbox.append(&row);

    // Reminders toggle
    let row = GBox::new(Orientation::Horizontal, 16);
    let lbl = Label::new(Some("Reminders"));
    lbl.set_hexpand(true);
    lbl.set_xalign(0.0);
    let sw = Switch::new();
    sw.set_active(cfg.borrow().reminders);
    {
        let cfg = cfg.clone();
        sw.connect_active_notify(move |s| {
            cfg.borrow_mut().reminders = s.is_active();
        });
    }
    row.append(&lbl);
    row.append(&sw);
    vbox.append(&row);

    // Calendar toggle
    let row = GBox::new(Orientation::Horizontal, 16);
    let lbl = Label::new(Some("Calendar integration"));
    lbl.set_hexpand(true);
    lbl.set_xalign(0.0);
    let sw = Switch::new();
    sw.set_active(cfg.borrow().calendar);
    {
        let cfg = cfg.clone();
        sw.connect_active_notify(move |s| {
            cfg.borrow_mut().calendar = s.is_active();
        });
    }
    row.append(&lbl);
    row.append(&sw);
    vbox.append(&row);

    let save_btn = Button::with_label("Save");
    save_btn.set_margin_top(16);
    save_btn.set_halign(gtk4::Align::Start);
    {
        let cfg = cfg.clone();
        let path = path.clone();
        save_btn.connect_clicked(move |_| {
            let _ = config::save(&path, &*cfg.borrow());
        });
    }
    vbox.append(&save_btn);

    vbox
}
