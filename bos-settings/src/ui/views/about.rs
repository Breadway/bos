//! Read-only system info, plus the one thing worth making writable: hostname.
//! BOS is a rolling release (no fixed version number to show — `os-release`
//! ships `BUILD_ID=rolling` on purpose), so there's no "BOS 1.2.3" readout
//! here the way a point-release distro's About panel would have one.

use gtk4::prelude::*;
use gtk4::{Box as GBox, Button, Entry, Label, Orientation};
use std::fs;
use std::process::Command;

use crate::ui::widgets as w;

fn os_pretty_name() -> String {
    fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|s| {
            s.lines()
                .find_map(|l| l.strip_prefix("PRETTY_NAME=").map(|v| v.trim_matches('"').to_string()))
        })
        .unwrap_or_else(|| "BOS".to_string())
}

fn hostname() -> String {
    fs::read_to_string("/etc/hostname")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string())
}

fn kernel() -> String {
    Command::new("uname")
        .arg("-r")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

fn cpu() -> String {
    let model = fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|s| {
            s.lines()
                .find_map(|l| l.strip_prefix("model name").map(|v| v.trim_start_matches([':', ' ', '\t']).to_string()))
        })
        .unwrap_or_else(|| "unknown".to_string());
    let cores = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(0);
    if cores > 0 {
        format!("{model} ({cores} threads)")
    } else {
        model
    }
}

fn memory() -> String {
    let kb = fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("MemTotal:"))
                .and_then(|l| l.split_whitespace().nth(1))
                .and_then(|v| v.parse::<u64>().ok())
        });
    match kb {
        Some(kb) => format!("{:.1} GiB", kb as f64 / 1024.0 / 1024.0),
        None => "unknown".to_string(),
    }
}

fn gpu() -> String {
    let Ok(output) = Command::new("lspci").output() else {
        return "unknown".to_string();
    };
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines()
        // "Display controller" covers integrated GPUs some laptop chipsets
        // (this dev laptop's AMD Radeon 860M included) report under instead
        // of "VGA compatible controller" — without it those show "unknown".
        .find(|l| {
            l.contains("VGA compatible controller")
                || l.contains("3D controller")
                || l.contains("Display controller")
        })
        .and_then(|l| l.split(": ").nth(1))
        .unwrap_or("unknown")
        .to_string()
}

fn disk_usage() -> String {
    let Ok(output) = Command::new("df").args(["-h", "--output=used,size,pcent", "/"]).output() else {
        return "unknown".to_string();
    };
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines()
        .nth(1)
        .map(|l| {
            let cols: Vec<&str> = l.split_whitespace().collect();
            match cols.as_slice() {
                [used, size, pcent] => format!("{used} of {size} used ({pcent})"),
                _ => l.trim().to_string(),
            }
        })
        .unwrap_or_else(|| "unknown".to_string())
}

fn uptime() -> String {
    Command::new("uptime")
        .arg("-p")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

pub fn build() -> GBox {
    let (outer, content) = w::view_scaffold("About");

    content.append(&w::info_row("Operating system", &os_pretty_name()));
    content.append(&w::info_row("Kernel", &kernel()));
    content.append(&w::info_row("CPU", &cpu()));
    content.append(&w::info_row("GPU", &gpu()));
    content.append(&w::info_row("Memory", &memory()));
    content.append(&w::info_row("Disk (/)", &disk_usage()));
    content.append(&w::info_row("Uptime", &uptime()));

    content.append(&w::section("Hostname"));
    content.append(&w::hint(
        "Changes the machine's network name. Takes effect immediately; \
         needs your password (polkit).",
    ));

    let hn_row = GBox::new(Orientation::Horizontal, 12);
    let entry = Entry::new();
    entry.set_text(&hostname());
    entry.set_hexpand(true);
    let apply_btn = Button::with_label("Apply");
    let status = Label::new(None);
    status.add_css_class("dim-label");

    {
        let entry = entry.clone();
        let status = status.clone();
        apply_btn.connect_clicked(move |_| {
            let name = entry.text().to_string();
            if name.trim().is_empty() {
                status.set_text("Hostname can't be empty");
                return;
            }
            let log_buf = gtk4::TextBuffer::new(None);
            let status2 = status.clone();
            status.set_text("Applying…");
            w::stream_command_then(
                &["pkexec", "hostnamectl", "set-hostname", name.trim()],
                log_buf.clone(),
                move || {
                    let text = log_buf.text(&log_buf.start_iter(), &log_buf.end_iter(), false);
                    if text.trim().is_empty() {
                        status2.set_text("Applied");
                    } else {
                        status2.set_text(&format!("Error: {}", text.trim()));
                    }
                },
            );
        });
    }

    hn_row.append(&entry);
    hn_row.append(&apply_btn);
    content.append(&hn_row);
    content.append(&status);

    outer
}
