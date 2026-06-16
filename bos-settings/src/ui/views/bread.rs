//! breadd.toml — the bread daemon config.
//! Schema mirrors breadd/src/core/config.rs (daemon, lua, modules, adapters,
//! events, notifications). Edited non-destructively via the shared document.

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::Box as GBox;

use crate::config;
use crate::ui::widgets as w;

fn config_path() -> std::path::PathBuf {
    config::config_dir().join("bread/breadd.toml")
}

pub fn build() -> GBox {
    let path = config_path();
    let doc = Rc::new(RefCell::new(config::load_doc(&path)));

    let (outer, c) = w::view_scaffold("bread");

    c.append(&w::section("Daemon"));
    c.append(&w::dropdown_row(
        "Log level",
        &doc,
        &["daemon", "log_level"],
        &["error", "warn", "info", "debug", "trace"],
        "info",
    ));
    c.append(&w::entry_row(
        "Socket path",
        &doc,
        &["daemon", "socket_path"],
        "default (XDG runtime dir)",
        "",
    ));

    c.append(&w::section("Lua"));
    c.append(&w::entry_row(
        "Entry point",
        &doc,
        &["lua", "entry_point"],
        "~/.config/bread/init.lua",
        "",
    ));
    c.append(&w::entry_row(
        "Module path",
        &doc,
        &["lua", "module_path"],
        "~/.config/bread/modules",
        "",
    ));

    c.append(&w::section("Modules"));
    c.append(&w::switch_row(
        "Load built-in modules",
        &doc,
        &["modules", "builtin"],
        true,
    ));
    c.append(&w::csv_row(
        "Disabled modules",
        &doc,
        &["modules", "disable"],
        "module-a, module-b",
    ));

    c.append(&w::section("Adapters"));
    c.append(&w::hint(
        "Sources breadd normalises into events. Disable any you don't use.",
    ));
    c.append(&w::switch_row(
        "Hyprland",
        &doc,
        &["adapters", "hyprland", "enabled"],
        true,
    ));
    c.append(&w::switch_row(
        "udev (devices)",
        &doc,
        &["adapters", "udev", "enabled"],
        true,
    ));
    c.append(&w::csv_row(
        "udev subsystems",
        &doc,
        &["adapters", "udev", "subsystems"],
        "usb, input, power_supply",
    ));
    c.append(&w::switch_row(
        "Power",
        &doc,
        &["adapters", "power", "enabled"],
        true,
    ));
    c.append(&w::spin_row(
        "Power poll interval (s)",
        &doc,
        &["adapters", "power", "poll_interval_secs"],
        1.0,
        3600.0,
        1.0,
        30,
    ));
    c.append(&w::switch_row(
        "Network",
        &doc,
        &["adapters", "network", "enabled"],
        true,
    ));
    c.append(&w::switch_row(
        "Bluetooth",
        &doc,
        &["adapters", "bluetooth", "enabled"],
        true,
    ));

    c.append(&w::section("Events"));
    c.append(&w::spin_row(
        "Dedup window (ms)",
        &doc,
        &["events", "dedup_window_ms"],
        0.0,
        10000.0,
        50.0,
        250,
    ));

    c.append(&w::section("Notifications"));
    c.append(&w::spin_row(
        "Default timeout (ms)",
        &doc,
        &["notifications", "default_timeout_ms"],
        0.0,
        60000.0,
        500.0,
        5000,
    ));
    c.append(&w::dropdown_row(
        "Default urgency",
        &doc,
        &["notifications", "default_urgency"],
        &["low", "normal", "critical"],
        "normal",
    ));
    c.append(&w::entry_row(
        "notify-send path",
        &doc,
        &["notifications", "notify_send_path"],
        "auto-detected",
        "",
    ));

    outer.append(&w::save_button(&doc, path));
    outer
}
