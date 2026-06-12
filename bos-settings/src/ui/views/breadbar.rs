use gtk4::prelude::*;
use gtk4::{Box as GBox, Button, Label, Orientation, ScrolledWindow, TextView};
use std::path::PathBuf;

fn css_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/user".to_string());
    PathBuf::from(home).join(".config/breadbar/style.css")
}

pub fn build() -> GBox {
    let path = css_path();
    let existing_css = std::fs::read_to_string(&path).unwrap_or_default();

    let vbox = GBox::new(Orientation::Vertical, 12);
    vbox.add_css_class("view-content");

    let title = Label::new(Some("breadbar"));
    title.add_css_class("title");
    title.set_xalign(0.0);
    vbox.append(&title);

    let subtitle = Label::new(Some(
        "CSS overrides for breadbar. Leave empty to use the default bread theme.",
    ));
    subtitle.set_xalign(0.0);
    subtitle.set_margin_bottom(8);
    subtitle.set_wrap(true);
    vbox.append(&subtitle);

    let buf = gtk4::TextBuffer::new(None);
    buf.set_text(&existing_css);

    let text_view = TextView::with_buffer(&buf);
    text_view.set_monospace(true);

    let scroll = ScrolledWindow::new();
    scroll.set_vexpand(true);
    scroll.set_child(Some(&text_view));
    vbox.append(&scroll);

    let btn_row = GBox::new(Orientation::Horizontal, 12);
    btn_row.set_margin_top(12);

    let save_btn = Button::with_label("Save");
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
                    status_lbl.set_text("Saved");
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
    vbox.append(&btn_row);

    vbox
}
