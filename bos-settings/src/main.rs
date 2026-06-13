mod config;
mod theme;
mod ui;

use gtk4::prelude::*;

fn main() {
    let app = gtk4::Application::builder()
        .application_id("com.breadway.bos-settings")
        .build();
    app.connect_activate(ui::window::build_ui);
    app.run();
}
