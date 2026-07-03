//! Reusable settings rows bound to a shared `toml_edit` document.
//!
//! Every row reads its current value from the document on build and writes the
//! single key it owns back into the document on change. A view collects rows,
//! then a [`save_button`] persists the whole document to disk in one shot — so
//! unmodelled keys and comments are always preserved (see `crate::config`).

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{
    Adjustment, Box as GBox, Button, DropDown, Entry, Expression, Label, Orientation,
    SpinButton, StringList, Switch,
};
use toml_edit::DocumentMut;

use crate::config;

/// Shared, mutable config document handed to every row in a view.
pub type Doc = Rc<RefCell<DocumentMut>>;

/// A fixed key path into the document, e.g. `&["adapters", "power", "enabled"]`.
type Path = &'static [&'static str];

fn field_label(text: &str) -> Label {
    let lbl = Label::new(Some(text));
    lbl.set_hexpand(true);
    lbl.set_xalign(0.0);
    lbl
}

/// A label + control row, the same layout every `*_row` helper below uses.
/// Styled as its own small card (background + padding + rounded corners) so
/// a page of settings reads as a list of distinct rows, not a flat column of
/// labels with the control floating far away at the window's edge — which is
/// what plain edge-aligned rows look like once [`view_scaffold`] stops
/// letting the content column stretch to the full window width.
/// Exposed so panels that front live system state (not a `Doc`) — About,
/// Sound, Date & Time, Network — can still lay out rows consistently with
/// the TOML-editor panels.
pub fn row(label: &str, control: &impl IsA<gtk4::Widget>) -> GBox {
    let row = GBox::new(Orientation::Horizontal, 16);
    row.add_css_class("card");
    row.set_margin_top(3);
    row.set_margin_bottom(3);
    row.append(&field_label(label));
    control.set_halign(gtk4::Align::End);
    control.set_valign(gtk4::Align::Center);
    row.append(control);
    row
}

/// A label + read-only value label, for panels that only display state
/// (About's CPU/memory/disk readouts, etc).
pub fn info_row(label: &str, value: &str) -> GBox {
    let value_lbl = Label::new(Some(value));
    value_lbl.add_css_class("dim-label");
    value_lbl.set_selectable(true);
    row(label, &value_lbl)
}

/// A bold section heading with spacing above it.
pub fn section(text: &str) -> Label {
    let lbl = Label::new(Some(text));
    lbl.add_css_class("heading");
    lbl.set_xalign(0.0);
    lbl.set_margin_top(12);
    lbl.set_margin_bottom(2);
    lbl
}

/// Small dimmed helper text under a section or row.
pub fn hint(text: &str) -> Label {
    let lbl = Label::new(Some(text));
    lbl.add_css_class("dim-label");
    lbl.set_xalign(0.0);
    lbl.set_wrap(true);
    lbl.set_margin_bottom(4);
    lbl
}

/// Widest a settings column is allowed to grow. Without a cap, `content`
/// inherits hexpand from its rows' hexpand-ing labels and the ScrolledWindow
/// stretches it to the full window width — on a maximized/ultrawide window
/// that leaves the control on every row stranded ~1500px from its label.
const CONTENT_MAX_WIDTH: i32 = 760;

/// A centered placeholder for a panel's empty state (no snapshots yet, no
/// scan results yet, etc) — a dim icon + title + hint, instead of a single
/// small sentence lost in an otherwise-empty scroll area.
pub fn empty_state(icon_name: &str, title: &str, hint_text: &str) -> GBox {
    let wrapper = GBox::new(Orientation::Vertical, 6);
    wrapper.set_valign(gtk4::Align::Center);
    wrapper.set_halign(gtk4::Align::Center);
    wrapper.set_vexpand(true);
    wrapper.set_margin_top(32);
    wrapper.set_margin_bottom(32);

    let icon = gtk4::Image::from_icon_name(icon_name);
    icon.set_pixel_size(48);
    icon.add_css_class("dim-label");
    wrapper.append(&icon);

    let title_lbl = Label::new(Some(title));
    title_lbl.add_css_class("heading");
    wrapper.append(&title_lbl);

    let hint_lbl = Label::new(Some(hint_text));
    hint_lbl.add_css_class("dim-label");
    hint_lbl.set_justify(gtk4::Justification::Center);
    hint_lbl.set_wrap(true);
    wrapper.append(&hint_lbl);

    wrapper
}

