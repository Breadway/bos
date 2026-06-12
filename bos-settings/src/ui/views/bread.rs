use gtk4::prelude::*;
use gtk4::{Box as GBox, Button, DropDown, Label, Orientation, StringList, Switch};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::rc::Rc;

use crate::config;

#[derive(Deserialize, Serialize, Clone)]
pub struct BreadConfig {
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default)]
    pub adapters: AdaptersConfig,
}

fn default_log_level() -> String {
    "info".to_string()
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct AdaptersConfig {
    #[serde(default = "default_true")]
    pub keyboard: bool,
    #[serde(default = "default_true")]
    pub mouse: bool,
    #[serde(default = "default_true")]
    pub touchpad: bool,
    #[serde(default = "default_true")]
    pub bluetooth: bool,
    #[serde(default = "default_true")]
    pub gamepad: bool,
}

fn default_true() -> bool {
    true
}

impl Default for BreadConfig {
    fn default() -> Self {
        Self {
            log_level: default_log_level(),
            adapters: AdaptersConfig::default(),
        }
    }
}

fn config_path() -> std::path::PathBuf {
    config::config_dir().join("bread/breadd.toml")
}

fn adapter_row(label: &str, active: bool, cfg: Rc<RefCell<BreadConfig>>, field: &'static str) -> GBox {
    let row = GBox::new(Orientation::Horizontal, 16);
    let lbl = Label::new(Some(label));
    lbl.set_hexpand(true);
    lbl.set_xalign(0.0);
    let sw = Switch::new();
    sw.set_active(active);
    sw.connect_active_notify(move |s| {
        let val = s.is_active();
        let mut c = cfg.borrow_mut();
        match field {
            "keyboard" => c.adapters.keyboard = val,
            "mouse" => c.adapters.mouse = val,
            "touchpad" => c.adapters.touchpad = val,
            "bluetooth" => c.adapters.bluetooth = val,
            "gamepad" => c.adapters.gamepad = val,
            _ => {}
        }
    });
    row.append(&lbl);
    row.append(&sw);
    row
}

pub fn build() -> GBox {
    let path = config_path();
    let cfg: BreadConfig = config::load(&path).unwrap_or_default();
    let cfg = Rc::new(RefCell::new(cfg));

    let vbox = GBox::new(Orientation::Vertical, 12);
    vbox.add_css_class("view-content");

    let title = Label::new(Some("bread"));
    title.add_css_class("title");
    title.set_xalign(0.0);
    vbox.append(&title);

    // Log level
    let row = GBox::new(Orientation::Horizontal, 16);
    row.set_margin_bottom(8);
    let lbl = Label::new(Some("Log level"));
    lbl.set_hexpand(true);
    lbl.set_xalign(0.0);
    let levels = StringList::new(&["error", "warn", "info", "debug", "trace"]);
    let dropdown = DropDown::new(Some(levels), gtk4::Expression::NONE);
    let current_pos = match cfg.borrow().log_level.as_str() {
        "error" => 0u32,
        "warn" => 1,
        "info" => 2,
        "debug" => 3,
        "trace" => 4,
        _ => 2,
    };
    dropdown.set_selected(current_pos);
    {
        let cfg = cfg.clone();
        dropdown.connect_selected_notify(move |dd| {
            let levels = ["error", "warn", "info", "debug", "trace"];
            if let Some(&level) = levels.get(dd.selected() as usize) {
                cfg.borrow_mut().log_level = level.to_string();
            }
        });
    }
    row.append(&lbl);
    row.append(&dropdown);
    vbox.append(&row);

    // Adapter toggles
    let adapter_label = Label::new(Some("Adapters"));
    adapter_label.set_xalign(0.0);
    adapter_label.set_margin_top(8);
    adapter_label.set_margin_bottom(4);
    vbox.append(&adapter_label);

    let (kbd, mouse, touchpad, bluetooth, gamepad) = {
        let c = cfg.borrow();
        (c.adapters.keyboard, c.adapters.mouse, c.adapters.touchpad, c.adapters.bluetooth, c.adapters.gamepad)
    };

    vbox.append(&adapter_row("Keyboard", kbd, cfg.clone(), "keyboard"));
    vbox.append(&adapter_row("Mouse", mouse, cfg.clone(), "mouse"));
    vbox.append(&adapter_row("Touchpad", touchpad, cfg.clone(), "touchpad"));
    vbox.append(&adapter_row("Bluetooth", bluetooth, cfg.clone(), "bluetooth"));
    vbox.append(&adapter_row("Gamepad", gamepad, cfg.clone(), "gamepad"));

    let save_btn = Button::with_label("Save");
    save_btn.set_margin_top(16);
    save_btn.set_halign(gtk4::Align::Start);
    {
        let cfg = cfg.clone();
        save_btn.connect_clicked(move |_| {
            let _ = config::save(&path, &*cfg.borrow());
        });
    }
    vbox.append(&save_btn);

    vbox
}
