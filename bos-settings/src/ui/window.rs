use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Orientation, Paned, Stack};

use super::sidebar;
use super::views;

pub fn build_ui(app: &Application) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("BOS Settings")
        .default_width(960)
        .default_height(640)
        .build();

    crate::theme::load(&WidgetExt::display(&window));

    let hpaned = Paned::new(Orientation::Horizontal);
    hpaned.set_position(190);
    hpaned.set_shrink_start_child(false);
    hpaned.set_resize_start_child(false);

    let (sidebar_box, list) = sidebar::build();

    let stack = Stack::new();
    stack.set_hexpand(true);
    stack.set_vexpand(true);

    stack.add_named(&views::snapshots::build(), Some("snapshots"));
    stack.add_named(&views::packages::build(), Some("packages"));
    stack.add_named(&views::bread::build(), Some("bread"));
    stack.add_named(&views::breadbar::build(), Some("breadbar"));
    stack.add_named(&views::breadbox::build(), Some("breadbox"));
    stack.add_named(&views::breadcrumbs::build(), Some("breadcrumbs"));
    stack.add_named(&views::breadpad::build(), Some("breadpad"));
    stack.add_named(&views::hyprland::build(), Some("hyprland"));

    // Default to snapshots view
    stack.set_visible_child_name("snapshots");

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

    hpaned.set_start_child(Some(&sidebar_box));
    hpaned.set_end_child(Some(&stack));

    window.set_child(Some(&hpaned));
    window.present();
}