/// Standard view scaffold: an outer vertical box with a title and a scrollable
/// content area. Append setting rows to the returned `content`, then append a
/// [`save_button`] to `outer`. Returns `(outer, content)`.
pub fn view_scaffold(title: &str) -> (GBox, GBox) {
    let outer = GBox::new(Orientation::Vertical, 8);
    outer.add_css_class("view-content");

    let title_lbl = Label::new(Some(title));
    title_lbl.add_css_class("title");
    title_lbl.set_xalign(0.0);
    outer.append(&title_lbl);

    let content = GBox::new(Orientation::Vertical, 4);
    // Explicit hexpand(false) overrides the auto-computed value GTK would
    // otherwise derive from the hexpand-ing labels inside every row, and
    // Start keeps content pinned under the left-aligned title instead of
    // stretching (Fill) or floating in the middle of a wide pane (Center).
    content.set_hexpand(false);
    // Center rather than left-align: on a wide/maximized window, a
    // left-hugging column just moves the dead space from "right of every
    // row" to "right of the whole column" — centering distributes it evenly
    // either side instead, which reads as deliberate rather than stranded.
    content.set_halign(gtk4::Align::Center);
    content.set_size_request(CONTENT_MAX_WIDTH, -1);

    let scroll = gtk4::ScrolledWindow::new();
    scroll.set_vexpand(true);
    scroll.set_hscrollbar_policy(gtk4::PolicyType::Never);
    scroll.set_child(Some(&content));
    outer.append(&scroll);

    (outer, content)
}

pub fn switch_row(label: &str, doc: &Doc, path: Path, default: bool) -> GBox {
    let cur = config::get_bool(&doc.borrow(), path).unwrap_or(default);
    let sw = Switch::new();
    sw.set_active(cur);
    let doc = doc.clone();
    sw.connect_active_notify(move |s| {
        config::set_bool(&mut doc.borrow_mut(), path, s.is_active());
    });
    row(label, &sw)
}

pub fn entry_row(label: &str, doc: &Doc, path: Path, placeholder: &str, default: &str) -> GBox {
    let cur = config::get_str(&doc.borrow(), path).unwrap_or_else(|| default.to_string());
    let entry = Entry::new();
    entry.set_text(&cur);
    entry.set_hexpand(true);
    entry.set_width_chars(28);
    if !placeholder.is_empty() {
        entry.set_placeholder_text(Some(placeholder));
    }
    let doc = doc.clone();
    entry.connect_changed(move |e| {
        config::set_str_or_remove(&mut doc.borrow_mut(), path, e.text().as_str());
    });
    row(label, &entry)
}

pub fn password_row(label: &str, doc: &Doc, path: Path) -> GBox {
    let cur = config::get_str(&doc.borrow(), path).unwrap_or_default();
    let entry = Entry::new();
    entry.set_text(&cur);
    entry.set_visibility(false);
    entry.set_hexpand(true);
    entry.set_width_chars(28);
    entry.set_input_purpose(gtk4::InputPurpose::Password);
    let doc = doc.clone();
    entry.connect_changed(move |e| {
        config::set_str_or_remove(&mut doc.borrow_mut(), path, e.text().as_str());
    });
    row(label, &entry)
}

