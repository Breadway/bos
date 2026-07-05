use gtk4::prelude::*;
use gtk4::{Box as GBox, Label, Orientation, ScrolledWindow, Widget};

use crate::content::{keybinds::Keybind, markdown, ContentStore, Guide};

pub fn build(store: &ContentStore, guide: &Guide, simplified: bool, binds: &[Keybind]) -> Widget {
    let vbox = GBox::new(Orientation::Vertical, 12);
    vbox.add_css_class("view-content");

    let title = Label::new(Some(&guide.meta.title));
    title.add_css_class("title");
    title.set_xalign(0.0);
    vbox.append(&title);

    let body = store.body(guide, simplified);
    let blocks = markdown::parse(&body);
    vbox.append(&markdown::render(&blocks, binds));

    ScrolledWindow::builder().child(&vbox).vexpand(true).hexpand(true).build().upcast()
}
