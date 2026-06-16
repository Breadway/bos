//! breadcrumbs.toml — Wi-Fi profile state machine.
//! Schema mirrors breadcrumbs/src/config.rs:
//!   [settings]              scalar tunables
//!   [[networks]]            saved networks (ssid / password / hidden)
//!   [profiles.<name>]       per-location profile (networks, tailscale, …)
//! `[settings]` is edited in place; the `networks` array and `profiles` table
//! are rewritten from their editors on save. Other keys/comments are preserved.

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{
    Box as GBox, Button, Entry, Label, ListBox, ListBoxRow, Orientation, ScrolledWindow, Switch,
};
use toml_edit::{value, Array, ArrayOfTables, DocumentMut, Item, Table};

use crate::config;
use crate::ui::widgets as w;

fn config_path() -> std::path::PathBuf {
    config::config_dir().join("breadcrumbs/breadcrumbs.toml")
}

// --- networks ---------------------------------------------------------------

#[derive(Clone, Default)]
struct Network {
    ssid: String,
    password: String,
    hidden: bool,
}

fn read_networks(doc: &DocumentMut) -> Vec<Network> {
    let Some(aot) = doc.get("networks").and_then(Item::as_array_of_tables) else {
        return Vec::new();
    };
    aot.iter()
        .map(|t| Network {
            ssid: t.get("ssid").and_then(Item::as_str).unwrap_or("").to_string(),
            password: t
                .get("password")
                .and_then(Item::as_str)
                .unwrap_or("")
                .to_string(),
            hidden: t.get("hidden").and_then(Item::as_bool).unwrap_or(false),
        })
        .collect()
}

fn write_networks(doc: &mut DocumentMut, nets: &[Network]) {
    let mut aot = ArrayOfTables::new();
    for n in nets {
        let mut t = Table::new();
        t.insert("ssid", value(&n.ssid));
        t.insert("password", value(&n.password));
        t.insert("hidden", value(n.hidden));
        aot.push(t);
    }
    doc.as_table_mut().insert("networks", Item::ArrayOfTables(aot));
}

fn rebuild_networks(list: &ListBox, model: &Rc<RefCell<Vec<Network>>>) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
    for (i, n) in model.borrow().iter().enumerate() {
        let row = ListBoxRow::new();
        row.set_selectable(false);
        let hbox = GBox::new(Orientation::Horizontal, 8);
        hbox.set_margin_top(6);
        hbox.set_margin_bottom(6);
        hbox.set_margin_start(8);
        hbox.set_margin_end(8);

        let ssid = Entry::new();
        ssid.set_text(&n.ssid);
        ssid.set_width_chars(16);
        ssid.set_placeholder_text(Some("SSID"));

        let pass = Entry::new();
        pass.set_text(&n.password);
        pass.set_hexpand(true);
        pass.set_visibility(false);
        pass.set_input_purpose(gtk4::InputPurpose::Password);
        pass.set_placeholder_text(Some("password"));

        let hidden = Switch::new();
        hidden.set_active(n.hidden);
        hidden.set_valign(gtk4::Align::Center);
        hidden.set_tooltip_text(Some("Hidden network"));

        let remove = Button::with_label("Remove");
        remove.add_css_class("destructive-action");

        {
            let model = model.clone();
            ssid.connect_changed(move |e| {
                if let Some(n) = model.borrow_mut().get_mut(i) {
                    n.ssid = e.text().to_string();
                }
            });
        }
        {
            let model = model.clone();
            pass.connect_changed(move |e| {
                if let Some(n) = model.borrow_mut().get_mut(i) {
                    n.password = e.text().to_string();
                }
            });
        }
        {
            let model = model.clone();
            hidden.connect_active_notify(move |s| {
                if let Some(n) = model.borrow_mut().get_mut(i) {
                    n.hidden = s.is_active();
                }
            });
        }
        {
            let model = model.clone();
            let list = list.clone();
            remove.connect_clicked(move |_| {
                model.borrow_mut().remove(i);
                rebuild_networks(&list, &model);
            });
        }

        hbox.append(&ssid);
        hbox.append(&pass);
        hbox.append(&Label::new(Some("hidden")));
        hbox.append(&hidden);
        hbox.append(&remove);
        row.set_child(Some(&hbox));
        list.append(&row);
    }
}

// --- profiles ---------------------------------------------------------------

#[derive(Clone, Default)]
struct Profile {
    name: String,
    networks: Vec<String>,
    detect_ssids: Vec<String>,
    bootstrap: String,
    exit_node: String,
    tailscale: bool,
    include_all_known: bool,
}

