use gtk4::prelude::*;
use gtk4::Widget;

use crate::config::Mode;

const ALL_CLASSES: &[&str] = &["beginner-mode", "dad-mode", "compact-mode"];

/// Swaps the mode CSS class on `root` — the actual layout rules (font-size
/// bumps, larger touch targets for Dad, tighter margins for Compact) live in
/// `theme.rs`'s `APP_CSS`, same "app owns only layout" split as
/// `bos-settings/src/theme.rs`. Applied to the window's root widget so it
/// affects every tab, not just Home.
pub fn apply(root: &impl IsA<Widget>, mode: Mode) {
    let root = root.upcast_ref::<Widget>();
    for class in ALL_CLASSES {
        root.remove_css_class(class);
    }
    let class = match mode {
        Mode::Normal => return,
        Mode::Beginner => "beginner-mode",
        Mode::Dad => "dad-mode",
        Mode::Compact => "compact-mode",
    };
    root.add_css_class(class);
}
