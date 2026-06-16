//! Reusable settings rows bound to a shared `toml_edit` document.
//!
//! Every row reads its current value from the document on build and writes the
//! single key it owns back into the document on change. A view collects rows,
//! then a [`save_button`] persists the whole document to disk in one shot — so
//! unmodelled keys and comments are always preserved (see `crate::config`).

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{
    Adjustment, Box as GBox, Button, DropDown, Entry, Expression, Label, Orientation,
    SpinButton, StringList, Switch,
};
use toml_edit::DocumentMut;

use crate::config;

/// Shared, mutable config document handed to every row in a view.
pub type Doc = Rc<RefCell<DocumentMut>>;

/// A fixed key path into the document, e.g. `&["adapters", "power", "enabled"]`.
type Path = &'static [&'static str];

fn field_label(text: &str) -> Label {
    let lbl = Label::new(Some(text));
    lbl.set_hexpand(true);
    lbl.set_xalign(0.0);
    lbl
}

fn row(label: &str, control: &impl IsA<gtk4::Widget>) -> GBox {
    let row = GBox::new(Orientation::Horizontal, 16);
    row.append(&field_label(label));
    control.set_halign(gtk4::Align::End);
    control.set_valign(gtk4::Align::Center);
    row.append(control);
    row
}

/// A bold section heading with spacing above it.
pub fn section(text: &str) -> Label {
    let lbl = Label::new(Some(text));
    lbl.add_css_class("heading");
    lbl.set_xalign(0.0);
    lbl.set_margin_top(12);
    lbl.set_margin_bottom(2);
    lbl
}

/// Small dimmed helper text under a section or row.
pub fn hint(text: &str) -> Label {
    let lbl = Label::new(Some(text));
    lbl.add_css_class("dim-label");
    lbl.set_xalign(0.0);
    lbl.set_wrap(true);
    lbl.set_margin_bottom(4);
    lbl
}

/// Standard view scaffold: an outer vertical box with a title and a scrollable
/// content area. Append setting rows to the returned `content`, then append a
/// [`save_button`] to `outer`. Returns `(outer, content)`.
pub fn view_scaffold(title: &str) -> (GBox, GBox) {
    let outer = GBox::new(Orientation::Vertical, 8);
    outer.add_css_class("view-content");

    let title_lbl = Label::new(Some(title));
    title_lbl.add_css_class("title");
    title_lbl.set_xalign(0.0);
    outer.append(&title_lbl);

    let content = GBox::new(Orientation::Vertical, 8);
    let scroll = gtk4::ScrolledWindow::new();
    scroll.set_vexpand(true);
    scroll.set_hscrollbar_policy(gtk4::PolicyType::Never);
    scroll.set_child(Some(&content));
    outer.append(&scroll);

    (outer, content)
}

pub fn switch_row(label: &str, doc: &Doc, path: Path, default: bool) -> GBox {
    let cur = config::get_bool(&doc.borrow(), path).unwrap_or(default);
    let sw = Switch::new();
    sw.set_active(cur);
    let doc = doc.clone();
    sw.connect_active_notify(move |s| {
        config::set_bool(&mut doc.borrow_mut(), path, s.is_active());
    });
    row(label, &sw)
}

pub fn entry_row(label: &str, doc: &Doc, path: Path, placeholder: &str, default: &str) -> GBox {
    let cur = config::get_str(&doc.borrow(), path).unwrap_or_else(|| default.to_string());
    let entry = Entry::new();
    entry.set_text(&cur);
    entry.set_hexpand(true);
    entry.set_width_chars(28);
    if !placeholder.is_empty() {
        entry.set_placeholder_text(Some(placeholder));
    }
    let doc = doc.clone();
    entry.connect_changed(move |e| {
        config::set_str_or_remove(&mut doc.borrow_mut(), path, e.text().as_str());
    });
    row(label, &entry)
}

