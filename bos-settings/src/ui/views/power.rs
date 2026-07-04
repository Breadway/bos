//! Battery/power status (upower), brightness (brightnessctl), and TLP's
//! current profile. Deliberately no AC/Battery/Performance *switcher* —
//! TLP's whole model on BOS is automatic profile selection by power source
//! (see packages.x86_64: power-profiles-daemon is intentionally not
//! installed, it conflicts with tlp), so this panel is read-only for TLP
//! and only exposes controls for things that are actually user choices:
//! brightness, and charge thresholds where the hardware supports them.

use gtk4::prelude::*;
use gtk4::{Box as GBox, Scale};
use std::process::Command;

use crate::ui::widgets as w;

fn upower_device(kind: &str) -> Option<String> {
    let out = Command::new("upower").arg("-e").output().ok()?;
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .find(|l| l.to_lowercase().contains(kind))
        .map(str::to_string)
}

fn upower_field(device: &str, field: &str) -> Option<String> {
    let out = Command::new("upower").args(["-i", device]).output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    text.lines()
        .find(|l| l.trim_start().starts_with(field))
        .and_then(|l| l.split(':').nth(1))
        .map(|v| v.trim().to_string())
}

fn battery_summary() -> Vec<(String, String)> {
    let Some(bat) = upower_device("bat") else {
        return vec![("Battery".to_string(), "No battery detected".to_string())];
    };
    let mut rows = Vec::new();
    if let Some(state) = upower_field(&bat, "state") {
        rows.push(("Status".to_string(), state));
    }
    if let Some(pct) = upower_field(&bat, "percentage") {
        rows.push(("Charge".to_string(), pct));
    }
    if let Some(t) = upower_field(&bat, "time to empty").or_else(|| upower_field(&bat, "time to full")) {
        rows.push(("Time remaining".to_string(), t));
    }
    let full: Option<f64> = upower_field(&bat, "energy-full").and_then(|v| {
        v.split_whitespace().next()?.parse().ok()
    });
    let design: Option<f64> = upower_field(&bat, "energy-full-design").and_then(|v| {
        v.split_whitespace().next()?.parse().ok()
    });
    if let (Some(full), Some(design)) = (full, design) {
        if design > 0.0 {
            rows.push(("Battery health".to_string(), format!("{:.0}% of design capacity", full / design * 100.0)));
        }
    }
    rows
}

fn power_source() -> String {
    match upower_device("ac").and_then(|ac| upower_field(&ac, "online")) {
        Some(v) if v == "yes" => "AC power".to_string(),
        Some(_) => "Battery".to_string(),
        None => "Unknown".to_string(),
    }
}

fn tlp_profile() -> Option<String> {
    let out = Command::new("tlp-stat").arg("-s").output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    text.lines()
        .find(|l| l.trim_start().starts_with("TLP profile"))
        .and_then(|l| l.split('=').nth(1))
        .map(|v| v.trim().to_string())
}

fn brightness_device() -> Option<String> {
    let out = Command::new("brightnessctl").output().ok()?;
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .find(|l| l.starts_with("Device"))
        .and_then(|l| l.split('\'').nth(1))
        .map(str::to_string)
}

fn brightness_pct() -> Option<u32> {
    let out = Command::new("brightnessctl").output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    text.lines()
        .find(|l| l.contains("Current brightness"))
        .and_then(|l| l.split('(').nth(1))
        .and_then(|v| v.trim_end_matches("%)").parse().ok())
}

/// Charge-threshold sysfs paths, only Some when the running kernel driver
/// actually exposes them (ideapad_laptop/thinkpad_acpi on some models;
/// nothing on this dev laptop's Yoga Slim 7 — this is genuinely
/// hardware-dependent, not something every install will have).
fn charge_threshold_paths() -> Option<(std::path::PathBuf, std::path::PathBuf)> {
    let base = std::path::Path::new("/sys/class/power_supply");
    let entries = std::fs::read_dir(base).ok()?;
    for entry in entries.flatten() {
        let start = entry.path().join("charge_control_start_threshold");
        let end = entry.path().join("charge_control_end_threshold");
        if start.exists() && end.exists() {
            return Some((start, end));
        }
    }
    None
}

fn read_threshold(path: &std::path::Path) -> i64 {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(100)
}

pub fn build() -> GBox {
    let (outer, c) = w::view_scaffold("Power");

    c.append(&w::section("Battery"));
    for (label, value) in battery_summary() {
        c.append(&w::info_row(&label, &value));
    }
    c.append(&w::info_row("Power source", &power_source()));

    c.append(&w::section("Brightness"));
    if let Some(pct) = brightness_pct() {
        let scale = Scale::with_range(gtk4::Orientation::Horizontal, 1.0, 100.0, 1.0);
        scale.set_value(pct as f64);
        scale.set_hexpand(true);
        scale.set_draw_value(true);
        if let Some(device) = brightness_device() {
            scale.connect_value_changed(move |s| {
                let pct = format!("{}%", s.value() as i64);
                let _ = Command::new("brightnessctl")
                    .args(["--device", &device, "set", &pct])
                    .spawn();
            });
        }
        c.append(&w::row("Screen brightness", &scale));
    } else {
        c.append(&w::hint("No controllable backlight found."));
    }

    if let Some((start_path, end_path)) = charge_threshold_paths() {
        c.append(&w::section("Charge limits"));
        c.append(&w::hint(
            "Some laptops let you cap charging below 100% to slow battery \
             wear on a machine that's mostly plugged in. Not every model \
             supports this — shown here because yours reported it does.",
        ));
        let start_adj = gtk4::Adjustment::new(read_threshold(&start_path) as f64, 0.0, 100.0, 1.0, 5.0, 0.0);
        let start_spin = gtk4::SpinButton::new(Some(&start_adj), 1.0, 0);
        {
            let path = start_path.clone();
            start_spin.connect_value_changed(move |s| {
                let val = (s.value() as i64).to_string();
                let _ = Command::new("pkexec")
                    .args(["tee", &path.display().to_string()])
                    .arg(&val)
                    .output();
            });
        }
        c.append(&w::row("Start charging below (%)", &start_spin));

        let end_adj = gtk4::Adjustment::new(read_threshold(&end_path) as f64, 1.0, 100.0, 1.0, 5.0, 0.0);
        let end_spin = gtk4::SpinButton::new(Some(&end_adj), 1.0, 0);
        {
            let path = end_path.clone();
            end_spin.connect_value_changed(move |s| {
                let val = (s.value() as i64).to_string();
                let _ = Command::new("pkexec")
                    .args(["tee", &path.display().to_string()])
                    .arg(&val)
                    .output();
            });
        }
        c.append(&w::row("Stop charging at (%)", &end_spin));
    }

    c.append(&w::section("TLP"));
    c.append(&w::hint(
        "TLP automatically applies a power-saving profile on battery and a \
         performance profile on AC — there's no manual profile switch here \
         by design (power-profiles-daemon isn't installed; it conflicts \
         with tlp).",
    ));
    c.append(&w::info_row("Current profile", tlp_profile().as_deref().unwrap_or("unknown")));

    outer
}
