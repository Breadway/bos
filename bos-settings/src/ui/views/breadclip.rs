//! breadclip — clipboard history popup + its background daemon (breadclipd).
//! No config file to edit: breadclip takes no persistent settings. This
//! panel exists purely so the daemon backing it is visible/controllable
//! from bos-settings instead of only from a terminal — previously breadclip
//! had no presence in Settings at all despite running its own service.

use gtk4::prelude::*;
use gtk4::Box as GBox;

use crate::ui::widgets as w;

pub fn build() -> GBox {
    let (outer, c) = w::view_scaffold("Clipboard");

    c.append(&w::hint(
        "breadclip keeps a history of copied text/images and shows it as a \
         popup (SUPER+V or SUPER+SHIFT+V). breadclipd is the background \
         daemon that actually watches the clipboard — breadclip itself is \
         just the popup UI, launched on demand.",
    ));
    c.append(&w::service_control("breadclipd.service"));

    outer
}
