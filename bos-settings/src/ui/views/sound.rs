//! Output/input volume and device selection over PipeWire's pulse
//! compatibility layer (`pactl`) — the same surface `hyprland.lua`'s media
//! keys already use via `wpctl`. `pactl` is used here instead of `wpctl`
//! because it can enumerate devices with human-readable descriptions and
//! switch the default in one command; `wpctl` cannot easily do either.

use gtk4::prelude::*;
use gtk4::{Box as GBox, Button, DropDown, Expression, Orientation, Scale, StringList, Switch};
use serde::Deserialize;
use std::process::Command;

use crate::ui::widgets as w;

#[derive(Deserialize, Clone)]
struct Device {
    name: String,
    description: String,
    mute: bool,
    volume: std::collections::HashMap<String, VolumeChannel>,
}

#[derive(Deserialize, Clone)]
struct VolumeChannel {
    value_percent: String,
}

impl Device {
    fn percent(&self) -> f64 {
        self.volume
            .values()
            .next()
            .and_then(|v| v.value_percent.trim_end_matches('%').trim().parse::<f64>().ok())
            .unwrap_or(0.0)
    }
}

fn list_devices(kind: &str) -> Vec<Device> {
    let Ok(output) = Command::new("pactl").args(["-f", "json", "list", kind]).output() else {
        return Vec::new();
    };
    serde_json::from_slice(&output.stdout).unwrap_or_default()
}

fn default_device_name(kind: &str) -> Option<String> {
    let flag = if kind == "sinks" { "get-default-sink" } else { "get-default-source" };
    Command::new("pactl")
        .arg(flag)
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Build one section (Output or Input): device dropdown + volume slider + mute
/// switch. `kind` is "sinks" or "sources"; `set_default_flag` is the pactl
/// verb ("set-default-sink"/"set-default-source"); `vol_flag`/`mute_flag`
/// likewise.
fn build_section(
    title: &str,
    kind: &'static str,
    set_default_flag: &'static str,
    vol_flag: &'static str,
    mute_flag: &'static str,
) -> GBox {
    let section = GBox::new(Orientation::Vertical, 4);
    section.append(&w::section(title));

    let devices = list_devices(kind);
    if devices.is_empty() {
        section.append(&w::hint("No devices found."));
        return section;
    }

    let current_name = default_device_name(kind);
    let selected_idx = current_name
        .as_ref()
        .and_then(|n| devices.iter().position(|d| &d.name == n))
        .unwrap_or(0);
    let current = devices[selected_idx].clone();

    let labels: Vec<&str> = devices.iter().map(|d| d.description.as_str()).collect();
    let model = StringList::new(&labels);
    let dd = DropDown::new(Some(model), Expression::NONE);
    dd.set_selected(selected_idx as u32);
    section.append(&w::row("Device", &dd));

    let scale = Scale::with_range(Orientation::Horizontal, 0.0, 150.0, 1.0);
    scale.set_value(current.percent());
    scale.set_size_request(180, -1);
    scale.set_draw_value(true);
    scale.set_value_pos(gtk4::PositionType::Right);
    section.append(&w::row("Volume", &scale));

    let mute_sw = Switch::new();
    mute_sw.set_active(current.mute);
    section.append(&w::row("Mute", &mute_sw));

    // Device switch: fire-and-forget pactl call, then resync the slider/mute
    // switch to whatever the newly-default device's actual state is.
    {
        let devices = devices.clone();
        let scale = scale.clone();
        let mute_sw = mute_sw.clone();
        dd.connect_selected_notify(move |dd| {
            let Some(d) = devices.get(dd.selected() as usize) else { return };
            let _ = Command::new("pactl").args([set_default_flag, &d.name]).spawn();
            scale.set_value(d.percent());
            mute_sw.set_active(d.mute);
        });
    }

    // Volume: target whichever device is currently selected in the dropdown,
    // not necessarily the system default at the time this closure was built.
    {
        let devices = devices.clone();
        let dd = dd.clone();
        scale.connect_value_changed(move |s| {
            let Some(d) = devices.get(dd.selected() as usize) else { return };
            let pct = format!("{}%", s.value() as i64);
            let _ = Command::new("pactl").args([vol_flag, &d.name, &pct]).spawn();
        });
    }

    {
        let devices = devices.clone();
        let dd = dd.clone();
        mute_sw.connect_active_notify(move |s| {
            let Some(d) = devices.get(dd.selected() as usize) else { return };
            let val = if s.is_active() { "1" } else { "0" };
            let _ = Command::new("pactl").args([mute_flag, &d.name, val]).spawn();
        });
    }

    section
}

pub fn build() -> GBox {
    let (outer, content) = w::view_scaffold("Sound");

    content.append(&build_section(
        "Output",
        "sinks",
        "set-default-sink",
        "set-sink-volume",
        "set-sink-mute",
    ));
    content.append(&build_section(
        "Input",
        "sources",
        "set-default-source",
        "set-source-volume",
        "set-source-mute",
    ));

    content.append(&w::section("Advanced"));
    content.append(&w::hint(
        "Per-app volume, port selection, and profile switching aren't covered here.",
    ));
    let mixer_btn = Button::with_label("Open advanced mixer (pavucontrol)");
    mixer_btn.set_halign(gtk4::Align::Start);
    mixer_btn.connect_clicked(|_| {
        let _ = Command::new("pavucontrol").spawn();
    });
    content.append(&mixer_btn);

    outer
}
