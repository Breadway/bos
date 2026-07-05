use gtk4::prelude::*;
use gtk4::{Label, ListBox, ListBoxRow, Orientation, Paned, ScrolledWindow, SelectionMode, Stack, Widget};

use crate::config::Mode;
use crate::content::{keybinds::Keybind, ContentStore};

use super::guide_view;

pub fn build(store: &ContentStore, binds: &[Keybind], mode: Mode) -> Widget {
    let hpaned = Paned::new(Orientation::Horizontal);
    hpaned.set_position(220);
    hpaned.set_shrink_start_child(false);
    hpaned.set_resize_start_child(false);

    let list = ListBox::new();
    list.set_selection_mode(SelectionMode::Single);
    list.add_css_class("sidebar");

    let stack = Stack::new();
    stack.set_hexpand(true);
    stack.set_vexpand(true);

    let mut first_page: Option<String> = None;

    for category in store.categories() {
        let header_row = ListBoxRow::new();
        header_row.set_selectable(false);
        header_row.set_activatable(false);
        let header_lbl = Label::new(Some(&display_category(&category)));
        header_lbl.add_css_class("section-header");
        header_lbl.set_xalign(0.0);
        header_row.set_child(Some(&header_lbl));
        list.append(&header_row);

        for guide in store.guides_in(&category) {
            let page_name = format!("{}/{}", guide.category, guide.id);

            let row = ListBoxRow::new();
            row.set_widget_name(&page_name);
            let lbl = Label::new(Some(&guide.meta.title));
            lbl.set_xalign(0.0);
            lbl.set_margin_top(4);
            lbl.set_margin_bottom(4);
            lbl.set_margin_start(8);
            row.set_child(Some(&lbl));
            list.append(&row);

            let view = guide_view::build(store, guide, mode.is_simplified(), binds);
            stack.add_named(&view, Some(&page_name));
            if first_page.is_none() {
                first_page = Some(page_name);
            }
        }
    }

    if let Some(first) = &first_page {
        stack.set_visible_child_name(first);
        let mut i = 0;
        loop {
            match list.row_at_index(i) {
                None => break,
                Some(row) if row.widget_name() == *first => {
                    list.select_row(Some(&row));
                    break;
                }
                _ => i += 1,
            }
        }
    }

    {
        let stack = stack.clone();
        list.connect_row_selected(move |_, row| {
            if let Some(row) = row {
                let name = row.widget_name();
                if !name.is_empty() {
                    stack.set_visible_child_name(&name);
                }
            }
        });
    }

    let sidebar_scroller = ScrolledWindow::builder().child(&list).width_request(220).build();
    hpaned.set_start_child(Some(&sidebar_scroller));
    hpaned.set_end_child(Some(&stack));

    hpaned.upcast()
}

fn display_category(id: &str) -> String {
    id.split('-').map(capitalize).collect::<Vec<_>>().join(" ")
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}
