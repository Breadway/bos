use gtk4::prelude::*;
use gtk4::{Box as GBox, Button};
use std::process::Command;

use crate::ui::widgets as w;

fn get_monitors() -> Vec<(String, String)> {
    let Ok(output) = Command::new("hyprctl").args(["monitors", "-j"]).output() else {
        return Vec::new();
    };
    let text = String::from_utf8_lossy(&output.stdout);
    let Ok(monitors) = serde_json::from_str::<Vec<serde_json::Value>>(&text) else {
        return Vec::new();
    };
    monitors
        .iter()
        .filter_map(|m| {
            let name = m.get("name")?.as_str()?;
            let w = m.get("width")?.as_u64()?;
            let h = m.get("height")?.as_u64()?;
            let refresh = m.get("refreshRate")?.as_f64()?;
            Some((name.to_string(), format!("{w}x{h} @ {refresh:.0}Hz")))
        })
        .collect()
}

fn hypr_path(name: &str) -> std::path::PathBuf {
    crate::config::config_dir().join("hypr").join(name)
}

/// Open `path` in $EDITOR (nano if unset) inside a terminal window. Spawning
/// an editor directly (no terminal) is a silent no-op for any TUI editor —
/// there's nothing for it to attach to — so it always needs a terminal
/// wrapper. Uses kitty, which is what BOS actually ships (not foot).
fn open_in_terminal(path: &std::path::Path) {
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "nano".to_string());
    if let Ok(mut child) = Command::new("kitty").args(["-e", &editor]).arg(path).spawn() {
        std::thread::spawn(move || { let _ = child.wait(); });
    }
}

pub fn build() -> GBox {
    let (outer, content) = w::view_scaffold("Display");

    content.append(&w::section("Connected monitors"));
    let monitors = get_monitors();
    if monitors.is_empty() {
        content.append(&w::hint("No monitors detected (is Hyprland running?)"));
    } else {
        for (name, mode) in &monitors {
            content.append(&w::info_row(name, mode));
        }
    }

    content.append(&w::section("Configuration"));
    content.append(&w::hint(
        "Monitor layout, keyboard/input, and workspace rules are configured \
         directly in hyprland.lua — there's no live editor for them here yet.",
    ));

    // BOS's Hyprland config is Lua-native (hyprland.lua), not the classic
    // hyprland.conf/keybinds.conf pair — those names only ever matched a
    // stale, unshipped dotfiles/ directory, so this button opened (or
    // silently created) the wrong file entirely.
    let open_btn = Button::with_label("Open hyprland.lua in editor");
    open_btn.set_halign(gtk4::Align::Start);
    {
        let conf_path = hypr_path("hyprland.lua");
        open_btn.connect_clicked(move |_| open_in_terminal(&conf_path));
    }
    content.append(&open_btn);

    // Keybinds are defined inline in hyprland.lua (no separate file); point
    // this at the shipped cheat sheet instead of a keybinds.conf that has
    // never existed on BOS.
    let keybinds_btn = Button::with_label("View keybinds cheat sheet");
    keybinds_btn.set_halign(gtk4::Align::Start);
    {
        let kb_path = std::path::PathBuf::from("/usr/share/bos/keybinds.txt");
        keybinds_btn.connect_clicked(move |_| open_in_terminal(&kb_path));
    }
    content.append(&keybinds_btn);

    outer
}
