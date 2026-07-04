//! ufw firewall rules — BOS enables ufw by default (deny incoming, allow
//! outgoing, mDNS allowed for printer discovery; see post-install.sh) but
//! previously offered no graphical way to add/remove a rule for something
//! like a local dev server or a LAN game, only a terminal.
//!
//! `ufw status` itself requires root — confirmed against the installed
//! ufw script, which exits with "ERROR: You need to be root to run this
//! script" for a plain status check, not just for changes. Every other
//! panel's read-only state is free to query eagerly in `build()`, but doing
//! that here would mean a polkit password prompt on every single
//! bos-settings launch (every view is constructed immediately at startup —
//! see window.rs). So this panel starts blank and loads state only when the
//! user clicks Refresh, deferring the one unavoidable prompt to an explicit
//! action instead of forcing it on app open.

use gtk4::prelude::*;
use gtk4::{Box as GBox, Button, Entry, Label, ListBox, ListBoxRow, Orientation, ScrolledWindow, Switch};
use std::cell::Cell;
use std::rc::Rc;

use crate::ui::widgets as w;

#[derive(Clone)]
struct Rule {
    number: String,
    text: String,
}

struct Status {
    active: bool,
    rules: Vec<Rule>,
}

/// One `pkexec ufw status numbered` call, parsed for both the active/inactive
/// line and the numbered rules — a single privileged read instead of two.
fn fetch_status() -> Option<Status> {
    let output = std::process::Command::new("pkexec")
        .args(["ufw", "status", "numbered"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let active = text.lines().next().is_some_and(|l| l.trim() == "Status: active");
    let rules = text
        .lines()
        .filter_map(|l| {
            let l = l.trim_start();
            if !l.starts_with('[') {
                return None;
            }
            let (num, rest) = l.split_once(']')?;
            let number = num.trim_start_matches('[').trim().to_string();
            Some(Rule { number, text: rest.trim().to_string() })
        })
        .collect();
    Some(Status { active, rules })
}

fn render_rules(list: &ListBox, rules: &[Rule], programmatic: &Rc<Cell<bool>>) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
    if rules.is_empty() {
        let row = ListBoxRow::new();
        row.set_selectable(false);
        row.set_child(Some(&w::empty_state(
            "security-high-symbolic",
            "No rules",
            "Only the default policy (deny incoming, allow outgoing) applies.",
        )));
        list.append(&row);
        return;
    }
    for rule in rules {
        let row = ListBoxRow::new();
        row.set_selectable(false);
        let hbox = GBox::new(Orientation::Horizontal, 16);
        hbox.add_css_class("card");
        hbox.set_margin_top(3);
        hbox.set_margin_bottom(3);

        let text_lbl = Label::new(Some(&rule.text));
        text_lbl.set_hexpand(true);
        text_lbl.set_xalign(0.0);
        text_lbl.set_wrap(true);

        let remove_btn = Button::with_label("Remove");
        remove_btn.add_css_class("destructive-action");
        {
            let list = list.clone();
            let number = rule.number.clone();
            let programmatic = programmatic.clone();
            remove_btn.connect_clicked(move |_| {
                let log_buf = gtk4::TextBuffer::new(None);
                let list2 = list.clone();
                let programmatic2 = programmatic.clone();
                w::stream_command_then(
                    &["pkexec", "ufw", "--force", "delete", &number],
                    log_buf,
                    move || refresh(&list2, None, &programmatic2),
                );
            });
        }

        hbox.append(&text_lbl);
        hbox.append(&remove_btn);
        row.set_child(Some(&hbox));
        list.append(&row);
    }
}

/// Re-fetch status on a background thread and update the list (+ switch, if
/// given) on completion. `enabled_sw` is `None` when called from a row
/// action (delete/add) where the enabled state can't have changed.
///
/// `programmatic` guards against `set_active` below re-triggering the
/// switch's own `connect_active_notify` handler (which calls `ufw enable`/
/// `disable`) — without it, the very first Refresh after ufw turns out to
/// already be active would immediately fire an unwanted `ufw enable` the
/// moment `set_active(true)` flips a switch that just became sensitive.
fn refresh(list: &ListBox, enabled_sw: Option<&Switch>, programmatic: &Rc<Cell<bool>>) {
    let (tx, rx) = async_channel::bounded::<Option<Status>>(1);
    std::thread::spawn(move || {
        let _ = tx.send_blocking(fetch_status());
    });
    let list = list.clone();
    let enabled_sw = enabled_sw.cloned();
    let programmatic = programmatic.clone();
    glib::spawn_future_local(async move {
        if let Ok(Some(status)) = rx.recv().await {
            render_rules(&list, &status.rules, &programmatic);
            if let Some(sw) = &enabled_sw {
                programmatic.set(true);
                sw.set_sensitive(true);
                sw.set_active(status.active);
                programmatic.set(false);
            }
        }
    });
}

pub fn build() -> GBox {
    let (outer, content) = w::view_scaffold("Firewall");
    content.append(&w::hint(
        "Reading and changing firewall state needs your password (polkit) \
         — ufw requires root even just to check status. Click Refresh to \
         load the current state.",
    ));

    let programmatic = Rc::new(Cell::new(false));

    let enabled_sw = Switch::new();
    enabled_sw.set_sensitive(false);
    content.append(&w::row("Firewall enabled", &enabled_sw));
    content.append(&w::hint(
        "Default policy: deny incoming, allow outgoing. mDNS (5353/udp) is \
         allowed by default so printer/network discovery keeps working.",
    ));

    let list = ListBox::new();
    list.set_selection_mode(gtk4::SelectionMode::None);
    {
        let row = ListBoxRow::new();
        row.set_selectable(false);
        row.set_child(Some(&w::empty_state(
            "security-high-symbolic",
            "Status not loaded",
            "Click Refresh below to check the firewall's current state.",
        )));
        list.append(&row);
    }
    let scroll = ScrolledWindow::new();
    scroll.set_vexpand(true);
    scroll.set_min_content_height(260);
    scroll.set_child(Some(&list));
    content.append(&scroll);

    let refresh_btn = Button::with_label("Refresh status");
    {
        let list = list.clone();
        let enabled_sw = enabled_sw.clone();
        let programmatic = programmatic.clone();
        refresh_btn.connect_clicked(move |_| refresh(&list, Some(&enabled_sw), &programmatic));
    }
    content.append(&refresh_btn);

    {
        let list = list.clone();
        let programmatic = programmatic.clone();
        enabled_sw.connect_active_notify(move |s| {
            // Skip both the pre-refresh insensitive state and any
            // programmatic set_active() from refresh() itself — only a
            // real user click should call out to ufw enable/disable.
            if !s.is_sensitive() || programmatic.get() {
                return;
            }
            let verb = if s.is_active() { "enable" } else { "disable" };
            let log_buf = gtk4::TextBuffer::new(None);
            let list2 = list.clone();
            let programmatic2 = programmatic.clone();
            w::stream_command_then(&["pkexec", "ufw", "--force", verb], log_buf, move || {
                refresh(&list2, None, &programmatic2);
            });
        });
    }

    content.append(&w::section("Add rule"));
    content.append(&w::hint(
        "e.g. \"8080/tcp\", \"22/tcp\", or a service name like \"OpenSSH\".",
    ));
    let add_row = GBox::new(Orientation::Horizontal, 8);
    let add_entry = Entry::new();
    add_entry.set_hexpand(true);
    add_entry.set_placeholder_text(Some("port/proto or service name"));
    let add_btn = Button::with_label("Allow");
    {
        let list = list.clone();
        let add_entry = add_entry.clone();
        let programmatic = programmatic.clone();
        add_btn.connect_clicked(move |_| {
            let rule = add_entry.text().to_string();
            if rule.trim().is_empty() {
                return;
            }
            let log_buf = gtk4::TextBuffer::new(None);
            let list2 = list.clone();
            let add_entry2 = add_entry.clone();
            let programmatic2 = programmatic.clone();
            w::stream_command_then(
                &["pkexec", "ufw", "allow", rule.trim()],
                log_buf,
                move || {
                    add_entry2.set_text("");
                    refresh(&list2, None, &programmatic2);
                },
            );
        });
    }
    add_row.append(&add_entry);
    add_row.append(&add_btn);
    content.append(&add_row);

    outer
}
