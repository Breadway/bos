use gtk4::prelude::*;
use gtk4::{Box as GBox, Button, Entry, Label, ListBox, ListBoxRow, Orientation, ScrolledWindow};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::rc::Rc;

use crate::config;

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct BreadboxConfig {
    #[serde(default)]
    pub context: Vec<Context>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Context {
    pub name: String,
    #[serde(default)]
    pub apps: Vec<String>,
}

fn config_path() -> std::path::PathBuf {
    config::config_dir().join("breadbox/config.toml")
}

fn rebuild_list(list: &ListBox, cfg: &Rc<RefCell<BreadboxConfig>>) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
    for (i, ctx) in cfg.borrow().context.iter().enumerate() {
        let row = ListBoxRow::new();
        let hbox = GBox::new(Orientation::Horizontal, 8);
        hbox.set_margin_top(6);
        hbox.set_margin_bottom(6);
        hbox.set_margin_start(8);
        hbox.set_margin_end(8);

        let name_entry = Entry::new();
        name_entry.set_text(&ctx.name);
        name_entry.set_width_chars(16);

        let apps_entry = Entry::new();
        apps_entry.set_text(&ctx.apps.join(", "));
        apps_entry.set_hexpand(true);
        apps_entry.set_placeholder_text(Some("app1, app2, ..."));

        {
            let cfg = cfg.clone();
            name_entry.connect_changed(move |e| {
                if let Some(c) = cfg.borrow_mut().context.get_mut(i) {
                    c.name = e.text().to_string();
                }
            });
        }
        {
            let cfg = cfg.clone();
            apps_entry.connect_changed(move |e| {
                if let Some(c) = cfg.borrow_mut().context.get_mut(i) {
                    c.apps = e
                        .text()
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                }
            });
        }

        hbox.append(&name_entry);
        hbox.append(&apps_entry);
        row.set_child(Some(&hbox));
        list.append(&row);
    }
}

pub fn build() -> GBox {
    let path = config_path();
    let cfg: BreadboxConfig = config::load(&path).unwrap_or_default();
    let cfg = Rc::new(RefCell::new(cfg));

    let vbox = GBox::new(Orientation::Vertical, 12);
    vbox.add_css_class("view-content");

    let title = Label::new(Some("breadbox"));
    title.add_css_class("title");
    title.set_xalign(0.0);
    vbox.append(&title);

    let subtitle = Label::new(Some("Context priority lists — apps shown in each context."));
    subtitle.set_xalign(0.0);
    subtitle.set_margin_bottom(8);
    vbox.append(&subtitle);

    let list = ListBox::new();
    list.set_selection_mode(gtk4::SelectionMode::None);

    rebuild_list(&list, &cfg);

    let scroll = ScrolledWindow::new();
    scroll.set_vexpand(true);
    scroll.set_child(Some(&list));
    vbox.append(&scroll);

    let btn_row = GBox::new(Orientation::Horizontal, 8);
    btn_row.set_margin_top(8);

    let add_btn = Button::with_label("Add context");
    {
        let cfg = cfg.clone();
        let list = list.clone();
        add_btn.connect_clicked(move |_| {
            cfg.borrow_mut().context.push(Context {
                name: "new".to_string(),
                apps: Vec::new(),
            });
            rebuild_list(&list, &cfg);
        });
    }

    let save_btn = Button::with_label("Save");
    {
        let cfg = cfg.clone();
        let path = path.clone();
        save_btn.connect_clicked(move |_| {
            let _ = config::save(&path, &*cfg.borrow());
        });
    }

    btn_row.append(&add_btn);
    btn_row.append(&save_btn);
    vbox.append(&btn_row);

    vbox
}
