use gtk4::prelude::*;
use gtk4::{
    Box as GBox, Button, Label, ListBox, ListBoxRow, Orientation, ScrolledWindow, TextView,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::process::Command;

#[derive(Deserialize, Default)]
struct InstalledPackages {
    #[serde(flatten)]
    packages: HashMap<String, PackageInfo>,
}

#[derive(Deserialize)]
struct PackageInfo {
    version: String,
}

fn read_installed() -> HashMap<String, String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    let path = std::path::Path::new(&home)
        .join(".local/state/bakery/installed.json");

    let Ok(text) = std::fs::read_to_string(&path) else {
        return HashMap::new();
    };

    let Ok(parsed) = serde_json::from_str::<HashMap<String, serde_json::Value>>(&text) else {
        return HashMap::new();
    };

    parsed
        .into_iter()
        .filter_map(|(name, val)| {
            let version = val
                .get("version")
                .or_else(|| val.as_str().map(|_| &val))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| "unknown".to_string());
            Some((name, version))
        })
        .collect()
}

pub fn build() -> GBox {
    let vbox = GBox::new(Orientation::Vertical, 0);
    vbox.add_css_class("view-content");

    let title = Label::new(Some("Packages"));
    title.add_css_class("title");
    title.set_xalign(0.0);
    vbox.append(&title);

    let subtitle = Label::new(Some("Bread ecosystem packages installed via bakery."));
    subtitle.set_xalign(0.0);
    subtitle.set_margin_bottom(16);
    vbox.append(&subtitle);

    let list = ListBox::new();
    list.set_selection_mode(gtk4::SelectionMode::None);

    let packages = read_installed();
    if packages.is_empty() {
        let row = ListBoxRow::new();
        let lbl = Label::new(Some("No bakery packages found (~/.local/state/bakery/installed.json)"));
        lbl.set_margin_top(8);
        lbl.set_margin_bottom(8);
        lbl.set_margin_start(8);
        row.set_child(Some(&lbl));
        list.append(&row);
    } else {
        let mut names: Vec<_> = packages.iter().collect();
        names.sort_by_key(|(k, _)| k.as_str());
        for (name, version) in names {
            let row = ListBoxRow::new();
            let hbox = GBox::new(Orientation::Horizontal, 16);
            hbox.set_margin_top(6);
            hbox.set_margin_bottom(6);
            hbox.set_margin_start(8);
            hbox.set_margin_end(8);

            let name_lbl = Label::new(Some(name));
            name_lbl.set_hexpand(true);
            name_lbl.set_xalign(0.0);

            let ver_lbl = Label::new(Some(version));
            ver_lbl.set_xalign(1.0);

            let update_btn = Button::with_label("Update");
            let pkg_name = name.clone();
            update_btn.connect_clicked(move |_| {
                let _ = Command::new("bakery")
                    .args(["update", &pkg_name])
                    .spawn();
            });

            hbox.append(&name_lbl);
            hbox.append(&ver_lbl);
            hbox.append(&update_btn);
            row.set_child(Some(&hbox));
            list.append(&row);
        }
    }

    let scroll = ScrolledWindow::new();
    scroll.set_vexpand(true);
    scroll.set_child(Some(&list));
    vbox.append(&scroll);

    let btn_row = GBox::new(Orientation::Horizontal, 8);
    btn_row.set_margin_top(12);

    let check_btn = Button::with_label("Check for updates");
    let update_all_btn = Button::with_label("Update all");

    let log_buf = gtk4::TextBuffer::new(None);
    let log_view = TextView::with_buffer(&log_buf);
    log_view.set_editable(false);
    log_view.set_height_request(120);
    log_view.set_margin_top(8);

    {
        let log_buf = log_buf.clone();
        check_btn.connect_clicked(move |_| {
            log_buf.set_text("Checking for updates...\n");
            match Command::new("bakery").args(["list"]).output() {
                Ok(out) => {
                    let text = String::from_utf8_lossy(&out.stdout);
                    log_buf.set_text(&format!("{text}\n"));
                }
                Err(e) => {
                    log_buf.set_text(&format!("Error: {e}\n"));
                }
            }
        });
    }

    {
        let log_buf = log_buf.clone();
        update_all_btn.connect_clicked(move |_| {
            log_buf.set_text("Running bakery update --all...\n");
            let (sender, receiver) = glib::MainContext::channel(glib::Priority::DEFAULT);
            std::thread::spawn(move || {
                let result = Command::new("bakery")
                    .args(["update", "--all"])
                    .output();
                match result {
                    Ok(out) => {
                        let _ = sender.send(String::from_utf8_lossy(&out.stdout).to_string());
                    }
                    Err(e) => {
                        let _ = sender.send(format!("Error: {e}\n"));
                    }
                }
            });
            receiver.attach(None, move |msg| {
                log_buf.set_text(&msg);
                glib::ControlFlow::Break
            });
        });
    }

    btn_row.append(&check_btn);
    btn_row.append(&update_all_btn);
    vbox.append(&btn_row);
    vbox.append(&log_view);

    vbox
}
