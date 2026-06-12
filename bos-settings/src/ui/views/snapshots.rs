use gtk4::prelude::*;
use gtk4::{
    Box as GBox, Button, Label, ListBox, ListBoxRow, MessageDialog, Orientation, ScrolledWindow,
};
use std::process::Command;

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
            let cols: Vec<&str> = line.splitn(3, '|').collect();
            if cols.len() == 3 {
                Some(SnapshotRow {
                    number: cols[0].trim().to_string(),
                    date: cols[1].trim().to_string(),
                    description: cols[2].trim().to_string(),
                })
            } else {
                None
            }
        })
        .collect()
}

fn confirm_rollback(number: &str) {
    let number = number.to_string();
    let dialog = MessageDialog::new(
        None::<&gtk4::Window>,
        gtk4::DialogFlags::MODAL,
        gtk4::MessageType::Question,
        gtk4::ButtonsType::OkCancel,
        &format!("Roll back to snapshot #{number}?\n\nReboot required to apply."),
    );
    dialog.connect_response(move |d, resp| {
        if resp == gtk4::ResponseType::Ok {
            let _ = Command::new("snapper")
                .args(["rollback", &number])
                .status();
            let info = MessageDialog::new(
                None::<&gtk4::Window>,
                gtk4::DialogFlags::MODAL,
                gtk4::MessageType::Info,
                gtk4::ButtonsType::Ok,
                "Rollback queued. Please reboot to apply.",
            );
            info.connect_response(|d, _| d.destroy());
            info.present();
        }
        d.destroy();
    });
    dialog.present();
}

pub fn build() -> GBox {
    let vbox = GBox::new(Orientation::Vertical, 0);
    vbox.add_css_class("view-content");

    let title = Label::new(Some("Snapshots"));
    title.add_css_class("title");
    title.set_xalign(0.0);
    vbox.append(&title);

    let subtitle =
        Label::new(Some("System snapshots created by snap-pac on each pacman transaction."));
    subtitle.set_xalign(0.0);
    subtitle.set_margin_bottom(16);
    vbox.append(&subtitle);

    let list = ListBox::new();
    list.set_selection_mode(gtk4::SelectionMode::Single);

    let snapshots = list_snapshots();
    if snapshots.is_empty() {
        let row = ListBoxRow::new();
        let lbl = Label::new(Some(
            "No snapshots found (snapper may not be configured yet)",
        ));
        lbl.set_margin_top(8);
        lbl.set_margin_bottom(8);
        lbl.set_margin_start(8);
        row.set_child(Some(&lbl));
        list.append(&row);
    } else {
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

    let scroll = ScrolledWindow::new();
    scroll.set_vexpand(true);
    scroll.set_child(Some(&list));
    vbox.append(&scroll);

    let btn_row = GBox::new(Orientation::Horizontal, 8);
    btn_row.set_margin_top(12);

    let rollback_btn = Button::with_label("Rollback to selected");
    let delete_btn = Button::with_label("Delete selected");
    delete_btn.add_css_class("destructive-action");

    {
        let list = list.clone();
        rollback_btn.connect_clicked(move |_| {
            let Some(row) = list.selected_row() else {
                return;
            };
            let number = row.widget_name().to_string();
            confirm_rollback(&number);
        });
    }

    {
        let list = list.clone();
        delete_btn.connect_clicked(move |_| {
            let Some(row) = list.selected_row() else {
                return;
            };
            let number = row.widget_name().to_string();
            let _ = Command::new("snapper").args(["delete", &number]).status();
        });
    }

    btn_row.append(&rollback_btn);
    btn_row.append(&delete_btn);
    vbox.append(&btn_row);

    vbox
}