fn read_profiles(doc: &DocumentMut) -> Vec<Profile> {
    let Some(tbl) = doc.get("profiles").and_then(Item::as_table) else {
        return Vec::new();
    };
    let str_list = |item: Option<&Item>| -> Vec<String> {
        item.and_then(Item::as_array)
            .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default()
    };
    tbl.iter()
        .filter_map(|(name, item)| {
            let p = item.as_table()?;
            Some(Profile {
                name: name.to_string(),
                networks: str_list(p.get("networks")),
                detect_ssids: str_list(p.get("detect_ssids")),
                bootstrap: p.get("bootstrap").and_then(Item::as_str).unwrap_or("").to_string(),
                exit_node: p.get("exit_node").and_then(Item::as_str).unwrap_or("").to_string(),
                tailscale: p.get("tailscale").and_then(Item::as_bool).unwrap_or(false),
                include_all_known: p
                    .get("include_all_known")
                    .and_then(Item::as_bool)
                    .unwrap_or(false),
            })
        })
        .collect()
}

fn write_profiles(doc: &mut DocumentMut, profiles: &[Profile]) {
    let mut tbl = Table::new();
    let to_arr = |items: &[String]| {
        let mut a = Array::new();
        for s in items {
            a.push(s.as_str());
        }
        a
    };
    for p in profiles {
        if p.name.is_empty() {
            continue;
        }
        let mut t = Table::new();
        t.insert("networks", value(to_arr(&p.networks)));
        t.insert("tailscale", value(p.tailscale));
        t.insert("include_all_known", value(p.include_all_known));
        if !p.detect_ssids.is_empty() {
            t.insert("detect_ssids", value(to_arr(&p.detect_ssids)));
        }
        if !p.bootstrap.is_empty() {
            t.insert("bootstrap", value(&p.bootstrap));
        }
        if !p.exit_node.is_empty() {
            t.insert("exit_node", value(&p.exit_node));
        }
        tbl.insert(&p.name, Item::Table(t));
    }
    doc.as_table_mut().insert("profiles", Item::Table(tbl));
}

fn field(label: &str, control: &impl IsA<gtk4::Widget>) -> GBox {
    let row = GBox::new(Orientation::Horizontal, 12);
    let lbl = Label::new(Some(label));
    lbl.set_xalign(0.0);
    lbl.set_width_chars(16);
    row.append(&lbl);
    control.set_hexpand(true);
    row.append(control);
    row
}

fn rebuild_profiles(container: &GBox, model: &Rc<RefCell<Vec<Profile>>>) {
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }
    for (i, p) in model.borrow().iter().enumerate() {
        let card = GBox::new(Orientation::Vertical, 6);
        card.add_css_class("card");
        card.set_margin_top(6);
        card.set_margin_bottom(6);

        let header = GBox::new(Orientation::Horizontal, 8);
        let name = Entry::new();
        name.set_text(&p.name);
        name.set_hexpand(true);
        name.set_placeholder_text(Some("profile name (e.g. home)"));
        let remove = Button::with_label("Remove");
        remove.add_css_class("destructive-action");
        header.append(&name);
        header.append(&remove);
        card.append(&header);

        let networks = Entry::new();
        networks.set_text(&p.networks.join(", "));
        networks.set_placeholder_text(Some("SSID1, SSID2"));
        card.append(&field("Networks", &networks));

        let detect = Entry::new();
        detect.set_text(&p.detect_ssids.join(", "));
        detect.set_placeholder_text(Some("SSIDs that auto-select this profile"));
        card.append(&field("Detect SSIDs", &detect));

        let exit_node = Entry::new();
        exit_node.set_text(&p.exit_node);
        exit_node.set_placeholder_text(Some("tailscale exit node (optional)"));
        card.append(&field("Exit node", &exit_node));

        let bootstrap = Entry::new();
        bootstrap.set_text(&p.bootstrap);
        bootstrap.set_placeholder_text(Some("bootstrap command (optional)"));
        card.append(&field("Bootstrap", &bootstrap));

        let tailscale = Switch::new();
        tailscale.set_active(p.tailscale);
        tailscale.set_halign(gtk4::Align::Start);
        card.append(&field("Tailscale", &tailscale));

        let include_all = Switch::new();
        include_all.set_active(p.include_all_known);
        include_all.set_halign(gtk4::Align::Start);
        card.append(&field("Include all known", &include_all));

        // bind each control to the in-memory model entry
        macro_rules! bind_csv {
            ($entry:ident, $f:ident) => {{
                let model = model.clone();
                $entry.connect_changed(move |e| {
                    if let Some(p) = model.borrow_mut().get_mut(i) {
                        p.$f = e
                            .text()
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                    }
                });
            }};
        }
        macro_rules! bind_str {
            ($entry:ident, $f:ident) => {{
                let model = model.clone();
                $entry.connect_changed(move |e| {
                    if let Some(p) = model.borrow_mut().get_mut(i) {
                        p.$f = e.text().to_string();
                    }
                });
            }};
        }
        macro_rules! bind_bool {
            ($sw:ident, $f:ident) => {{
                let model = model.clone();
                $sw.connect_active_notify(move |s| {
                    if let Some(p) = model.borrow_mut().get_mut(i) {
                        p.$f = s.is_active();
                    }
                });
            }};
        }
        bind_str!(name, name);
        bind_csv!(networks, networks);
        bind_csv!(detect, detect_ssids);
        bind_str!(exit_node, exit_node);
        bind_str!(bootstrap, bootstrap);
        bind_bool!(tailscale, tailscale);
        bind_bool!(include_all, include_all_known);
        {
            let model = model.clone();
            let container = container.clone();
            remove.connect_clicked(move |_| {
                model.borrow_mut().remove(i);
                rebuild_profiles(&container, &model);
            });
        }

        container.append(&card);
    }
}

