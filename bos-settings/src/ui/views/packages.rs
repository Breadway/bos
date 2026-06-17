use async_channel;
use gtk4::prelude::*;
use gtk4::{
    Box as GBox, Button, Label, ListBox, ListBoxRow, Orientation, ScrolledWindow, TextView,
};
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

fn read_installed() -> HashMap<String, String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    let path = std::path::Path::new(&home)
        .join(".local/state/bakery/installed.json");

    let Ok(text) = std::fs::read_to_string(&path) else {
        return HashMap::new();
    };
    let Ok(parsed) = serde_json::from_str::<HashMap<String, serde_json::Value>>(&text) else {
        return HashMap::new();
    };

    parsed
        .into_iter()
        .filter_map(|(name, val)| {
            let version = val
                .get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            Some((name, version))
        })
        .collect()
}

fn stream_command(args: &[&str], log_buf: gtk4::TextBuffer) {
    let (sender, receiver) = async_channel::bounded::<String>(256);
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();

    std::thread::spawn(move || {
        let mut child = match Command::new(&args[0])
            .args(&args[1..])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                let _ = sender.send_blocking(format!("Error: {e}"));
                return;
            }
        };

        // Merge stderr into the channel too.
        // Both are Some because we spawned with Stdio::piped() above.
        let stdout = child.stdout.take().expect("stdout piped");
        let stderr = child.stderr.take().expect("stderr piped");

        let tx2 = sender.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines().flatten() {
                let _ = tx2.send_blocking(line);
            }
        });

        for line in BufReader::new(stdout).lines().flatten() {
            let _ = sender.send_blocking(line);
        }
        let _ = child.wait();
    });

    glib::spawn_future_local(async move {
        while let Ok(line) = receiver.recv().await {
            let mut end = log_buf.end_iter();
            log_buf.insert(&mut end, &format!("{line}\n"));
        }
    });
}

pub fn build() -> GBox {
    let vbox = GBox::new(Orientation::Vertical, 0);
    vbox.add_css_class("view-content");

    let title = Label::new(Some("Packages"));
    title.add_css_class("title");
    title.set_xalign(0.0);
    vbox.append(&title);

    let subtitle = Label::new(Some("Bread ecosystem packages installed via bakery."));
    subtitle.set_xalign(0.0);
    subtitle.set_margin_bottom(16);
    vbox.append(&subtitle);

    let list = ListBox::new();
    list.set_selection_mode(gtk4::SelectionMode::None);

    let packages = read_installed();
    if packages.is_empty() {
        let row = ListBoxRow::new();
        row.set_selectable(false);
        let lbl = Label::new(Some(
            "No bakery packages found (~/.local/state/bakery/installed.json)",
        ));
        lbl.set_margin_top(8);
        lbl.set_margin_bottom(8);
        lbl.set_margin_start(8);
        row.set_child(Some(&lbl));
        list.append(&row);
    } else {
        let mut names: Vec<_> = packages.iter().collect();
        names.sort_by_key(|(k, _)| k.as_str());

        for (name, version) in names {
            let row = ListBoxRow::new();
            row.set_selectable(false);
            let hbox = GBox::new(Orientation::Horizontal, 16);
            hbox.set_margin_top(6);
            hbox.set_margin_bottom(6);
            hbox.set_margin_start(8);
            hbox.set_margin_end(8);

            let name_lbl = Label::new(Some(name));
            name_lbl.set_hexpand(true);
            name_lbl.set_xalign(0.0);

            let ver_lbl = Label::new(Some(version));
            ver_lbl.set_xalign(1.0);

            // Spawn a thread to reap the child process — no zombies
            let pkg_name = name.clone();
            let update_btn = Button::with_label("Update");
            update_btn.connect_clicked(move |_| {
                match Command::new("bakery").args(["update", &pkg_name]).spawn() {
                    Ok(mut child) => {
                        std::thread::spawn(move || { let _ = child.wait(); });
                    }
                    Err(_) => {}  // bakery not found; button is a no-op
                }
            });

            hbox.append(&name_lbl);
            hbox.append(&ver_lbl);
            hbox.append(&update_btn);
            row.set_child(Some(&hbox));
            list.append(&row);
        }
    }

    let scroll = ScrolledWindow::new();
    scroll.set_vexpand(true);
    scroll.set_child(Some(&list));
    vbox.append(&scroll);

    let log_buf = gtk4::TextBuffer::new(None);
    let log_view = TextView::with_buffer(&log_buf);
    log_view.set_editable(false);
    log_view.set_monospace(true);
    log_view.set_height_request(140);
    log_view.set_margin_top(8);

    let btn_row = GBox::new(Orientation::Horizontal, 8);
    btn_row.set_margin_top(12);

    let check_btn = Button::with_label("Check for updates");
    let update_all_btn = Button::with_label("Update all");

    {
        let log_buf = log_buf.clone();
        check_btn.connect_clicked(move |_| {
            log_buf.set_text("");
            stream_command(&["bakery", "list"], log_buf.clone());
        });
    }

    {
        let log_buf = log_buf.clone();
        update_all_btn.connect_clicked(move |_| {
            log_buf.set_text("");
            stream_command(&["bakery", "update", "--all"], log_buf.clone());
        });
    }

    btn_row.append(&check_btn);
    btn_row.append(&update_all_btn);
    vbox.append(&btn_row);
    vbox.append(&log_view);

    vbox
}
