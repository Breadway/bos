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
            Some(format!("{name}  {w}×{h} @ {refresh:.0}Hz"))
        })
        .collect()
}

fn config_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    std::path::PathBuf::from(home).join(".config/hypr/hyprland.conf")
}

pub fn build() -> GBox {
    let vbox = GBox::new(Orientation::Vertical, 12);
    vbox.add_css_class("view-content");

    let title = Label::new(Some("Hyprland"));
    title.add_css_class("title");
    title.set_xalign(0.0);
    vbox.append(&title);

    // Monitors section
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

    // Open config button
    let open_btn = Button::with_label("Open hyprland.conf in editor");
    open_btn.set_margin_top(16);
    open_btn.set_halign(gtk4::Align::Start);
    {
        let path = config_path();
        open_btn.connect_clicked(move |_| {
            let editor = std::env::var("EDITOR").unwrap_or_else(|_| "foot".to_string());
            let _ = Command::new(&editor)
                .arg(path.to_str().unwrap_or(""))
                .spawn();
        });
    }
    vbox.append(&open_btn);

    // Open keybinds button
    let keybinds_btn = Button::with_label("Open keybinds.conf in editor");
    keybinds_btn.set_margin_top(8);
    keybinds_btn.set_halign(gtk4::Align::Start);
    {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
        let kb_path = std::path::PathBuf::from(home).join(".config/hypr/keybinds.conf");
        keybinds_btn.connect_clicked(move |_| {
            let editor = std::env::var("EDITOR").unwrap_or_else(|_| "foot".to_string());
            let _ = Command::new(&editor)
                .arg(kb_path.to_str().unwrap_or(""))
                .spawn();
        });
    }
    vbox.append(&keybinds_btn);

    vbox
}
