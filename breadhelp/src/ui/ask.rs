use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{Box as GBox, Entry, Label, ListBox, ListBoxRow, Orientation, Paned, ScrolledWindow, SelectionMode, Widget};

use crate::config::Mode;
use crate::content::{keybinds::Keybind, ContentStore};

use super::{guide_view, keybind_viewer};

struct CommandEntry {
    name: &'static str,
    description: &'static str,
}

/// A small, curated command gallery — not every `bread*` binary, just the
/// ones worth surfacing to someone searching for "what can I run".
const COMMANDS: &[CommandEntry] = &[
    CommandEntry { name: "bos-update", description: "Update both the system (pacman) and the bread ecosystem (bakery) in one go." },
    CommandEntry { name: "bos-settings", description: "Open BOS Settings." },
    CommandEntry { name: "breadbox", description: "Open the app launcher." },
    CommandEntry { name: "breadpad", description: "Open notes/reminders." },
    CommandEntry { name: "breadman", description: "Open the notes/task manager." },
    CommandEntry { name: "breadpaper", description: "Open the wallpaper picker." },
    CommandEntry { name: "breadclip", description: "Open clipboard history." },
    CommandEntry { name: "breadcrumbs", description: "Manage Wi-Fi profiles." },
];

enum Kind {
    Guide { category: String, id: String },
    Keybind(Keybind),
    Command { name: &'static str, description: &'static str },
}

struct IndexEntry {
    label: String,
    haystack: String,
    kind: Kind,
}

fn build_index(store: &ContentStore, binds: &[Keybind]) -> Vec<IndexEntry> {
    let mut index = Vec::new();
    for guide in store.all() {
        let haystack = format!("{} {}", guide.meta.title, guide.meta.tags.join(" ")).to_lowercase();
        index.push(IndexEntry {
            label: guide.meta.title.clone(),
            haystack,
            kind: Kind::Guide { category: guide.category.clone(), id: guide.id.clone() },
        });
    }
    for bind in binds {
        let label = format!("{} — {}", bind.combo, bind.action);
        let haystack = label.to_lowercase();
        index.push(IndexEntry { label, haystack, kind: Kind::Keybind(bind.clone()) });
    }
    for cmd in COMMANDS {
        let haystack = format!("{} {}", cmd.name, cmd.description).to_lowercase();
        index.push(IndexEntry {
            label: cmd.name.to_string(),
            haystack,
            kind: Kind::Command { name: cmd.name, description: cmd.description },
        });
    }
    index
}

pub fn build(store: Rc<ContentStore>, binds: Rc<Vec<Keybind>>, mode: Mode) -> Widget {
    let hpaned = Paned::new(Orientation::Horizontal);
    hpaned.set_position(320);
    hpaned.set_shrink_start_child(false);
    hpaned.set_resize_start_child(false);

    let left = GBox::new(Orientation::Vertical, 8);
    left.add_css_class("view-content");

    let search = Entry::new();
    search.set_placeholder_text(Some("Search guides, keybinds, commands\u{2026}"));
    left.append(&search);

    let results_list = ListBox::new();
    results_list.set_selection_mode(SelectionMode::None);
    results_list.set_activate_on_single_click(true);
    results_list.add_css_class("boxed-list");
    let results_scroller = ScrolledWindow::builder().child(&results_list).vexpand(true).build();
    left.append(&results_scroller);

    let detail = GBox::new(Orientation::Vertical, 10);
    detail.add_css_class("view-content");
    let detail_scroller = ScrolledWindow::builder().child(&detail).vexpand(true).hexpand(true).build();

    let index = Rc::new(build_index(&store, &binds));

    let populate: Rc<dyn Fn(&str)> = {
        let results_list = results_list.clone();
        let index = index.clone();
        Rc::new(move |query: &str| {
            while let Some(child) = results_list.first_child() {
                results_list.remove(&child);
            }
            let q = query.to_lowercase();
            let matches = index
                .iter()
                .enumerate()
                .filter(|(_, e)| q.is_empty() || e.haystack.contains(&q))
                .take(40);
            for (i, entry) in matches {
                let row = ListBoxRow::new();
                row.set_widget_name(&i.to_string());
                let lbl = Label::new(Some(&entry.label));
                lbl.set_xalign(0.0);
                lbl.set_wrap(true);
                lbl.set_margin_top(6);
                lbl.set_margin_bottom(6);
                lbl.set_margin_start(8);
                lbl.set_margin_end(8);
                row.set_child(Some(&lbl));
                results_list.append(&row);
            }
        })
    };
    populate("");

    search.connect_changed({
        let populate = populate.clone();
        move |entry| populate(&entry.text())
    });

    results_list.connect_row_activated(move |_, row| {
        let Ok(i) = row.widget_name().parse::<usize>() else { return };
        let Some(entry) = index.get(i) else { return };

        while let Some(child) = detail.first_child() {
            detail.remove(&child);
        }

        match &entry.kind {
            Kind::Guide { category, id } => {
                if let Some(guide) = store.guide(category, id) {
                    detail.append(&guide_view::build(&store, guide, mode.is_simplified(), &binds));
                }
            }
            Kind::Keybind(bind) => {
                let list = ListBox::new();
                list.set_selection_mode(SelectionMode::None);
                list.add_css_class("boxed-list");
                list.append(&keybind_viewer::row(bind));
                detail.append(&list);
            }
            Kind::Command { name, description } => {
                let title = Label::new(Some(name));
                title.add_css_class("title");
                title.set_xalign(0.0);
                detail.append(&title);

                let desc = Label::new(Some(description));
                desc.set_wrap(true);
                desc.set_xalign(0.0);
                detail.append(&desc);

                let run_btn = gtk4::Button::with_label("Run");
                run_btn.set_halign(gtk4::Align::Start);
                let cmd = name.to_string();
                run_btn.connect_clicked(move |_| crate::services::exec::run(&cmd));
                detail.append(&run_btn);
            }
        }
    });

    hpaned.set_start_child(Some(&left));
    hpaned.set_end_child(Some(&detail_scroller));

    hpaned.upcast()
}
