use gtk4::prelude::*;
use gtk4::{
    AlertDialog, Box as GBox, Button, Label, ListBox, ListBoxRow, Orientation, ScrolledWindow,
};
use std::process::Command;

#[derive(Clone)]
struct SnapshotRow {
    number: String,
    date: String,
    description: String,
}

fn list_snapshots() -> Vec<SnapshotRow> {
    let Ok(output) = Command::new("snapper")
        .args(["list", "--output-cols", "number,date,description"])
        .output()
    else {
        return Vec::new();
    };

    let text = String::from_utf8_lossy(&output.stdout);
    text.lines()
        .skip(2) // header + separator
        .filter_map(|line| {
            let mut cols = line.splitn(3, '|');
            Some(SnapshotRow {
                number:      cols.next()?.trim().to_string(),
                date:        cols.next()?.trim().to_string(),
                description: cols.next()?.trim().to_string(),
            })
        })
        .collect()
}

fn populate_list(list: &ListBox) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
    let snapshots = list_snapshots();
    if snapshots.is_empty() {
        let row = ListBoxRow::new();
        row.set_selectable(false);
        let lbl = Label::new(Some("No snapshots found (snapper may not be configured yet)"));
        lbl.set_margin_top(8);
        lbl.set_margin_bottom(8);
        lbl.set_margin_start(8);
        row.set_child(Some(&lbl));
        list.append(&row);
        return;
    }
    for snap in &snapshots {
        let row = ListBoxRow::new();
        row.set_widget_name(&snap.number);

        let hbox = GBox::new(Orientation::Horizontal, 16);
        hbox.set_margin_top(6);
        hbox.set_margin_bottom(6);
        hbox.set_margin_start(8);
        hbox.set_margin_end(8);

        let num_lbl = Label::new(Some(&snap.number));
        num_lbl.set_width_chars(4);
        num_lbl.set_xalign(0.0);

        let date_lbl = Label::new(Some(&snap.date));
        date_lbl.set_width_chars(22);
        date_lbl.set_xalign(0.0);

        let desc_lbl = Label::new(Some(&snap.description));
        desc_lbl.set_hexpand(true);
        desc_lbl.set_xalign(0.0);

        hbox.append(&num_lbl);
        hbox.append(&date_lbl);
        hbox.append(&desc_lbl);
        row.set_child(Some(&hbox));
        list.append(&row);
    }
}

pub fn build() -> GBox {
    let vbox = GBox::new(Orientation::Vertical, 0);
    vbox.add_css_class("view-content");

    let title = Label::new(Some("Snapshots"));
    title.add_css_class("title");
    title.set_xalign(0.0);
    vbox.append(&title);

    let subtitle = Label::new(Some(
        "System snapshots created by snap-pac on each pacman transaction.",
    ));
    subtitle.set_xalign(0.0);
    subtitle.set_margin_bottom(16);
    vbox.append(&subtitle);

    let list = ListBox::new();
    list.set_selection_mode(gtk4::SelectionMode::Single);
    populate_list(&list);

    let scroll = ScrolledWindow::new();
    scroll.set_vexpand(true);
    scroll.set_child(Some(&list));
    vbox.append(&scroll);

    let btn_row = GBox::new(Orientation::Horizontal, 8);
    btn_row.set_margin_top(12);

    let refresh_btn = Button::with_label("Refresh");
    let rollback_btn = Button::with_label("Rollback to selected");
    let delete_btn = Button::with_label("Delete selected");
    delete_btn.add_css_class("destructive-action");

    {
        let list = list.clone();
        refresh_btn.connect_clicked(move |_| {
            populate_list(&list);
        });
    }

    {
        let list = list.clone();
        rollback_btn.connect_clicked(move |btn| {
            let Some(row) = list.selected_row() else { return };
            let number = row.widget_name().to_string();
            if number.is_empty() { return }

            let window = btn
                .root()
                .and_then(|r| r.downcast::<gtk4::Window>().ok());

            let dialog = AlertDialog::builder()
                .message(&format!("Roll back to snapshot #{number}?"))
                .detail("The current system state will be replaced on next boot. \
                         A polkit prompt will ask for your password.")
                .buttons(["Cancel", "Roll back"])
                .cancel_button(0)
                .default_button(0)
                .build();

            dialog.choose(window.as_ref(), gtk4::gio::Cancellable::NONE, move |result| {
                if result == Ok(1) {
                    // pkexec so polkit handles the privilege escalation
                    std::thread::spawn(move || {
                        let _ = Command::new("pkexec")
                            .args(["snapper", "rollback", &number])
                            .status();
                    });
                }
            });
        });
    }

    {
        let list = list.clone();
        delete_btn.connect_clicked(move |btn| {
            let Some(row) = list.selected_row() else { return };
            let number = row.widget_name().to_string();
            if number.is_empty() { return }

            let window = btn
                .root()
                .and_then(|r| r.downcast::<gtk4::Window>().ok());

            let list = list.clone();
            let dialog = AlertDialog::builder()
                .message(&format!("Delete snapshot #{number}?"))
                .detail("This cannot be undone.")
                .buttons(["Cancel", "Delete"])
                .cancel_button(0)
                .default_button(0)
                .build();

            dialog.choose(window.as_ref(), gtk4::gio::Cancellable::NONE, move |result| {
                if result == Ok(1) {
                    let _ = Command::new("snapper").args(["delete", &number]).status();
                    populate_list(&list);
                }
            });
        });
    }

    btn_row.append(&refresh_btn);
    btn_row.append(&rollback_btn);
    btn_row.append(&delete_btn);
    vbox.append(&btn_row);

    vbox
}
