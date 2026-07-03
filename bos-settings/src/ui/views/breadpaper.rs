//! breadpaper — wallpaper manager. No config file to edit here; breadpaper
//! takes no persistent settings, just an image path via its CLI (`breadpaper
//! set <path>` / `breadpaper get`). This panel is a thin GUI front-end for
//! that CLI so wallpaper (and the pywal-driven theme it generates) has a
//! discoverable home in BOS Settings instead of only being reachable from a
//! terminal.

use std::path::PathBuf;
use std::process::Command;

use gtk4::prelude::*;
use gtk4::{
    Box as GBox, Button, FileChooserAction, FileChooserDialog, Image, Label, Orientation,
    ResponseType,
};

use crate::ui::widgets as w;

fn current_wallpaper() -> Option<PathBuf> {
    let out = Command::new("breadpaper").arg("get").output().ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(PathBuf::from(s))
    }
}

fn refresh_preview(preview: &Image, path_lbl: &Label) {
    match current_wallpaper() {
        Some(path) => {
            let filename = path.file_name().map(|f| f.to_string_lossy().to_string());
            path_lbl.set_text(filename.as_deref().unwrap_or("(unknown filename)"));
            path_lbl.set_tooltip_text(Some(&path.display().to_string()));
            preview.set_from_file(Some(&path));
        }
        None => {
            path_lbl.set_text("No wallpaper set");
            path_lbl.set_tooltip_text(None);
            preview.set_icon_name(Some("image-missing"));
        }
    }
}

pub fn build() -> GBox {
    let (outer, c) = w::view_scaffold("Wallpaper");

    c.append(&w::hint(
        "Sets the desktop wallpaper, generates a matching pywal palette, and \
         reloads the shared bread-theme stylesheet — the wallpaper drives \
         the whole desktop's accent colors.",
    ));

    let preview_card = GBox::new(Orientation::Vertical, 8);
    preview_card.add_css_class("card");
    preview_card.set_halign(gtk4::Align::Center);
    preview_card.set_margin_top(8);
    preview_card.set_margin_bottom(8);

    let preview = Image::new();
    preview.set_pixel_size(320);
    preview_card.append(&preview);

    let path_lbl = Label::new(None);
    path_lbl.set_wrap(true);
    path_lbl.add_css_class("dim-label");
    preview_card.append(&path_lbl);
    c.append(&preview_card);

    refresh_preview(&preview, &path_lbl);

    let btn_row = GBox::new(Orientation::Horizontal, 8);
    btn_row.set_margin_top(8);
    btn_row.set_halign(gtk4::Align::Center);

    let choose_btn = Button::with_label("Choose image...");
    choose_btn.add_css_class("suggested-action");
    let status = Label::new(None);
    status.add_css_class("dim-label");

    {
        let preview = preview.clone();
        let path_lbl = path_lbl.clone();
        let status = status.clone();
        choose_btn.connect_clicked(move |btn| {
            let window = btn.root().and_then(|r| r.downcast::<gtk4::Window>().ok());
            let dialog = FileChooserDialog::new(
                Some("Choose a wallpaper"),
                window.as_ref(),
                FileChooserAction::Open,
                &[("Cancel", ResponseType::Cancel), ("Set", ResponseType::Accept)],
            );
            // Restricted to what breadpaper's own validate() actually
            // accepts (png/jpg/jpeg/webp/gif/bmp) — add_pixbuf_formats()
            // also offers svg/tiff/etc. that breadpaper rejects outright.
            let filter = gtk4::FileFilter::new();
            for ext in ["png", "jpg", "jpeg", "webp", "gif", "bmp"] {
                filter.add_suffix(ext);
            }
            filter.set_name(Some("Images"));
            dialog.add_filter(&filter);

            let preview = preview.clone();
            let path_lbl = path_lbl.clone();
            let status = status.clone();
            dialog.connect_response(move |dialog, response| {
                if response == ResponseType::Accept {
                    if let Some(file) = dialog.file() {
                        if let Some(path) = file.path() {
                            // breadpaper set runs `awww img` + pywal palette
                            // generation, which is routinely 1-3s (pywal
                            // spawns Python + an ImageMagick backend) — not
                            // the "sub-second" call this used to assume.
                            // GTK widgets aren't Send, so run it in a thread
                            // and hand the result back over a channel
                            // (same pattern as snapshots.rs).
                            status.set_text("Setting...");
                            let (tx, rx) = async_channel::bounded::<bool>(1);
                            std::thread::spawn(move || {
                                let ok = Command::new("breadpaper")
                                    .arg("set")
                                    .arg(&path)
                                    .status()
                                    .map(|s| s.success())
                                    .unwrap_or(false);
                                let _ = tx.send_blocking(ok);
                            });

                            let preview = preview.clone();
                            let path_lbl = path_lbl.clone();
                            let status = status.clone();
                            glib::spawn_future_local(async move {
                                let ok = rx.recv().await.unwrap_or(false);
                                if ok {
                                    refresh_preview(&preview, &path_lbl);
                                    status.set_text("Wallpaper set");
                                } else {
                                    status.set_text("breadpaper failed — see terminal/journal");
                                }
                                let lbl = status.clone();
                                glib::timeout_add_seconds_local(3, move || {
                                    lbl.set_text("");
                                    glib::ControlFlow::Break
                                });
                            });
                        }
                    }
                }
                dialog.close();
            });
            dialog.show();
        });
    }

    btn_row.append(&choose_btn);
    btn_row.append(&status);
    c.append(&btn_row);

    outer
}
