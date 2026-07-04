//! fwupd firmware updates — thin GUI over `fwupdmgr`, following the same
//! "stream command output, refresh the list on completion" pattern as
//! Packages. fwupd itself (the daemon + CLI) is always installed;
//! `fwupd-refresh.timer` already keeps the metadata current in the
//! background, this panel is just for actually seeing/applying updates.

use gtk4::prelude::*;
use gtk4::{Box as GBox, Button, Label, ListBox, ListBoxRow, Orientation, ScrolledWindow, TextView};

use crate::ui::widgets as w;
use crate::ui::widgets::stream_command_then;

#[derive(Clone)]
struct FwDevice {
    name: String,
    version: String,
}

fn list_updatable() -> Vec<FwDevice> {
    let Ok(output) = std::process::Command::new("fwupdmgr")
        .args(["get-devices", "--json"])
        .output()
    else {
        return Vec::new();
    };
    let Ok(root) = serde_json::from_slice::<serde_json::Value>(&output.stdout) else {
        return Vec::new();
    };
    let Some(devices) = root.get("Devices").and_then(|d| d.as_array()) else {
        return Vec::new();
    };
    devices
        .iter()
        .filter(|d| {
            d.get("Flags")
                .and_then(|f| f.as_array())
                .is_some_and(|flags| flags.iter().any(|f| f.as_str() == Some("updatable")))
        })
        .filter_map(|d| {
            Some(FwDevice {
                name: d.get("Name")?.as_str()?.to_string(),
                version: d.get("Version").and_then(|v| v.as_str()).unwrap_or("unknown").to_string(),
            })
        })
        .collect()
}

fn populate(list: &ListBox) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
    let devices = list_updatable();
    if devices.is_empty() {
        let row = ListBoxRow::new();
        row.set_selectable(false);
        row.set_child(Some(&w::empty_state(
            "software-update-available-symbolic",
            "No updatable firmware devices found",
            "Not every device supports firmware updates through fwupd.",
        )));
        list.append(&row);
        return;
    }
    for dev in devices {
        let row = ListBoxRow::new();
        row.set_selectable(false);
        let hbox = GBox::new(Orientation::Horizontal, 16);
        hbox.add_css_class("card");
        hbox.set_margin_top(3);
        hbox.set_margin_bottom(3);

        let name_lbl = Label::new(Some(&dev.name));
        name_lbl.set_hexpand(true);
        name_lbl.set_xalign(0.0);
        let ver_lbl = Label::new(Some(&dev.version));
        ver_lbl.add_css_class("dim-label");
        ver_lbl.set_xalign(1.0);

        hbox.append(&name_lbl);
        hbox.append(&ver_lbl);
        row.set_child(Some(&hbox));
        list.append(&row);
    }
}

pub fn build() -> GBox {
    let (outer, content) = w::view_scaffold("Firmware");
    content.append(&w::hint(
        "Firmware updates for hardware that supports them (UEFI, some \
         peripherals). fwupd-refresh.timer already keeps update metadata \
         current in the background.",
    ));

    let list = ListBox::new();
    list.set_selection_mode(gtk4::SelectionMode::None);
    populate(&list);
    let scroll = ScrolledWindow::new();
    scroll.set_vexpand(true);
    scroll.set_child(Some(&list));
    content.append(&scroll);

    let log_buf = gtk4::TextBuffer::new(None);
    let log_view = TextView::with_buffer(&log_buf);
    log_view.set_editable(false);
    log_view.set_monospace(true);
    log_view.set_height_request(140);
    log_view.set_margin_top(8);
    log_view.set_visible(false);

    let btn_row = GBox::new(Orientation::Horizontal, 8);
    btn_row.set_margin_top(12);

    let check_btn = Button::with_label("Check for updates");
    {
        let log_buf = log_buf.clone();
        let log_view = log_view.clone();
        let list = list.clone();
        check_btn.connect_clicked(move |_| {
            log_buf.set_text("");
            log_view.set_visible(true);
            let list2 = list.clone();
            stream_command_then(&["fwupdmgr", "refresh"], log_buf.clone(), move || {
                populate(&list2);
            });
        });
    }

    let update_btn = Button::with_label("Update all");
    update_btn.add_css_class("suggested-action");
    {
        let log_buf = log_buf.clone();
        let log_view = log_view.clone();
        let list = list.clone();
        update_btn.connect_clicked(move |_| {
            log_buf.set_text("");
            log_view.set_visible(true);
            let list2 = list.clone();
            stream_command_then(&["fwupdmgr", "update", "-y"], log_buf.clone(), move || {
                populate(&list2);
            });
        });
    }

    let refresh_btn = Button::with_label("Refresh list");
    {
        let list = list.clone();
        refresh_btn.connect_clicked(move |_| {
            populate(&list);
        });
    }

    btn_row.append(&check_btn);
    btn_row.append(&update_btn);
    btn_row.append(&refresh_btn);
    content.append(&btn_row);
    content.append(&log_view);

    outer
}
