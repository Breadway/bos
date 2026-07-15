//! Resolves a `content::tour::Target` to a screen rect + the `gdk4::Monitor`
//! it's on, via `services::hyprland`'s `hyprctl -j` snapshots. `hyprctl -j
//! clients` reports a window's monitor as a numeric id, while GDK/Wayland
//! only exposes a monitor's connector name — `hyprctl -j monitors` is the
//! join between the two (`Client::monitor == Monitor::id`, then match
//! `Monitor::name` against `gdk4::Monitor::connector()`).

use gdk4::prelude::*;

use crate::content::tour::Target;
use crate::services::hyprland;

#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

pub struct Resolved {
    pub rect: Rect,
    pub monitor: gdk4::Monitor,
}

/// `None` means the target isn't on screen (app not launched yet, or a
/// pure-concept `Target::None` step with no spotlight at all).
pub fn resolve(target: &Target, display: &gdk4::Display) -> Option<Resolved> {
    let (rect, monitor_name) = match target {
        Target::None => return None,
        Target::Namespace(ns) => {
            let layer = hyprland::layers().into_iter().find(|l| &l.namespace == ns)?;
            (Rect { x: layer.x, y: layer.y, w: layer.w, h: layer.h }, layer.monitor)
        }
        Target::WindowClass(class) => {
            let client = hyprland::clients().into_iter().find(|c| c.class.eq_ignore_ascii_case(class))?;
            let name = hyprland::monitors().into_iter().find(|m| m.id == client.monitor)?.name;
            (Rect { x: client.at.0, y: client.at.1, w: client.size.0, h: client.size.1 }, name)
        }
    };
    let monitor = find_gdk_monitor(display, &monitor_name)?;
    Some(Resolved { rect, monitor })
}

fn find_gdk_monitor(display: &gdk4::Display, name: &str) -> Option<gdk4::Monitor> {
    display.monitors().iter::<gdk4::Monitor>().filter_map(|m| m.ok()).find(|m| m.connector().as_deref() == Some(name))
}

/// Falls back to this when a step has no target (nothing to resolve a
/// monitor from) or a `launch`ed target never appeared — the tour should
/// still show up on whichever output the user is actually looking at.
pub fn focused_monitor(display: &gdk4::Display) -> Option<gdk4::Monitor> {
    let name = hyprland::monitors().into_iter().find(|m| m.focused)?.name;
    find_gdk_monitor(display, &name)
}

/// Whether `target` is currently resolvable on screen — checked before
/// firing a step's `launch` command, since breadbox/breadclip/breadsearch
/// toggle-close on a second invocation of the same command; unconditionally
/// launching would close an already-open instance instead of opening one.
pub fn is_resolved(target: &Target) -> bool {
    match target {
        Target::None => true,
        Target::Namespace(ns) => hyprland::layers().iter().any(|l| &l.namespace == ns),
        Target::WindowClass(class) => hyprland::clients().iter().any(|c| c.class.eq_ignore_ascii_case(class)),
    }
}
