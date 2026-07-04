use gtk4::prelude::*;
use gtk4::{
    AlertDialog, Box as GBox, Button, Label, ListBox, ListBoxRow, Orientation, ScrolledWindow,
};
use std::process::Command;

use crate::ui::widgets as w;

#[derive(Clone)]
struct SnapshotRow {
    number: String,
    date: String,
    description: String,
}

fn list_snapshots() -> Vec<SnapshotRow> {
    // NOTE: the real flag is --columns, not --output-cols (which snapper
    // rejects outright with "Unknown option") — confirmed against snapper
    // 0.13's own --help. With the wrong flag this always failed and the
    // panel silently showed "No snapshots found" on every install.
    let Ok(output) = Command::new("snapper")
        .args(["list", "--columns", "number,date,description"])
        .output()
    else {
        return Vec::new();
    };
    if !output.status.success() {
        eprintln!(
            "bos-settings: snapper list failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
        return Vec::new();
    }

    let text = String::from_utf8_lossy(&output.stdout);
    text.lines()
        .skip(2) // header + separator
        .filter_map(|line| {
            let mut cols = line.splitn(3, '|');
            let number = cols.next()?.trim().to_string();
            // Snapshot 0 ("current") always exists, can't be rolled back to
            // or deleted, and isn't a real snapshot — filter it out.
            if number == "0" {
                return None;
            }
            Some(SnapshotRow {
                number,
                date:        cols.next()?.trim().to_string(),
                description: cols.next()?.trim().to_string(),
            })
        })
        .collect()
}

/// Returns whether the list ended up empty, so callers can disable the
/// selection-dependent buttons instead of leaving them clickable no-ops.
fn populate_list(list: &ListBox) -> bool {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
    let snapshots = list_snapshots();
    if snapshots.is_empty() {
        let row = ListBoxRow::new();
        row.set_selectable(false);
        row.set_child(Some(&w::empty_state(
            "document-open-recent-symbolic",
            "No snapshots yet",
            "Snapshots are created automatically on every pacman transaction \
             (snapper may not be configured yet).",
        )));
        list.append(&row);
        return true;
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
    false
}

pub fn build() -> GBox {
    let (outer, content) = w::view_scaffold("Snapshots");
    content.append(&w::hint(
        "System snapshots created by snap-pac on each pacman transaction. \
         Boot into one from the GRUB menu to recover; delete old ones here.",
    ));

    let list = ListBox::new();
    list.set_selection_mode(gtk4::SelectionMode::Single);
    let empty = populate_list(&list);

    let scroll = ScrolledWindow::new();
    scroll.set_vexpand(true);
    scroll.set_child(Some(&list));
    content.append(&scroll);

    let btn_row = GBox::new(Orientation::Horizontal, 8);
    btn_row.set_margin_top(12);

    let refresh_btn = Button::with_label("Refresh");
    let rollback_btn = Button::with_label("Boot into selected...");
    let delete_btn = Button::with_label("Delete selected");
    delete_btn.add_css_class("destructive-action");
    rollback_btn.set_sensitive(!empty);
    delete_btn.set_sensitive(!empty);

    {
        let list = list.clone();
        let rollback_btn = rollback_btn.clone();
        let delete_btn = delete_btn.clone();
        refresh_btn.connect_clicked(move |_| {
            let empty = populate_list(&list);
            rollback_btn.set_sensitive(!empty);
            delete_btn.set_sensitive(!empty);
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

            // BOS boots with root pinned to a named subvolume (grub emits
            // rootflags=subvol=@), so `snapper rollback`'s usual mechanism —
            // switching the btrfs *default* subvolume — has no effect here;
            // grub never consults it. The real, working way to get back to a
            // snapshot on this layout is grub-btrfs (already installed +
            // running via grub-btrfsd.service): it generates a GRUB submenu
            // entry per snapshot, bootable directly. So this button doesn't
            // touch the filesystem at all — it just points you at that menu.
            let dialog = AlertDialog::builder()
                .message(&format!("Boot into snapshot #{number}?"))
                .detail("Snapshots on BOS are booted directly from the GRUB \
                         menu (under \"BOS snapshots\"), not rolled back in \
                         place. Reboot now and pick this snapshot there, or \
                         later if you'd rather keep working — the menu entry \
                         will still be there.")
                .buttons(["Later", "Reboot now"])
                .cancel_button(0)
                .default_button(0)
                .build();

            dialog.choose(window.as_ref(), gtk4::gio::Cancellable::NONE, move |result| {
                if result == Ok(1) {
                    let _ = Command::new("systemctl").args(["reboot"]).spawn();
                }
            });
        });
    }

    {
        let list = list.clone();
        let rollback_btn = rollback_btn.clone();
        delete_btn.connect_clicked(move |btn| {
            let Some(row) = list.selected_row() else { return };
            let number = row.widget_name().to_string();
            if number.is_empty() { return }

            let rollback_btn = rollback_btn.clone();
            let delete_btn = btn.clone();
            let window = btn
                .root()
                .and_then(|r| r.downcast::<gtk4::Window>().ok());

            let dialog = AlertDialog::builder()
                .message(&format!("Delete snapshot #{number}?"))
                .detail("This cannot be undone.")
                .buttons(["Cancel", "Delete"])
                .cancel_button(0)
                .default_button(0)
                .build();

            let window2 = window.clone();
            let list2 = list.clone();
            let rollback_btn2 = rollback_btn.clone();
            let delete_btn2 = delete_btn.clone();
            dialog.choose(window.as_ref(), gtk4::gio::Cancellable::NONE, move |result| {
                if result != Ok(1) { return }

                // snapper's DBus path authorizes via ALLOW_USERS, not pkexec —
                // this only works because post-install.sh seeds that config
                // key, but if it's ever missing this fails silently unless we
                // check the exit status. GTK widgets aren't Send, so hand the
                // outcome back over a channel rather than touching them from
                // the thread (same pattern as the rollback flow used to).
                let (tx, rx) = async_channel::bounded::<bool>(1);
                std::thread::spawn(move || {
                    let ok = Command::new("snapper")
                        .args(["delete", &number])
                        .status()
                        .map(|s| s.success())
                        .unwrap_or(false);
                    let _ = tx.send_blocking(ok);
                });

                let list = list2.clone();
                let window = window2.clone();
                let rollback_btn = rollback_btn2.clone();
                let delete_btn = delete_btn2.clone();
                glib::spawn_future_local(async move {
                    let ok = rx.recv().await.unwrap_or(false);
                    if ok {
                        let empty = populate_list(&list);
                        rollback_btn.set_sensitive(!empty);
                        delete_btn.set_sensitive(!empty);
                    } else {
                        let err = AlertDialog::builder()
                            .message("Delete failed")
                            .detail("snapper delete exited with an error — the \
                                     snapshot wasn't removed.")
                            .buttons(["OK"])
                            .build();
                        err.choose(window.as_ref(), gtk4::gio::Cancellable::NONE, |_| {});
                    }
                });
            });
        });
    }

    btn_row.append(&refresh_btn);
    btn_row.append(&rollback_btn);
    btn_row.append(&delete_btn);
    outer.append(&btn_row);

    outer
}
