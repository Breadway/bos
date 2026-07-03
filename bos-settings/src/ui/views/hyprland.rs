use gtk4::prelude::*;
use gtk4::{Box as GBox, Button, Label, Orientation};
use std::process::Command;

fn get_monitors() -> Vec<String> {
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
            Some(format!("{name}  {w}x{h} @ {refresh:.0}Hz"))
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
    let vbox = GBox::new(Orientation::Vertical, 12);
    vbox.add_css_class("view-content");

    let title = Label::new(Some("Hyprland"));
    title.add_css_class("title");
    title.set_xalign(0.0);
    vbox.append(&title);

    let monitors_lbl = Label::new(Some("Connected monitors"));
    monitors_lbl.set_xalign(0.0);
    monitors_lbl.set_margin_top(8);
    monitors_lbl.set_margin_bottom(4);
    vbox.append(&monitors_lbl);

    let monitors = get_monitors();
    if monitors.is_empty() {
        let lbl = Label::new(Some("No monitors detected (is Hyprland running?)"));
        lbl.set_xalign(0.0);
        vbox.append(&lbl);
    } else {
        for mon in &monitors {
            let lbl = Label::new(Some(mon));
            lbl.set_xalign(0.0);
            lbl.add_css_class("monospace");
            vbox.append(&lbl);
        }
    }

    // BOS's Hyprland config is Lua-native (hyprland.lua), not the classic
    // hyprland.conf/keybinds.conf pair — those names only ever matched a
    // stale, unshipped dotfiles/ directory, so this button opened (or
    // silently created) the wrong file entirely.
    let open_btn = Button::with_label("Open hyprland.lua in editor");
    open_btn.set_margin_top(16);
    open_btn.set_halign(gtk4::Align::Start);
    {
        let conf_path = hypr_path("hyprland.lua");
        open_btn.connect_clicked(move |_| open_in_terminal(&conf_path));
    }
    vbox.append(&open_btn);

    // Keybinds are defined inline in hyprland.lua (no separate file); point
    // this at the shipped cheat sheet instead of a keybinds.conf that has
    // never existed on BOS.
    let keybinds_btn = Button::with_label("View keybinds cheat sheet");
    keybinds_btn.set_margin_top(8);
    keybinds_btn.set_halign(gtk4::Align::Start);
    {
        let kb_path = std::path::PathBuf::from("/usr/share/bos/keybinds.txt");
        keybinds_btn.connect_clicked(move |_| open_in_terminal(&kb_path));
    }
    vbox.append(&keybinds_btn);

    vbox
}
