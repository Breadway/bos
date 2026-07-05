use gtk4::prelude::*;
use gtk4::{Box as GBox, Button, Entry, Label, ListBox, ListBoxRow, MenuButton, Orientation, Popover, ScrolledWindow, SelectionMode};

use crate::content::keybinds::{self, Keybind};

pub fn build(binds: &[Keybind]) -> gtk4::Widget {
    let vbox = GBox::new(Orientation::Vertical, 6);

    let header = Label::new(Some("Keybinds"));
    header.add_css_class("title");
    header.set_xalign(0.0);
    vbox.append(&header);

    let list = ListBox::new();
    list.set_selection_mode(SelectionMode::None);
    list.add_css_class("boxed-list");

    if binds.is_empty() {
        let empty = Label::new(Some("No binds.json found at ~/.config/hypr/binds.json"));
        empty.add_css_class("dim-label");
        vbox.append(&empty);
        return vbox.upcast();
    }

    for bind in binds {
        list.append(&row(bind));
    }

    let scroller = ScrolledWindow::builder().min_content_height(320).vexpand(true).child(&list).build();
    vbox.append(&scroller);
    vbox.upcast()
}

pub fn row(bind: &Keybind) -> ListBoxRow {
    let row = ListBoxRow::new();
    row.set_selectable(false);

    let hbox = GBox::new(Orientation::Horizontal, 12);
    hbox.set_margin_top(6);
    hbox.set_margin_bottom(6);
    hbox.set_margin_start(8);
    hbox.set_margin_end(8);

    let combo = Label::new(Some(&bind.combo));
    combo.add_css_class("keybind-chip");
    combo.set_width_chars(20);
    combo.set_xalign(0.0);
    hbox.append(&combo);

    let action = Label::new(Some(&bind.action));
    action.set_hexpand(true);
    action.set_xalign(0.0);
    action.set_wrap(true);
    hbox.append(&action);

    if let Some(demo) = &bind.demo_cmd {
        let try_btn = Button::with_label("Try it");
        let cmd = demo.clone();
        try_btn.connect_clicked(move |_| crate::services::exec::run(&cmd));
        hbox.append(&try_btn);
    }

    if let Some(raw) = bind.raw.clone() {
        let rebind_btn = MenuButton::builder().icon_name("document-edit-symbolic").build();
        rebind_btn.set_tooltip_text(Some("Temporary rebind"));
        // Icon-only button — screen readers have nothing else to announce.
        rebind_btn.update_property(&[gtk4::accessible::Property::Label("Temporary rebind")]);
        rebind_btn.set_popover(Some(&rebind_popover(&raw)));
        hbox.append(&rebind_btn);
    }

    row.set_child(Some(&hbox));
    row
}

fn rebind_popover(raw: &str) -> Popover {
    let popover = Popover::new();
    let vbox = GBox::new(Orientation::Vertical, 8);
    vbox.set_margin_top(10);
    vbox.set_margin_bottom(10);
    vbox.set_margin_start(10);
    vbox.set_margin_end(10);

    let hint = Label::new(Some("Temporary — resets on reload/login.\nEdit hyprland.lua for a permanent change."));
    hint.add_css_class("dim-label");
    hint.set_wrap(true);
    vbox.append(&hint);

    let entry = Entry::new();
    entry.set_placeholder_text(Some("e.g. super + shift + u"));
    vbox.append(&entry);

    let apply = Button::with_label("Apply temporarily");
    apply.add_css_class("suggested-action");
    let raw = raw.to_string();
    let popover_weak = popover.downgrade();
    apply.connect_clicked(move |_| {
        let combo = entry.text();
        if combo.trim().is_empty() {
            return;
        }
        let mods_key = keybinds::to_hypr_mods_key(&combo);
        if mods_key.is_empty() {
            return;
        }
        crate::services::hyprland::rebind_temp(&format!("{mods_key},{raw}"));
        if let Some(p) = popover_weak.upgrade() {
            p.popdown();
        }
    });
    vbox.append(&apply);

    popover.set_child(Some(&vbox));
    popover
}
