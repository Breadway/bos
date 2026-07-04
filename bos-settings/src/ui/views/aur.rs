//! AUR search via yay — graphical discovery for the wider AUR beyond
//! bakery's bread ecosystem and [breadway]'s own republished packages.
//!
//! Search and browsing are fully graphical; actually installing a package
//! opens a terminal running `yay -S <pkg>` instead of a silent `--noconfirm`
//! install. That's deliberate, not a shortcut we didn't get around to:
//! AUR packages run arbitrary maintainer-supplied build scripts, and yay's
//! interactive PKGBUILD diff review (plus the sudo password prompt) is the
//! actual safety mechanism against a malicious/compromised AUR package —
//! automating it away in the name of "no terminal" would remove the one
//! step that exists to catch that.

use gtk4::prelude::*;
use gtk4::{Box as GBox, Button, Entry, Label, ListBox, ListBoxRow, Orientation, ScrolledWindow};
use std::process::Command;

use crate::ui::widgets as w;

#[derive(Clone)]
struct AurResult {
    name: String,
    version: String,
    description: String,
}

fn search(query: &str) -> Vec<AurResult> {
    let Ok(output) = Command::new("yay").args(["-Ss", "--aur", query]).output() else {
        return Vec::new();
    };
    let text = String::from_utf8_lossy(&output.stdout);
    let mut results = Vec::new();
    let mut lines = text.lines().peekable();
    while let Some(header) = lines.next() {
        // "aur/name version (+votes score) [Orphaned]" — name/version are
        // always the first two whitespace-separated fields after the repo/.
        let Some(rest) = header.strip_prefix("aur/") else { continue };
        let mut parts = rest.split_whitespace();
        let Some(name) = parts.next() else { continue };
        let version = parts.next().unwrap_or("").to_string();
        let description = lines.next().unwrap_or("").trim().to_string();
        results.push(AurResult { name: name.to_string(), version, description });
    }
    results
}

fn install_in_terminal(pkg: &str) {
    let _ = Command::new("kitty").args(["-e", "yay", "-S", pkg]).spawn();
}

pub fn build() -> GBox {
    let (outer, content) = w::view_scaffold("AUR");
    content.append(&w::hint(
        "Search the Arch User Repository via yay. Installing opens a \
         terminal — AUR packages run arbitrary build scripts, and reviewing \
         what yay is about to do (and entering your password) is a real \
         safety step, not just a formality.",
    ));

    let search_row = GBox::new(Orientation::Horizontal, 8);
    let search_entry = Entry::new();
    search_entry.set_hexpand(true);
    search_entry.set_placeholder_text(Some("Search the AUR…"));
    let search_btn = Button::with_label("Search");
    search_btn.add_css_class("suggested-action");
    search_row.append(&search_entry);
    search_row.append(&search_btn);
    content.append(&search_row);

    let status = w::hint("Search for a package to see results here.");
    content.append(&status);

    let list = ListBox::new();
    list.set_selection_mode(gtk4::SelectionMode::None);
    let scroll = ScrolledWindow::new();
    scroll.set_vexpand(true);
    scroll.set_min_content_height(320);
    scroll.set_child(Some(&list));
    content.append(&scroll);

    let run_search = {
        let list = list.clone();
        let status = status.clone();
        let search_entry = search_entry.clone();
        move |btn: Option<Button>| {
            let query = search_entry.text().to_string();
            if query.trim().is_empty() {
                return;
            }
            if let Some(b) = &btn {
                b.set_sensitive(false);
            }
            status.set_text("Searching…");

            let (tx, rx) = async_channel::bounded::<Vec<AurResult>>(1);
            std::thread::spawn(move || {
                let _ = tx.send_blocking(search(&query));
            });

            let list = list.clone();
            let status = status.clone();
            let btn = btn.clone();
            glib::spawn_future_local(async move {
                if let Ok(results) = rx.recv().await {
                    while let Some(child) = list.first_child() {
                        list.remove(&child);
                    }
                    if results.is_empty() {
                        let row = ListBoxRow::new();
                        row.set_selectable(false);
                        row.set_child(Some(&w::empty_state(
                            "system-search-symbolic",
                            "No results",
                            "Try a different search term.",
                        )));
                        list.append(&row);
                        status.set_text("No results.");
                    } else {
                        status.set_text(&format!("{} result(s)", results.len()));
                        for r in results.into_iter().take(50) {
                            let row = ListBoxRow::new();
                            row.set_selectable(false);
                            let vbox = GBox::new(Orientation::Vertical, 2);
                            vbox.add_css_class("card");
                            vbox.set_margin_top(3);
                            vbox.set_margin_bottom(3);

                            let top = GBox::new(Orientation::Horizontal, 12);
                            let name_lbl = Label::new(Some(&format!("{}  {}", r.name, r.version)));
                            name_lbl.set_hexpand(true);
                            name_lbl.set_xalign(0.0);
                            let install_btn = Button::with_label("Install");
                            {
                                let pkg = r.name.clone();
                                install_btn.connect_clicked(move |_| install_in_terminal(&pkg));
                            }
                            top.append(&name_lbl);
                            top.append(&install_btn);
                            vbox.append(&top);

                            let desc_lbl = Label::new(Some(&r.description));
                            desc_lbl.add_css_class("dim-label");
                            desc_lbl.set_xalign(0.0);
                            desc_lbl.set_wrap(true);
                            desc_lbl.set_max_width_chars(64);
                            vbox.append(&desc_lbl);

                            row.set_child(Some(&vbox));
                            list.append(&row);
                        }
                    }
                }
                if let Some(b) = &btn {
                    b.set_sensitive(true);
                }
            });
        }
    };

    {
        let run_search = run_search.clone();
        search_btn.connect_clicked(move |btn| run_search(Some(btn.clone())));
    }
    {
        let run_search = run_search.clone();
        search_entry.connect_activate(move |_| run_search(None));
    }

    outer
}
