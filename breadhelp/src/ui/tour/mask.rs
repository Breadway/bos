//! The spotlight: up to 4 `Layer::Overlay` strip windows (top/bottom/left/
//! right) sized to leave a target's bounding box as a real gap with no
//! surface over it at all — gtk4-layer-shell has no click-through/input-
//! region API (confirmed in its own docs), so the only way to let pointer
//! input reach the real UI underneath is to literally not cover it with
//! anything. A `Target::None` step instead gets one full-screen dim window,
//! since there's nothing to leave a gap around.

use gtk4::prelude::*;
use gtk4::{Box as GBox, Window};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

use super::target::Rect;

fn new_mask_window(monitor: &gdk4::Monitor) -> Window {
    let window = Window::new();
    window.set_decorated(false);
    window.add_css_class("tour-window");
    window.init_layer_shell();
    window.set_layer(Layer::Overlay);
    window.set_keyboard_mode(KeyboardMode::None);
    window.set_monitor(Some(monitor));
    let dim = GBox::new(gtk4::Orientation::Vertical, 0);
    dim.add_css_class("tour-mask");
    dim.set_hexpand(true);
    dim.set_vexpand(true);
    window.set_child(Some(&dim));
    window
}

/// One dim window covering the whole monitor — used for `Target::None` steps.
pub fn full_screen(monitor: &gdk4::Monitor) -> Vec<Window> {
    let window = new_mask_window(monitor);
    for edge in [Edge::Top, Edge::Bottom, Edge::Left, Edge::Right] {
        window.set_anchor(edge, true);
    }
    window.present();
    vec![window]
}

/// The 4-strip letterbox around `rect`, in monitor-local coordinates.
/// `monitor_size` is `(width, height)` from `gdk4::Monitor::geometry()`.
pub fn around(monitor: &gdk4::Monitor, rect: Rect, monitor_size: (i32, i32)) -> Vec<Window> {
    let (mon_w, mon_h) = monitor_size;
    let mut windows = Vec::new();

    let top_h = rect.y;
    if top_h > 0 {
        let w = new_mask_window(monitor);
        w.set_anchor(Edge::Top, true);
        w.set_anchor(Edge::Left, true);
        w.set_anchor(Edge::Right, true);
        w.set_default_size(-1, top_h);
        w.present();
        windows.push(w);
    }

    let bottom_h = mon_h - (rect.y + rect.h);
    if bottom_h > 0 {
        let w = new_mask_window(monitor);
        w.set_anchor(Edge::Bottom, true);
        w.set_anchor(Edge::Left, true);
        w.set_anchor(Edge::Right, true);
        w.set_default_size(-1, bottom_h);
        w.present();
        windows.push(w);
    }

    let left_w = rect.x;
    if left_w > 0 {
        let w = new_mask_window(monitor);
        w.set_anchor(Edge::Left, true);
        w.set_anchor(Edge::Top, true);
        w.set_margin(Edge::Top, rect.y);
        w.set_default_size(left_w, rect.h);
        w.present();
        windows.push(w);
    }

    let right_w = mon_w - (rect.x + rect.w);
    if right_w > 0 {
        let w = new_mask_window(monitor);
        w.set_anchor(Edge::Right, true);
        w.set_anchor(Edge::Top, true);
        w.set_margin(Edge::Top, rect.y);
        w.set_default_size(right_w, rect.h);
        w.present();
        windows.push(w);
    }

    windows
}
