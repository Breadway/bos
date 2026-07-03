//! Wi-Fi + Ethernet over `nmcli`. NetworkManager lets the active session user
//! manage connections via polkit already, so the common paths (scan, connect,
//! toggle radio) need no `pkexec`. VPN import, 802.1x, and other edge cases
//! are punted to `nm-connection-editor` via the Advanced button rather than
//! reimplemented here.
//!
//! The scan is deliberately NOT run in `build()` — every view is constructed
//! eagerly at app launch (see `window.rs`), and a Wi-Fi scan takes seconds;
//! doing it here would add that latency to every bos-settings launch. It
//! only runs when the user clicks Scan.

use gtk4::prelude::*;
use gtk4::{Box as GBox, Button, Entry, Label, ListBox, ListBoxRow, Orientation, ScrolledWindow, Switch};
use std::cell::RefCell;
use std::collections::HashSet;
use std::process::Command;
use std::rc::Rc;

use crate::ui::widgets as w;

#[derive(Clone)]
struct WifiNetwork {
    ssid: String,
    signal: i32,
    secured: bool,
    active: bool,
}

fn radio_enabled() -> bool {
    Command::new("nmcli")
        .args(["radio", "wifi"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "enabled")
        .unwrap_or(false)
}

fn ethernet_status() -> Option<String> {
    let out = Command::new("nmcli").args(["-t", "-f", "DEVICE,TYPE,STATE"]).arg("dev").output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    text.lines().find_map(|l| {
        let mut cols = l.splitn(3, ':');
        let (dev, ty, state) = (cols.next()?, cols.next()?, cols.next()?);
        (ty == "ethernet").then(|| format!("{dev}: {state}"))
    })
}

fn known_connection_names() -> HashSet<String> {
    let Ok(out) = Command::new("nmcli").args(["-t", "-f", "NAME"]).arg("con").arg("show").output() else {
        return HashSet::new();
    };
    String::from_utf8_lossy(&out.stdout).lines().map(str::to_string).collect()
}

/// Scan + list Wi-Fi networks, deduplicated by SSID (keeping the strongest
/// signal — the same AP shows once per band/BSSID otherwise).
fn scan_wifi() -> Vec<WifiNetwork> {
    let _ = Command::new("nmcli").args(["dev", "wifi", "rescan"]).output();
    std::thread::sleep(std::time::Duration::from_secs(2));
    let Ok(out) = Command::new("nmcli")
        .args(["-t", "-f", "SSID,SIGNAL,SECURITY,IN-USE", "dev", "wifi", "list"])
        .output()
    else {
        return Vec::new();
    };
    let text = String::from_utf8_lossy(&out.stdout);
    let mut by_ssid: std::collections::HashMap<String, WifiNetwork> = std::collections::HashMap::new();
    for line in text.lines() {
        let mut cols = line.splitn(4, ':');
        let (ssid, signal, security, in_use) = (cols.next(), cols.next(), cols.next(), cols.next());
        let Some(ssid) = ssid.filter(|s| !s.is_empty()) else { continue };
        let signal: i32 = signal.and_then(|s| s.parse().ok()).unwrap_or(0);
        let net = WifiNetwork {
            ssid: ssid.to_string(),
            signal,
            secured: security.map(|s| !s.is_empty()).unwrap_or(false),
            active: in_use == Some("*"),
        };
        by_ssid
            .entry(ssid.to_string())
            .and_modify(|existing| if net.signal > existing.signal { *existing = net.clone() })
            .or_insert(net);
    }
    let mut list: Vec<_> = by_ssid.into_values().collect();
    list.sort_by(|a, b| b.signal.cmp(&a.signal));
    list
}

pub fn build() -> GBox {
    let (outer, content) = w::view_scaffold("Network");

    content.append(&w::section("Wi-Fi"));
    let radio_sw = Switch::new();
    radio_sw.set_active(radio_enabled());
    radio_sw.connect_active_notify(|s| {
        let val = if s.is_active() { "on" } else { "off" };
        let _ = Command::new("nmcli").args(["radio", "wifi", val]).spawn();
    });
    content.append(&w::row("Wi-Fi radio", &radio_sw));

    let status = Label::new(None);
    status.add_css_class("dim-label");
    status.set_xalign(0.0);

    let list = ListBox::new();
    list.set_selection_mode(gtk4::SelectionMode::None);
    let scroll = ScrolledWindow::new();
    scroll.set_min_content_height(220);
    scroll.set_child(Some(&list));
    // Hidden until a scan actually produces results — a 220px empty scroll
    // area between the radio row and the Scan button was dead space on
    // every fresh open of this panel.
    scroll.set_visible(false);
    content.append(&scroll);

    let not_scanned = w::hint("Not scanned yet — press Scan to see nearby networks.");
    content.append(&not_scanned);

    // Password entry, shown only while connecting to a secured, unknown
    // network — set visible/hidden rather than a modal dialog, to keep this
    // panel's async flow in one place instead of a nested dialog callback.
    let pw_row = GBox::new(Orientation::Horizontal, 8);
    let pw_entry = Entry::new();
    pw_entry.set_visibility(false);
    pw_entry.set_hexpand(true);
    pw_entry.set_placeholder_text(Some("Password"));
    let pw_connect_btn = Button::with_label("Connect");
    pw_row.append(&pw_entry);
    pw_row.append(&pw_connect_btn);
    pw_row.set_visible(false);
    content.append(&pw_row);
    content.append(&status);

    let pending_ssid: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));

    let connect_open_or_known = {
        let status = status.clone();
        move |ssid: String| {
            status.set_text(&format!("Connecting to {ssid}…"));
            let log_buf = gtk4::TextBuffer::new(None);
            let status2 = status.clone();
            let ssid2 = ssid.clone();
            let known = known_connection_names();
            let args: Vec<String> = if known.contains(&ssid) {
                vec!["nmcli".into(), "con".into(), "up".into(), ssid.clone()]
            } else {
                vec!["nmcli".into(), "dev".into(), "wifi".into(), "connect".into(), ssid.clone()]
            };
            let args_ref: Vec<&str> = args.iter().map(String::as_str).collect();
            w::stream_command_then(&args_ref, log_buf.clone(), move || {
                let text = log_buf.text(&log_buf.start_iter(), &log_buf.end_iter(), false);
                if text.to_lowercase().contains("error") {
                    status2.set_text(&format!("Failed to connect to {ssid2}: {}", text.trim()));
                } else {
                    status2.set_text(&format!("Connected to {ssid2}"));
                }
            });
        }
    };

    {
        let pw_row = pw_row.clone();
        let pw_entry = pw_entry.clone();
        let pending_ssid = pending_ssid.clone();
        let status = status.clone();
        pw_connect_btn.connect_clicked(move |_| {
            let Some(ssid) = pending_ssid.borrow_mut().take() else { return };
            let password = pw_entry.text().to_string();
            pw_row.set_visible(false);
            pw_entry.set_text("");
            status.set_text(&format!("Connecting to {ssid}…"));
            let log_buf = gtk4::TextBuffer::new(None);
            let status2 = status.clone();
            let ssid2 = ssid.clone();
            w::stream_command_then(
                &["nmcli", "dev", "wifi", "connect", &ssid, "password", &password],
                log_buf.clone(),
                move || {
                    let text = log_buf.text(&log_buf.start_iter(), &log_buf.end_iter(), false);
                    if text.to_lowercase().contains("error") {
                        status2.set_text(&format!("Failed to connect to {ssid2}: wrong password?"));
                    } else {
                        status2.set_text(&format!("Connected to {ssid2}"));
                    }
                },
            );
        });
    }

    let populate_list = {
        let list = list.clone();
        let scroll = scroll.clone();
        let not_scanned = not_scanned.clone();
        let pw_row = pw_row.clone();
        let pending_ssid = pending_ssid.clone();
        let connect_open_or_known = connect_open_or_known.clone();
        move |networks: Vec<WifiNetwork>| {
            while let Some(child) = list.first_child() {
                list.remove(&child);
            }
            not_scanned.set_visible(false);
            scroll.set_visible(true);
            if networks.is_empty() {
                let row = ListBoxRow::new();
                row.set_selectable(false);
                row.set_child(Some(&w::empty_state(
                    "network-wireless-offline-symbolic",
                    "No networks found",
                    "Try Scan again, or check Wi-Fi radio is on.",
                )));
                list.append(&row);
                return;
            }
            let known = known_connection_names();
            for net in networks {
                let row = ListBoxRow::new();
                let hbox = GBox::new(Orientation::Horizontal, 12);
                hbox.set_margin_top(4);
                hbox.set_margin_bottom(4);
                hbox.set_margin_start(8);
                hbox.set_margin_end(8);

                let name = if net.active {
                    format!("{}  (connected)", net.ssid)
                } else {
                    net.ssid.clone()
                };
                let name_lbl = Label::new(Some(&name));
                name_lbl.set_hexpand(true);
                name_lbl.set_xalign(0.0);
                if net.active {
                    name_lbl.add_css_class("heading");
                }

                let signal_icon_name = match net.signal.clamp(0, 100) {
                    0..=24 => "network-wireless-signal-weak-symbolic",
                    25..=49 => "network-wireless-signal-ok-symbolic",
                    50..=74 => "network-wireless-signal-good-symbolic",
                    _ => "network-wireless-signal-excellent-symbolic",
                };
                let signal_icon = gtk4::Image::from_icon_name(signal_icon_name);
                signal_icon.add_css_class("dim-label");

                let meta_lbl = Label::new(Some(&format!("{}%", net.signal.clamp(0, 100))));
                meta_lbl.add_css_class("dim-label");

                hbox.append(&name_lbl);
                if net.secured {
                    let lock_icon = gtk4::Image::from_icon_name("channel-secure-symbolic");
                    lock_icon.add_css_class("dim-label");
                    hbox.append(&lock_icon);
                }
                hbox.append(&signal_icon);
                hbox.append(&meta_lbl);

                if !net.active {
                    let connect_btn = Button::with_label("Connect");
                    let ssid = net.ssid.clone();
                    let secured = net.secured;
                    let already_known = known.contains(&net.ssid);
                    let pw_row = pw_row.clone();
                    let pending_ssid = pending_ssid.clone();
                    let connect_open_or_known = connect_open_or_known.clone();
                    connect_btn.connect_clicked(move |_| {
                        if secured && !already_known {
                            *pending_ssid.borrow_mut() = Some(ssid.clone());
                            pw_row.set_visible(true);
                        } else {
                            pw_row.set_visible(false);
                            connect_open_or_known(ssid.clone());
                        }
                    });
                    hbox.append(&connect_btn);
                }

                row.set_child(Some(&hbox));
                list.append(&row);
            }
        }
    };

    let scan_btn = Button::with_label("Scan");
    {
        let status = status.clone();
        let populate_list = populate_list.clone();
        scan_btn.connect_clicked(move |btn| {
            btn.set_sensitive(false);
            status.set_text("Scanning…");
            let (tx, rx) = async_channel::bounded::<Vec<WifiNetwork>>(1);
            std::thread::spawn(move || {
                let nets = scan_wifi();
                let _ = tx.send_blocking(nets);
            });
            let btn = btn.clone();
            let status = status.clone();
            let populate_list = populate_list.clone();
            glib::spawn_future_local(async move {
                if let Ok(nets) = rx.recv().await {
                    let count = nets.len();
                    populate_list(nets);
                    status.set_text(&format!("Found {count} network(s)"));
                }
                btn.set_sensitive(true);
            });
        });
    }
    content.append(&scan_btn);

    if let Some(eth) = ethernet_status() {
        content.append(&w::section("Ethernet"));
        content.append(&w::info_row("Status", &eth));
    }

    content.append(&w::section("Advanced"));
    content.append(&w::hint("VPN, 802.1x, and static IP configuration aren't covered here."));
    let adv_btn = Button::with_label("Open connection editor");
    adv_btn.set_halign(gtk4::Align::Start);
    adv_btn.connect_clicked(|_| {
        let _ = Command::new("nm-connection-editor").spawn();
    });
    content.append(&adv_btn);

    outer
}
