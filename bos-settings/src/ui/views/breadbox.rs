//! breadbox config.toml — launcher contexts.
//! Schema mirrors breadbox-shared (`[[contexts]]` with `name` + `priority`, an
//! ordered list of app/category hints). The contexts array is rewritten on
//! save; any other top-level keys/comments in the file are preserved.

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{
    Box as GBox, Button, Entry, Label, ListBox, ListBoxRow, Orientation, ScrolledWindow,
};
use toml_edit::{value, Array, ArrayOfTables, DocumentMut, Item, Table};

use crate::config;

#[derive(Clone, Default)]
struct Context {
    name: String,
    priority: Vec<String>,
}

fn config_path() -> std::path::PathBuf {
    config::config_dir().join("breadbox/config.toml")
}

fn read_contexts(doc: &DocumentMut) -> Vec<Context> {
    let Some(aot) = doc.get("contexts").and_then(Item::as_array_of_tables) else {
        return Vec::new();
    };
    aot.iter()
        .map(|t| Context {
            name: t.get("name").and_then(Item::as_str).unwrap_or("").to_string(),
            priority: t
                .get("priority")
                .and_then(Item::as_array)
                .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default(),
        })
        .collect()
}

/// Rewrite only the `contexts` array-of-tables, leaving the rest of the doc.
fn write_contexts(doc: &mut DocumentMut, ctxs: &[Context]) {
    let mut aot = ArrayOfTables::new();
    for ctx in ctxs {
        let mut t = Table::new();
        t.insert("name", value(&ctx.name));
        let mut arr = Array::new();
        for p in &ctx.priority {
            arr.push(p.as_str());
        }
        t.insert("priority", value(arr));
        aot.push(t);
    }
    doc.as_table_mut().insert("contexts", Item::ArrayOfTables(aot));
}

fn rebuild_list(list: &ListBox, model: &Rc<RefCell<Vec<Context>>>) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
    for (i, ctx) in model.borrow().iter().enumerate() {
        let row = ListBoxRow::new();
        row.set_selectable(false);

        let hbox = GBox::new(Orientation::Horizontal, 8);
        hbox.set_margin_top(6);
        hbox.set_margin_bottom(6);
        hbox.set_margin_start(8);
        hbox.set_margin_end(8);

        let name_entry = Entry::new();
        name_entry.set_text(&ctx.name);
        name_entry.set_width_chars(14);
        name_entry.set_placeholder_text(Some("name"));

        let prio_entry = Entry::new();
        prio_entry.set_text(&ctx.priority.join(", "));
        prio_entry.set_hexpand(true);
        prio_entry.set_placeholder_text(Some("firefox, code, Development, ..."));

        let remove_btn = Button::with_label("Remove");
        remove_btn.add_css_class("destructive-action");

        {
            let model = model.clone();
            name_entry.connect_changed(move |e| {
                if let Some(c) = model.borrow_mut().get_mut(i) {
                    c.name = e.text().to_string();
                }
            });
        }
        {
            let model = model.clone();
            prio_entry.connect_changed(move |e| {
                if let Some(c) = model.borrow_mut().get_mut(i) {
                    c.priority = e
                        .text()
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                }
            });
        }
        {
            let model = model.clone();
            let list = list.clone();
            remove_btn.connect_clicked(move |_| {
                model.borrow_mut().remove(i);
                rebuild_list(&list, &model);
            });
        }

        hbox.append(&name_entry);
        hbox.append(&prio_entry);
        hbox.append(&remove_btn);
        row.set_child(Some(&hbox));
        list.append(&row);
    }
}

pub fn build() -> GBox {
    let path = config_path();
    let doc = Rc::new(RefCell::new(config::load_doc(&path)));
    let model = Rc::new(RefCell::new(read_contexts(&doc.borrow())));

    let vbox = GBox::new(Orientation::Vertical, 12);
    vbox.add_css_class("view-content");

    let title = Label::new(Some("breadbox"));
    title.add_css_class("title");
    title.set_xalign(0.0);
    vbox.append(&title);

    let subtitle = Label::new(Some(
        "Launcher contexts — each lists, in priority order, the apps/categories surfaced first.",
    ));
    subtitle.set_xalign(0.0);
    subtitle.set_wrap(true);
    subtitle.set_margin_bottom(8);
    vbox.append(&subtitle);

    let list = ListBox::new();
    list.set_selection_mode(gtk4::SelectionMode::None);
    rebuild_list(&list, &model);

    let scroll = ScrolledWindow::new();
    scroll.set_vexpand(true);
    scroll.set_child(Some(&list));
    vbox.append(&scroll);

    let btn_row = GBox::new(Orientation::Horizontal, 8);
    btn_row.set_margin_top(8);

    let add_btn = Button::with_label("Add context");
    {
        let model = model.clone();
        let list = list.clone();
        add_btn.connect_clicked(move |_| {
            model.borrow_mut().push(Context {
                name: "new".to_string(),
                priority: Vec::new(),
            });
            rebuild_list(&list, &model);
        });
    }

    let save_btn = Button::with_label("Save");
    save_btn.add_css_class("suggested-action");
    let status_lbl = Label::new(None);
    status_lbl.add_css_class("dim-label");

    {
        let doc = doc.clone();
        let model = model.clone();
        let path = path.clone();
        let status_lbl = status_lbl.clone();
        save_btn.connect_clicked(move |_| {
            write_contexts(&mut doc.borrow_mut(), &model.borrow());
            match config::save_doc(&path, &doc.borrow()) {
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
