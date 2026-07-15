//! The floating explanation bubble: title/body (rendered through the same
//! constrained markdown subset guides use, so `[Show Keybind]`/`[Run]` tags
//! work here too), dot progress, Back/Next. Positioned just below (or, if
//! that would run off the bottom of the screen, above) the spotlighted
//! target; a `Target::None` step gets no anchors at all, which wlr-layer-
//! shell/gtk4-layer-shell centers on the monitor by convention.

use gtk4::prelude::*;
use gtk4::{Box as GBox, Button, Label, Orientation, Window};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

use crate::content::keybinds::Keybind;
use crate::content::markdown;
use crate::content::tour::Step;

use super::target::Rect;

pub struct Callout {
    pub window: Window,
    pub back_btn: Button,
    pub next_btn: Button,
    pub skip_btn: Button,
    pub demo_btn: Option<Button>,
}

const CALLOUT_WIDTH: i32 = 360;
/// Rough height estimate used only to decide "does this fit below the
/// target" before the widget has actually been laid out — doesn't need to
/// be exact, just enough to avoid running off the bottom of the screen.
const ESTIMATED_HEIGHT: i32 = 220;

#[allow(clippy::too_many_arguments)]
pub fn build(
    monitor: &gdk4::Monitor,
    rect: Option<Rect>,
    monitor_size: (i32, i32),
    step: &Step,
    binds: &[Keybind],
    step_index: usize,
    total: usize,
    is_last: bool,
    confirmed: bool,
    show_demo: bool,
) -> Callout {
    let window = Window::new();
    window.set_decorated(false);
    window.add_css_class("tour-window");
    window.init_layer_shell();
    window.set_layer(Layer::Overlay);
    window.set_keyboard_mode(KeyboardMode::OnDemand);
    window.set_monitor(Some(monitor));

    if let Some(rect) = rect {
        let (mon_w, mon_h) = monitor_size;
        window.set_anchor(Edge::Left, true);
        window.set_anchor(Edge::Top, true);
        let margin_left = (rect.x).clamp(20, (mon_w - CALLOUT_WIDTH - 20).max(20));
        let below = rect.y + rect.h + 12;
        let margin_top = if below + ESTIMATED_HEIGHT <= mon_h {
            below
        } else {
            (rect.y - 12 - ESTIMATED_HEIGHT).max(20)
        };
        window.set_margin(Edge::Left, margin_left);
        window.set_margin(Edge::Top, margin_top);
    }
    // No anchors for a `Target::None` step — left to the compositor, which
    // centers unanchored layer surfaces.

    let root = GBox::new(Orientation::Vertical, 12);
    root.add_css_class("tour-callout");
    root.set_size_request(CALLOUT_WIDTH, -1);

    let title = Label::new(Some(&step.title));
    title.add_css_class("title");
    title.set_xalign(0.0);
    title.set_wrap(true);
    root.append(&title);

    let body = markdown::render(&markdown::parse(&step.body), binds);
    root.append(&body);

    if confirmed {
        let done = Label::new(Some("\u{2713} Nice work — click Next to continue."));
        done.add_css_class("tour-confirmed");
        done.set_xalign(0.0);
        root.append(&done);
    }

    let demo_btn = if show_demo {
        let hint_row = GBox::new(Orientation::Horizontal, 8);
        hint_row.add_css_class("tour-hint-row");
        let hint_label = Label::new(Some("Still there?"));
        hint_label.add_css_class("dim-label");
        hint_label.set_hexpand(true);
        hint_label.set_xalign(0.0);
        let btn = Button::with_label("Show me");
        hint_row.append(&hint_label);
        hint_row.append(&btn);
        root.append(&hint_row);
        Some(btn)
    } else {
        None
    };

    // Text only, not a dot row too — a separate row of dots next to this
    // said the identical thing twice with no added legibility.
    let counter = Label::new(Some(&format!("Step {} of {total}", step_index + 1)));
    counter.add_css_class("dim-label");
    counter.set_xalign(0.0);
    root.append(&counter);

    let nav = GBox::new(Orientation::Horizontal, 8);
    let skip_btn = Button::with_label("Skip tour");
    skip_btn.add_css_class("flat");
    let back_btn = Button::with_label("Back");
    back_btn.set_sensitive(step_index > 0);
    let spacer = GBox::new(Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    let next_btn = Button::with_label(if is_last { "Finish" } else { "Next" });
    next_btn.add_css_class("suggested-action");
    nav.append(&skip_btn);
    nav.append(&spacer);
    nav.append(&back_btn);
    nav.append(&next_btn);
    root.append(&nav);

    window.set_child(Some(&root));
    window.present();
    if confirmed {
        next_btn.grab_focus();
    }

    Callout { window, back_btn, next_btn, skip_btn, demo_btn }
}
