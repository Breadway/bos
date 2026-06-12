use gtk4::prelude::*;
use gtk4::{Box as GBox, Label, ListBox, ListBoxRow, Orientation, Separator};

pub struct SidebarItem {
    pub id: &'static str,
    pub label: &'static str,
}

pub const APPS_ITEMS: &[SidebarItem] = &[
    SidebarItem { id: "bread", label: "bread" },
    SidebarItem { id: "breadbar", label: "breadbar" },
    SidebarItem { id: "breadbox", label: "breadbox" },
    SidebarItem { id: "breadcrumbs", label: "breadcrumbs" },
    SidebarItem { id: "breadpad", label: "breadpad" },
];

pub const SYSTEM_ITEMS: &[SidebarItem] = &[
    SidebarItem { id: "snapshots", label: "Snapshots" },
    SidebarItem { id: "packages", label: "Packages" },
    SidebarItem { id: "hyprland", label: "Hyprland" },
];

pub fn build() -> (GBox, ListBox) {
    let vbox = GBox::new(Orientation::Vertical, 0);
    vbox.add_css_class("sidebar");
    vbox.set_width_request(190);

    let list = ListBox::new();
    list.set_selection_mode(gtk4::SelectionMode::Single);
    list.add_css_class("sidebar");

    append_section(&list, "Apps", APPS_ITEMS);
    append_section(&list, "System", SYSTEM_ITEMS);

    // Select first item by default
    if let Some(first) = list.row_at_index(1) {
        list.select_row(Some(&first));
    }

    vbox.append(&list);
    (vbox, list)
}

fn append_section(list: &ListBox, title: &str, items: &[SidebarItem]) {
    // Section header (non-selectable)
    let header_row = ListBoxRow::new();
    header_row.set_selectable(false);
    header_row.set_activatable(false);
    let header_lbl = Label::new(Some(title));
    header_lbl.add_css_class("section-header");
    header_lbl.set_xalign(0.0);
    header_row.set_child(Some(&header_lbl));
    list.append(&header_row);

    for item in items {
        let row = ListBoxRow::new();
        row.set_widget_name(item.id);

        let lbl = Label::new(Some(item.label));
        lbl.set_xalign(0.0);
        lbl.set_margin_top(2);
        lbl.set_margin_bottom(2);
        row.set_child(Some(&lbl));
        list.append(&row);
    }
}
