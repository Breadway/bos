use gtk4::prelude::*;
use gtk4::{
    Box as GBox, Button, Label, ListBox, ListBoxRow, Orientation, ScrolledWindow, TextView,
};
use std::collections::HashMap;

use crate::ui::widgets as w;
use crate::ui::widgets::{stream_command, stream_command_then};

fn read_installed() -> HashMap<String, String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    let path = std::path::Path::new(&home)
        .join(".local/state/bakery/installed.json");

    let Ok(text) = std::fs::read_to_string(&path) else {
        return HashMap::new();
    };
    let Ok(mut parsed) = serde_json::from_str::<serde_json::Value>(&text) else {
        return HashMap::new();
    };
    // installed.json is {"packages": {name: {version, binaries, services}}},
    // not a flat map of package name to metadata — without unwrapping this,
    // every install shows a single bogus row named "packages".
    let Some(packages) = parsed.get_mut("packages").map(std::mem::take) else {
        return HashMap::new();
    };
    let Ok(packages) = serde_json::from_value::<HashMap<String, serde_json::Value>>(packages) else {
        return HashMap::new();
    };

    packages
        .into_iter()
        .filter_map(|(name, val)| {
            let version = val
                .get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            Some((name, version))
        })
        .collect()
}

fn populate_packages(list: &ListBox, log_buf: &gtk4::TextBuffer, log_view: &TextView) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }

    let packages = read_installed();
    if packages.is_empty() {
        let row = ListBoxRow::new();
        row.set_selectable(false);
        row.set_child(Some(&w::empty_state(
            "package-x-generic-symbolic",
            "No bakery packages found",
            "~/.local/state/bakery/installed.json is missing or empty.",
        )));
        list.append(&row);
        return;
    }

    let mut names: Vec<_> = packages.iter().collect();
    names.sort_by_key(|(k, _)| k.as_str());

    for (name, version) in names {
        let row = ListBoxRow::new();
        row.set_selectable(false);
        let hbox = GBox::new(Orientation::Horizontal, 16);
        hbox.add_css_class("card");
        hbox.set_margin_top(3);
        hbox.set_margin_bottom(3);

        let name_lbl = Label::new(Some(name));
        name_lbl.set_hexpand(true);
        name_lbl.set_xalign(0.0);

        let ver_lbl = Label::new(Some(version));
        ver_lbl.set_xalign(1.0);

        let pkg_name = name.clone();
        let update_btn = Button::with_label("Update");
        {
            let log_buf = log_buf.clone();
            let log_view = log_view.clone();
            let list = list.clone();
            update_btn.connect_clicked(move |_| {
                log_buf.set_text("");
                log_view.set_visible(true);
                let list2 = list.clone();
                let log_buf2 = log_buf.clone();
                let log_view2 = log_view.clone();
                // Route through stream_command (like the other buttons) so
                // output is visible and the row refreshes with the new
                // version once the update actually finishes — previously
                // this was fire-and-forget with the Err case silently
                // swallowed, so a missing `bakery` binary made the button
                // look broken with zero feedback either way.
                stream_command_then(&["bakery", "update", &pkg_name], log_buf.clone(), move || {
                    populate_packages(&list2, &log_buf2, &log_view2);
                });
            });
        }

        hbox.append(&name_lbl);
        hbox.append(&ver_lbl);
        hbox.append(&update_btn);
        row.set_child(Some(&hbox));
        list.append(&row);
    }
}

pub fn build() -> GBox {
    let (outer, content) = w::view_scaffold("Packages");
    content.append(&w::hint(
        "Bread ecosystem packages installed via bakery, and system packages via pacman below.",
    ));

    let list = ListBox::new();
    list.set_selection_mode(gtk4::SelectionMode::None);

    let log_buf = gtk4::TextBuffer::new(None);

    // Hidden until a command actually produces output — an always-visible
    // empty log box below a short package list was the single biggest
    // "dead space" offender in the app.
    let log_view = TextView::with_buffer(&log_buf);
    log_view.set_editable(false);
    log_view.set_monospace(true);
    log_view.set_height_request(140);
    log_view.set_margin_top(8);
    log_view.set_visible(false);

    populate_packages(&list, &log_buf, &log_view);

    let scroll = ScrolledWindow::new();
    scroll.set_vexpand(true);
    scroll.set_child(Some(&list));
    content.append(&scroll);

    let btn_row = GBox::new(Orientation::Horizontal, 8);
    btn_row.set_margin_top(12);

    // Labeled "List installed", not "Check for updates" — bakery list is a
    // listing of installed packages, it doesn't check for available updates.
    let check_btn = Button::with_label("List installed");
    let update_all_btn = Button::with_label("Update all");

    {
        let log_buf = log_buf.clone();
        let log_view = log_view.clone();
        check_btn.connect_clicked(move |_| {
            log_buf.set_text("");
            log_view.set_visible(true);
            stream_command(&["bakery", "list"], log_buf.clone());
        });
    }

    {
        let log_buf = log_buf.clone();
        let log_view = log_view.clone();
        update_all_btn.connect_clicked(move |_| {
            log_buf.set_text("");
            log_view.set_visible(true);
            stream_command(&["bakery", "update", "--all"], log_buf.clone());
        });
    }

    btn_row.append(&check_btn);
    btn_row.append(&update_all_btn);
    content.append(&btn_row);

    // ---------------------------------------------------------------------
    // System packages (pacman) — the other update channel. bakery only
    // covers the userspace bread apps; base system/kernel/bos-settings/AUR
    // republished packages come from pacman + the [breadway] repo, and
    // bos-update (the CLI) already updates both — this panel previously
    // only exposed the bakery half, so a user relying on it alone would
    // never get base-system updates through the GUI.
    // ---------------------------------------------------------------------
    content.append(&w::section("System packages (pacman)"));
    content.append(&w::hint(
        "Base system, kernel, bos-settings, and republished AUR packages — \
         the other half of what `bos-update` covers. Needs your password \
         (polkit) since pacman requires root.",
    ));

    let pacman_btn_row = GBox::new(Orientation::Horizontal, 8);
    pacman_btn_row.set_margin_top(8);
    let pacman_update_btn = Button::with_label("Update system (pacman -Syu)");
    {
        let log_buf = log_buf.clone();
        let log_view = log_view.clone();
        pacman_update_btn.connect_clicked(move |_| {
            log_buf.set_text("");
            log_view.set_visible(true);
            stream_command(&["pkexec", "pacman", "-Syu", "--noconfirm"], log_buf.clone());
        });
    }
    pacman_btn_row.append(&pacman_update_btn);
    content.append(&pacman_btn_row);

    content.append(&log_view);

    outer
}
