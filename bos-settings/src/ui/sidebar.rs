use gtk4::prelude::*;
use gtk4::{Box as GBox, Image, Label, ListBox, ListBoxRow, Orientation};

pub struct SidebarItem {
    /// Must match the `Stack` page name registered in `window.rs`.
    pub id: &'static str,
    pub label: &'static str,
    /// Dim second line — the underlying binary/config name, for items whose
    /// human label doesn't already make that obvious.
    pub sublabel: Option<&'static str>,
    /// A `-symbolic` icon name from the system icon theme (Papirus-Dark ships
    /// the full Adwaita-compatible symbolic set this app relies on).
    pub icon: &'static str,
}

const fn item(id: &'static str, label: &'static str, icon: &'static str) -> SidebarItem {
    SidebarItem { id, label, sublabel: None, icon }
}

const fn item_sub(
    id: &'static str,
    label: &'static str,
    sublabel: &'static str,
    icon: &'static str,
) -> SidebarItem {
    SidebarItem { id, label, sublabel: Some(sublabel), icon }
}

// Grouped by task, not by "app vs system internals" — a user thinks "I want
// to change my Wi-Fi" or "I want to change my wallpaper", not "which of
// these is a bread-ecosystem app". breadcrumbs (Wi-Fi profiles) moves out of
// the old "Apps" bucket into System for the same reason.
pub const SYSTEM_ITEMS: &[SidebarItem] = &[
    item("network", "Network", "network-wireless-symbolic"),
    item_sub("breadcrumbs", "Wi-Fi Profiles", "breadcrumbs", "network-workgroup-symbolic"),
    item("sound", "Sound", "audio-volume-high-symbolic"),
    item("datetime", "Date & Time", "preferences-system-time-symbolic"),
    item_sub("hyprland", "Display", "hyprland.lua", "video-display-symbolic"),
];

pub const PERSONALIZATION_ITEMS: &[SidebarItem] = &[
    item_sub("breadpaper", "Wallpaper", "breadpaper", "preferences-desktop-wallpaper-symbolic"),
    item_sub("breadbar", "Bar", "breadbar", "view-grid-symbolic"),
    item_sub("breadbox", "Launcher", "breadbox", "view-app-grid-symbolic"),
    item_sub("breadclip", "Clipboard", "breadclipd", "edit-paste-symbolic"),
    item_sub("breadpad", "Notes", "breadpad", "text-editor-symbolic"),
    item_sub("breadsearch", "File Search", "breadsearch", "system-search-symbolic"),
    item_sub("bread", "Daemon", "breadd", "applications-system-symbolic"),
];

pub const MAINTENANCE_ITEMS: &[SidebarItem] = &[
    item("packages", "Packages", "package-x-generic-symbolic"),
    item("snapshots", "Snapshots", "document-open-recent-symbolic"),
];

pub const ABOUT_ITEMS: &[SidebarItem] = &[item("about", "About", "help-about-symbolic")];

pub fn build() -> (GBox, ListBox) {
    let vbox = GBox::new(Orientation::Vertical, 0);
    vbox.add_css_class("sidebar");
    vbox.set_width_request(210);

    let list = ListBox::new();
    list.set_selection_mode(gtk4::SelectionMode::Single);
    list.add_css_class("sidebar");

    append_section(&list, "System", SYSTEM_ITEMS);
    append_section(&list, "Personalization", PERSONALIZATION_ITEMS);
    append_section(&list, "Maintenance", MAINTENANCE_ITEMS);
    append_section(&list, None, ABOUT_ITEMS);

    // Select About so it matches the default stack page.
    let mut i = 0;
    loop {
        match list.row_at_index(i) {
            None => break,
            Some(row) if row.widget_name() == "about" => {
                list.select_row(Some(&row));
                break;
            }
            _ => i += 1,
        }
    }

    vbox.append(&list);
    (vbox, list)
}

fn append_section(list: &ListBox, title: impl Into<Option<&'static str>>, items: &[SidebarItem]) {
    if let Some(title) = title.into() {
        let header_row = ListBoxRow::new();
        header_row.set_selectable(false);
        header_row.set_activatable(false);
        let header_lbl = Label::new(Some(title));
        header_lbl.add_css_class("section-header");
        header_lbl.set_xalign(0.0);
        header_row.set_child(Some(&header_lbl));
        list.append(&header_row);
    }

    for entry in items {
        let row = ListBoxRow::new();
        row.set_widget_name(entry.id);

        let hbox = GBox::new(Orientation::Horizontal, 10);
        hbox.set_margin_top(4);
        hbox.set_margin_bottom(4);

        let icon = Image::from_icon_name(entry.icon);
        icon.set_pixel_size(16);
        hbox.append(&icon);

        let labels = GBox::new(Orientation::Vertical, 0);
        let lbl = Label::new(Some(entry.label));
        lbl.set_xalign(0.0);
        labels.append(&lbl);
        if let Some(sub) = entry.sublabel {
            let sub_lbl = Label::new(Some(sub));
            sub_lbl.add_css_class("dim-label");
            sub_lbl.set_xalign(0.0);
            // Match the sidebar's smaller "section-header" scale rather than
            // the shared sheet's default dim-label size, so it reads as a
            // caption under the row label, not a second full-size label.
            sub_lbl.add_css_class("caption");
            labels.append(&sub_lbl);
        }
        hbox.append(&labels);

        row.set_child(Some(&hbox));
        list.append(&row);
    }
}