pub fn password_row(label: &str, doc: &Doc, path: Path) -> GBox {
    let cur = config::get_str(&doc.borrow(), path).unwrap_or_default();
    let entry = Entry::new();
    entry.set_text(&cur);
    entry.set_visibility(false);
    entry.set_hexpand(true);
    entry.set_width_chars(28);
    entry.set_input_purpose(gtk4::InputPurpose::Password);
    let doc = doc.clone();
    entry.connect_changed(move |e| {
        config::set_str_or_remove(&mut doc.borrow_mut(), path, e.text().as_str());
    });
    row(label, &entry)
}

/// A dropdown that stores the selected option string at `path`.
pub fn dropdown_row(label: &str, doc: &Doc, path: Path, options: &[&str], default: &str) -> GBox {
    let cur = config::get_str(&doc.borrow(), path).unwrap_or_else(|| default.to_string());
    let model = StringList::new(options);
    let dd = DropDown::new(Some(model), Expression::NONE);
    let sel = options.iter().position(|o| *o == cur).unwrap_or(0) as u32;
    dd.set_selected(sel);
    let owned: Vec<String> = options.iter().map(|s| s.to_string()).collect();
    let doc = doc.clone();
    dd.connect_selected_notify(move |dd| {
        if let Some(opt) = owned.get(dd.selected() as usize) {
            config::set_str(&mut doc.borrow_mut(), path, opt);
        }
    });
    row(label, &dd)
}

/// An integer spin button storing its value at `path`.
pub fn spin_row(
    label: &str,
    doc: &Doc,
    path: Path,
    min: f64,
    max: f64,
    step: f64,
    default: i64,
) -> GBox {
    let cur = config::get_i64(&doc.borrow(), path).unwrap_or(default);
    let adj = Adjustment::new(cur as f64, min, max, step, step, 0.0);
    let spin = SpinButton::new(Some(&adj), step, 0);
    let doc = doc.clone();
    spin.connect_value_changed(move |s| {
        config::set_i64(&mut doc.borrow_mut(), path, s.value() as i64);
    });
    row(label, &spin)
}

/// A fractional spin button (e.g. 0.0–1.0 confidence) storing a float.
pub fn spin_f64_row(
    label: &str,
    doc: &Doc,
    path: Path,
    min: f64,
    max: f64,
    step: f64,
    digits: u32,
    default: f64,
) -> GBox {
    let cur = config::get_f64(&doc.borrow(), path).unwrap_or(default);
    let adj = Adjustment::new(cur, min, max, step, step, 0.0);
    let spin = SpinButton::new(Some(&adj), step, digits);
    let doc = doc.clone();
    spin.connect_value_changed(move |s| {
        config::set_f64(&mut doc.borrow_mut(), path, s.value());
    });
    row(label, &spin)
}

/// A comma-separated list editor storing an array of strings at `path`.
pub fn csv_row(label: &str, doc: &Doc, path: Path, placeholder: &str) -> GBox {
    let cur = config::get_str_list(&doc.borrow(), path).join(", ");
    let entry = Entry::new();
    entry.set_text(&cur);
    entry.set_hexpand(true);
    entry.set_width_chars(28);
    if !placeholder.is_empty() {
        entry.set_placeholder_text(Some(placeholder));
    }
    let doc = doc.clone();
    entry.connect_changed(move |e| {
        let items: Vec<String> = e
            .text()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        config::set_str_list(&mut doc.borrow_mut(), path, &items);
    });
    row(label, &entry)
}

/// A Save button + transient status label that persists the document to `path`.
pub fn save_button(doc: &Doc, path: PathBuf) -> GBox {
    let btn_row = GBox::new(Orientation::Horizontal, 12);
    btn_row.set_margin_top(16);

    let save_btn = Button::with_label("Save");
    save_btn.add_css_class("suggested-action");
    let status = Label::new(None);
    status.add_css_class("dim-label");

    let doc = doc.clone();
    let status_c = status.clone();
    save_btn.connect_clicked(move |_| match config::save_doc(&path, &doc.borrow()) {
        Ok(()) => {
            status_c.set_text("Saved");
            let lbl = status_c.clone();
            glib::timeout_add_seconds_local(3, move || {
                lbl.set_text("");
                glib::ControlFlow::Break
            });
        }
        Err(e) => status_c.set_text(&format!("Error: {e}")),
    });

    btn_row.append(&save_btn);
    btn_row.append(&status);
    btn_row
}
