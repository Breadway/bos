use gtk4::prelude::*;
use gtk4::{Box as GBox, Button, Label, Orientation, ScrolledWindow, TextView};
use std::path::PathBuf;

use crate::ui::widgets as w;

fn css_path() -> PathBuf {
    crate::config::config_dir().join("breadbar/style.css")
}

pub fn build() -> GBox {
    let path = css_path();
    let existing_css = std::fs::read_to_string(&path).unwrap_or_default();

    let (outer, content) = w::view_scaffold("Bar");
    content.append(&w::hint(
        "CSS overrides for breadbar. Leave empty to use the default bread theme. \
         Reloads live on save — no need to restart the bar.",
    ));

    let buf = gtk4::TextBuffer::new(None);
    buf.set_text(&existing_css);

    let text_view = TextView::with_buffer(&buf);
    text_view.set_monospace(true);
    text_view.set_top_margin(12);
    text_view.set_bottom_margin(12);
    text_view.set_left_margin(12);
    text_view.set_right_margin(12);

    // A borderless full-bleed textview reads as a rendering bug, not an
    // editor — give it the same card framing every other panel's content
    // gets, and cap its height so it doesn't compete with the button row
    // for the one screen's worth of space.
    let editor_card = GBox::new(Orientation::Vertical, 0);
    editor_card.add_css_class("card");
    editor_card.set_vexpand(true);

    let scroll = ScrolledWindow::new();
    scroll.set_vexpand(true);
    scroll.set_min_content_height(360);
    scroll.set_child(Some(&text_view));
    editor_card.append(&scroll);
    content.append(&editor_card);

    let btn_row = GBox::new(Orientation::Horizontal, 12);
    btn_row.set_margin_top(12);

    let save_btn = Button::with_label("Save");
    save_btn.add_css_class("suggested-action");
    let status_lbl = Label::new(None);
    status_lbl.add_css_class("dim-label");

    {
        let path = path.clone();
        let status_lbl = status_lbl.clone();
        save_btn.connect_clicked(move |_| {
            let (start, end) = buf.bounds();
            let text = buf.text(&start, &end, false);
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            match std::fs::write(&path, text.as_str()) {
                Ok(()) => {
                    // breadbar has no systemd unit (it's launched directly by
                    // hyprland.lua's exec-once) — SIGHUP is its own documented
                    // live-reload mechanism (see this file's own header
                    // comment: "reload live: kill -HUP $(pidof breadbar)").
                    // -x pkill exits non-zero if breadbar isn't running,
                    // which is fine — the file's still saved either way.
                    let reloaded = std::process::Command::new("pkill")
                        .args(["-HUP", "-x", "breadbar"])
                        .status()
                        .map(|s| s.success())
                        .unwrap_or(false);
                    status_lbl.set_text(if reloaded { "Saved & reloaded" } else { "Saved" });
                    let lbl = status_lbl.clone();
                    glib::timeout_add_seconds_local(3, move || {
                        lbl.set_text("");
                        glib::ControlFlow::Break
                    });
                }
                Err(e) => status_lbl.set_text(&format!("Error: {e}")),
            }
        });
    }

    btn_row.append(&save_btn);
    btn_row.append(&status_lbl);
    outer.append(&btn_row);

    outer
}
