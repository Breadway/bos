//! breadclip — clipboard history popup + its background daemon (breadclipd).
//! No config file to edit: breadclip takes no persistent settings. This
//! panel exists purely so the daemon backing it is visible/controllable
//! from bos-settings instead of only from a terminal — previously breadclip
//! had no presence in Settings at all despite running its own service.

use gtk4::prelude::*;
use gtk4::{Box as GBox, Button};

use crate::ui::widgets as w;

pub fn build() -> GBox {
    let (outer, c) = w::view_scaffold("Clipboard");

    c.append(&w::hint(
        "Keeps a history of copied text/images and shows it as a popup. \
         breadclipd (below) is the background daemon that watches the \
         clipboard — breadclip itself is just the popup UI, launched on \
         demand by the keybind or the button below.",
    ));

    let open_btn = Button::with_label("Open history (SUPER+V)");
    open_btn.add_css_class("suggested-action");
    open_btn.set_halign(gtk4::Align::Start);
    open_btn.connect_clicked(|_| {
        let _ = std::process::Command::new("breadclip").spawn();
    });
    c.append(&open_btn);

    c.append(&w::service_control("breadclipd.service", false, false));

    outer
}