/// A dropdown that stores the selected option string at `path`.
pub fn dropdown_row(label: &str, doc: &Doc, path: Path, options: &[&str], default: &str) -> GBox {
    let cur = config::get_str(&doc.borrow(), path).unwrap_or_else(|| default.to_string());
    let model = StringList::new(options);
    let dd = DropDown::new(Some(model), Expression::NONE);
    let sel = options.iter().position(|o| *o == cur).unwrap_or(0) as u32;
    dd.set_selected(sel);
    let owned: Vec<String> = options.iter().map(|s| s.to_string()).collect();
    let doc = doc.clone();
    dd.connect_selected_notify(move |dd| {
        if let Some(opt) = owned.get(dd.selected() as usize) {
            config::set_str(&mut doc.borrow_mut(), path, opt);
        }
    });
    row(label, &dd)
}

/// An integer spin button storing its value at `path`.
pub fn spin_row(
    label: &str,
    doc: &Doc,
    path: Path,
    min: f64,
    max: f64,
    step: f64,
    default: i64,
) -> GBox {
    let cur = config::get_i64(&doc.borrow(), path).unwrap_or(default);
    let adj = Adjustment::new(cur as f64, min, max, step, step, 0.0);
    let spin = SpinButton::new(Some(&adj), step, 0);
    let doc = doc.clone();
    spin.connect_value_changed(move |s| {
        config::set_i64(&mut doc.borrow_mut(), path, s.value() as i64);
    });
    row(label, &spin)
}

/// A fractional spin button (e.g. 0.0–1.0 confidence) storing a float.
pub fn spin_f64_row(
    label: &str,
    doc: &Doc,
    path: Path,
    min: f64,
    max: f64,
    step: f64,
    digits: u32,
    default: f64,
) -> GBox {
    let cur = config::get_f64(&doc.borrow(), path).unwrap_or(default);
    let adj = Adjustment::new(cur, min, max, step, step, 0.0);
    let spin = SpinButton::new(Some(&adj), step, digits);
    let doc = doc.clone();
    spin.connect_value_changed(move |s| {
        config::set_f64(&mut doc.borrow_mut(), path, s.value());
    });
    row(label, &spin)
}

/// A comma-separated list editor storing an array of strings at `path`.
pub fn csv_row(label: &str, doc: &Doc, path: Path, placeholder: &str) -> GBox {
    let cur = config::get_str_list(&doc.borrow(), path).join(", ");
    let entry = Entry::new();
    entry.set_text(&cur);
    entry.set_hexpand(true);
    entry.set_width_chars(28);
    if !placeholder.is_empty() {
        entry.set_placeholder_text(Some(placeholder));
    }
    let doc = doc.clone();
    entry.connect_changed(move |e| {
        let items: Vec<String> = e
            .text()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        config::set_str_list(&mut doc.borrow_mut(), path, &items);
    });
    row(label, &entry)
}

/// Run `args` as a subprocess, streaming stdout+stderr into `log_buf` line by
/// line, then call `on_done` once the process exits. GTK widgets aren't
/// `Send`, so the child runs on its own thread and results come back over an
/// `async_channel` to a `glib::spawn_future_local` task that touches the
/// widget. Shared by every panel that shells out to a CLI tool and wants live
/// output (Packages, Network, Firewall, Firmware, ...).
pub fn stream_command_then(
    args: &[&str],
    log_buf: gtk4::TextBuffer,
    on_done: impl FnOnce() + 'static,
) {
    let (sender, receiver) = async_channel::bounded::<String>(256);
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();

    std::thread::spawn(move || {
        let mut child = match std::process::Command::new(&args[0])
            .args(&args[1..])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                let _ = sender.send_blocking(format!("Error: {e}"));
                return;
            }
        };

        // Both are Some because we spawned with Stdio::piped() above.
        let stdout = child.stdout.take().expect("stdout piped");
        let stderr = child.stderr.take().expect("stderr piped");

        let tx2 = sender.clone();
        let stderr_thread = std::thread::spawn(move || {
            for line in std::io::BufRead::lines(std::io::BufReader::new(stderr)).flatten() {
                let _ = tx2.send_blocking(line);
            }
        });

        for line in std::io::BufRead::lines(std::io::BufReader::new(stdout)).flatten() {
            let _ = sender.send_blocking(line);
        }
        let _ = child.wait();
        let _ = stderr_thread.join();
    });

    glib::spawn_future_local(async move {
        while let Ok(line) = receiver.recv().await {
            let mut end = log_buf.end_iter();
            log_buf.insert(&mut end, &format!("{line}\n"));
        }
        on_done();
    });
}

