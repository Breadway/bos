//! Timezone + NTP, over `timedatectl`. `systemd-timesyncd` is enabled by
//! default (see `post-install.sh`), so NTP sync is on out of the box — this
//! panel is mostly for picking a timezone and confirming sync is healthy.

use gtk4::prelude::*;
use gtk4::{Box as GBox, DropDown, Expression, Label, StringList, Switch};
use std::process::Command;

use crate::ui::widgets as w;

fn show_property(prop: &str) -> String {
    Command::new("timedatectl")
        .args(["show", &format!("--property={prop}")])
        .output()
        .ok()
        .and_then(|o| {
            String::from_utf8_lossy(&o.stdout)
                .trim()
                .strip_prefix(&format!("{prop}="))
                .map(str::to_string)
        })
        .unwrap_or_default()
}

fn list_timezones() -> Vec<String> {
    Command::new("timedatectl")
        .arg("list-timezones")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).lines().map(str::to_string).collect())
        .unwrap_or_default()
}

fn current_time_label() -> String {
    Command::new("date")
        .arg("+%A, %d %B %Y  %H:%M")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

pub fn build() -> GBox {
    let (outer, content) = w::view_scaffold("Date & Time");

    content.append(&w::info_row("Current time", &current_time_label()));

    content.append(&w::section("Timezone"));
    let zones = list_timezones();
    let current_tz = show_property("Timezone");
    if zones.is_empty() {
        content.append(&w::hint("Couldn't list timezones (is timedatectl available?)."));
    } else {
        let labels: Vec<&str> = zones.iter().map(String::as_str).collect();
        let model = StringList::new(&labels);
        let dd = DropDown::new(Some(model), Expression::NONE);
        let sel = zones.iter().position(|z| *z == current_tz).unwrap_or(0);
        dd.set_selected(sel as u32);

        let status = Label::new(None);
        status.add_css_class("dim-label");

        {
            let zones = zones.clone();
            let status = status.clone();
            dd.connect_selected_notify(move |dd| {
                let Some(tz) = zones.get(dd.selected() as usize) else { return };
                status.set_text("Applying…");
                let log_buf = gtk4::TextBuffer::new(None);
                let status2 = status.clone();
                w::stream_command_then(
                    &["pkexec", "timedatectl", "set-timezone", tz],
                    log_buf.clone(),
                    move || {
                        let text = log_buf.text(&log_buf.start_iter(), &log_buf.end_iter(), false);
                        status2.set_text(if text.trim().is_empty() {
                            "Applied"
                        } else {
                            "Error — check the timezone name"
                        });
                    },
                );
            });
        }

        content.append(&w::row("Timezone", &dd));
        content.append(&status);
    }

    content.append(&w::section("Network time"));
    let ntp_sw = Switch::new();
    ntp_sw.set_active(show_property("NTP") == "yes");
    ntp_sw.connect_active_notify(|s| {
        let val = if s.is_active() { "true" } else { "false" };
        let _ = Command::new("pkexec").args(["timedatectl", "set-ntp", val]).spawn();
    });
    content.append(&w::row("Synchronize automatically", &ntp_sw));

    let synced = show_property("NTPSynchronized") == "yes";
    content.append(&w::hint(if synced {
        "Synchronized."
    } else {
        "Not synchronized yet (needs network)."
    }));

    outer
}
