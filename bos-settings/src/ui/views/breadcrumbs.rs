use gtk4::prelude::*;
use gtk4::{Box as GBox, Button, Entry, Label, ListBox, ListBoxRow, Orientation, ScrolledWindow};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::rc::Rc;

use crate::config;

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct BreadcrumbsConfig {
    #[serde(default)]
    pub profile: Vec<Profile>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Profile {
    pub name: String,
    #[serde(default)]
    pub ssids: Vec<String>,
}

fn config_path() -> std::path::PathBuf {
    config::config_dir().join("breadcrumbs/breadcrumbs.toml")
}

fn rebuild_list(list: &ListBox, cfg: &Rc<RefCell<BreadcrumbsConfig>>) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
    for (i, profile) in cfg.borrow().profile.iter().enumerate() {
        let row = ListBoxRow::new();
        row.set_selectable(false);

        let hbox = GBox::new(Orientation::Horizontal, 8);
        hbox.set_margin_top(6);
        hbox.set_margin_bottom(6);
        hbox.set_margin_start(8);
        hbox.set_margin_end(8);

        let name_entry = Entry::new();
        name_entry.set_text(&profile.name);
        name_entry.set_width_chars(14);
        name_entry.set_placeholder_text(Some("name"));

        let ssids_entry = Entry::new();
        ssids_entry.set_text(&profile.ssids.join(", "));
        ssids_entry.set_hexpand(true);
        ssids_entry.set_placeholder_text(Some("SSID1, SSID2, ..."));

        let remove_btn = Button::with_label("Remove");
        remove_btn.add_css_class("destructive-action");

        {
            let cfg = cfg.clone();
            name_entry.connect_changed(move |e| {
                if let Some(p) = cfg.borrow_mut().profile.get_mut(i) {
                    p.name = e.text().to_string();
                }
            });
        }
        {
            let cfg = cfg.clone();
            ssids_entry.connect_changed(move |e| {
                if let Some(p) = cfg.borrow_mut().profile.get_mut(i) {
                    p.ssids = e.text()
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                }
            });
        }
        {
            let cfg = cfg.clone();
            let list = list.clone();
            remove_btn.connect_clicked(move |_| {
                cfg.borrow_mut().profile.remove(i);
                rebuild_list(&list, &cfg);
            });
        }

        hbox.append(&name_entry);
        hbox.append(&ssids_entry);
        hbox.append(&remove_btn);
        row.set_child(Some(&hbox));
        list.append(&row);
    }
}

pub fn build() -> GBox {
    let path = config_path();
    let cfg: BreadcrumbsConfig = config::load(&path).unwrap_or_default();
    let cfg = Rc::new(RefCell::new(cfg));

    let vbox = GBox::new(Orientation::Vertical, 12);
    vbox.add_css_class("view-content");

    let title = Label::new(Some("breadcrumbs"));
    title.add_css_class("title");
    title.set_xalign(0.0);
    vbox.append(&title);

    let subtitle = Label::new(Some("Network profiles — SSIDs associated with each location."));
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

    let add_btn = Button::with_label("Add profile");
    {
        let cfg = cfg.clone();
        let list = list.clone();
        add_btn.connect_clicked(move |_| {
            cfg.borrow_mut().profile.push(Profile {
                name: "new".to_string(),
                ssids: Vec::new(),
            });
            rebuild_list(&list, &cfg);
        });
    }

    let save_btn = Button::with_label("Save");
    let status_lbl = Label::new(None);
    status_lbl.add_css_class("dim-label");

    {
        let cfg = cfg.clone();
        let path = path.clone();
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

    btn_row.append(&add_btn);
    btn_row.append(&save_btn);
    btn_row.append(&status_lbl);
    vbox.append(&btn_row);

    vbox
}