/// [`stream_command_then`] with no completion callback.
pub fn stream_command(args: &[&str], log_buf: gtk4::TextBuffer) {
    stream_command_then(args, log_buf, || {});
}

fn systemctl_active(unit: &str) -> bool {
    std::process::Command::new("systemctl")
        .args(["--user", "is-active", "--quiet", unit])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Live systemd `--user` unit status plus Start/Stop/Restart/Logs controls —
/// every bread-ecosystem panel whose app is actually a daemon (not just a
/// config file) gets this, instead of only ever being able to edit the TOML
/// and hope the running process picks it up.
pub fn service_control(unit: &'static str) -> GBox {
    let wrapper = GBox::new(Orientation::Vertical, 4);
    wrapper.append(&section("Service"));

    let status_lbl = Label::new(None);
    status_lbl.add_css_class("dim-label");
    status_lbl.set_selectable(true);
    wrapper.append(&row(unit, &status_lbl));

    let btn_row = GBox::new(Orientation::Horizontal, 8);
    btn_row.set_margin_top(4);
    let toggle_btn = Button::new();
    let restart_btn = Button::with_label("Restart");
    let logs_btn = Button::with_label("View logs");

    // Rc<dyn Fn()> rather than a plain closure: three different button
    // handlers all need to re-run this after their action completes, and
    // plain closures aren't Clone.
    let refresh: Rc<dyn Fn()> = {
        let status_lbl = status_lbl.clone();
        let toggle_btn = toggle_btn.clone();
        Rc::new(move || {
            let active = systemctl_active(unit);
            status_lbl.set_text(if active { "Running" } else { "Stopped" });
            toggle_btn.set_label(if active { "Stop" } else { "Start" });
        })
    };
    refresh();

    {
        let refresh = refresh.clone();
        toggle_btn.connect_clicked(move |_| {
            let verb = if systemctl_active(unit) { "stop" } else { "start" };
            let log_buf = gtk4::TextBuffer::new(None);
            let refresh = refresh.clone();
            stream_command_then(&["systemctl", "--user", verb, unit], log_buf, move || {
                refresh();
            });
        });
    }
    {
        let refresh = refresh.clone();
        restart_btn.connect_clicked(move |_| {
            let log_buf = gtk4::TextBuffer::new(None);
            let refresh = refresh.clone();
            stream_command_then(&["systemctl", "--user", "restart", unit], log_buf, move || {
                refresh();
            });
        });
    }
    logs_btn.connect_clicked(move |_| {
        let _ = std::process::Command::new("kitty")
            .args(["-e", "journalctl", "--user", "-u", unit, "-f"])
            .spawn();
    });

    btn_row.append(&toggle_btn);
    btn_row.append(&restart_btn);
    btn_row.append(&logs_btn);
    wrapper.append(&btn_row);

    wrapper
}

/// A Save button + transient status label that persists the document to `path`.
pub fn save_button(doc: &Doc, path: PathBuf) -> GBox {
    let btn_row = GBox::new(Orientation::Horizontal, 12);
    btn_row.set_margin_top(16);

    let save_btn = Button::with_label("Save");
    save_btn.add_css_class("suggested-action");
    let status = Label::new(None);
    status.add_css_class("dim-label");

    let doc = doc.clone();
    let status_c = status.clone();
    save_btn.connect_clicked(move |_| match config::save_doc(&path, &doc.borrow()) {
        Ok(()) => {
            status_c.set_text("Saved");
            let lbl = status_c.clone();
            glib::timeout_add_seconds_local(3, move || {
                lbl.set_text("");
                glib::ControlFlow::Break
            });
        }
        Err(e) => status_c.set_text(&format!("Error: {e}")),
    });

    btn_row.append(&save_btn);
    btn_row.append(&status);
    btn_row
}