// --- view -------------------------------------------------------------------

pub fn build() -> GBox {
    let path = config_path();
    let doc = Rc::new(RefCell::new(config::load_doc(&path)));
    let nets = Rc::new(RefCell::new(read_networks(&doc.borrow())));
    let profiles = Rc::new(RefCell::new(read_profiles(&doc.borrow())));

    let outer = GBox::new(Orientation::Vertical, 8);
    outer.add_css_class("view-content");

    let title = Label::new(Some("breadcrumbs"));
    title.add_css_class("title");
    title.set_xalign(0.0);
    outer.append(&title);

    let content = GBox::new(Orientation::Vertical, 8);
    let scroll = ScrolledWindow::new();
    scroll.set_vexpand(true);
    scroll.set_hscrollbar_policy(gtk4::PolicyType::Never);
    scroll.set_child(Some(&content));
    outer.append(&scroll);

    // [settings] — edited in place on the shared doc
    content.append(&w::section("Settings"));
    content.append(&w::dropdown_row(
        "Default profile",
        &doc,
        &["settings", "default_profile"],
        &["home", "away"],
        "home",
    ));
    content.append(&w::entry_row("DNS", &doc, &["settings", "dns"], "1.1.1.1", ""));
    content.append(&w::entry_row(
        "Exit node",
        &doc,
        &["settings", "exit_node"],
        "tailscale exit node",
        "",
    ));
    content.append(&w::entry_row(
        "Ping host",
        &doc,
        &["settings", "ping_host"],
        "1.1.1.1",
        "",
    ));
    content.append(&w::entry_row(
        "Connectivity URL",
        &doc,
        &["settings", "connectivity_url"],
        "http://connectivitycheck.gstatic.com/generate_204",
        "",
    ));
    content.append(&w::spin_row(
        "nmcli wait (s)",
        &doc,
        &["settings", "nmcli_wait"],
        1.0,
        120.0,
        1.0,
        8,
    ));
    content.append(&w::spin_row(
        "Watch interval (s)",
        &doc,
        &["settings", "watch_interval"],
        1.0,
        600.0,
        1.0,
        12,
    ));

    // [[networks]]
    content.append(&w::section("Saved networks"));
    let net_list = ListBox::new();
    net_list.set_selection_mode(gtk4::SelectionMode::None);
    rebuild_networks(&net_list, &nets);
    content.append(&net_list);
    let add_net = Button::with_label("Add network");
    add_net.set_halign(gtk4::Align::Start);
    {
        let nets = nets.clone();
        let net_list = net_list.clone();
        add_net.connect_clicked(move |_| {
            nets.borrow_mut().push(Network::default());
            rebuild_networks(&net_list, &nets);
        });
    }
    content.append(&add_net);

    // [profiles.*]
    content.append(&w::section("Profiles"));
    let prof_box = GBox::new(Orientation::Vertical, 4);
    rebuild_profiles(&prof_box, &profiles);
    content.append(&prof_box);
    let add_prof = Button::with_label("Add profile");
    add_prof.set_halign(gtk4::Align::Start);
    {
        let profiles = profiles.clone();
        let prof_box = prof_box.clone();
        add_prof.connect_clicked(move |_| {
            profiles.borrow_mut().push(Profile {
                name: "new".to_string(),
                ..Default::default()
            });
            rebuild_profiles(&prof_box, &profiles);
        });
    }
    content.append(&add_prof);

    // Save — fold the network + profile editors back into the doc, then write.
    let btn_row = GBox::new(Orientation::Horizontal, 12);
    btn_row.set_margin_top(16);
    let save_btn = Button::with_label("Save");
    save_btn.add_css_class("suggested-action");
    let status = Label::new(None);
    status.add_css_class("dim-label");
    {
        let doc = doc.clone();
        let nets = nets.clone();
        let profiles = profiles.clone();
        let path = path.clone();
        let status = status.clone();
        save_btn.connect_clicked(move |_| {
            {
                let mut d = doc.borrow_mut();
                write_networks(&mut d, &nets.borrow());
                write_profiles(&mut d, &profiles.borrow());
            }
            match config::save_doc(&path, &doc.borrow()) {
                Ok(()) => {
                    status.set_text("Saved");
                    let lbl = status.clone();
                    glib::timeout_add_seconds_local(3, move || {
                        lbl.set_text("");
                        glib::ControlFlow::Break
                    });
                }
                Err(e) => status.set_text(&format!("Error: {e}")),
            }
        });
    }
    btn_row.append(&save_btn);
    btn_row.append(&status);
    outer.append(&btn_row);

    outer
}
