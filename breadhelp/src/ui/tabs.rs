use gtk4::prelude::*;
use gtk4::{Align, Stack, StackSwitcher};

/// Home/Learn/Ask tab switcher. Only 3 pages, so GTK's built-in
/// `StackSwitcher` is a better fit here than the grouped-`ListBox` sidebar
/// bos-settings uses for its ~19 settings pages.
pub fn build(stack: &Stack) -> StackSwitcher {
    let switcher = StackSwitcher::new();
    switcher.set_stack(Some(stack));
    switcher.set_halign(Align::Center);
    switcher.set_margin_top(8);
    switcher.set_margin_bottom(8);
    switcher
}
