mod cli;
mod config;
mod content;
mod services;
mod theme;
mod ui;

use gtk4::gio::ApplicationFlags;
use gtk4::prelude::*;

fn main() {
    // GApplication requires a dotted id (g_application_id_is_valid rejects a
    // bare "breadhelp" with a GLib-GIO-CRITICAL and silently skips setting
    // it, which would break D-Bus single-instance activation). The Wayland
    // app-id GTK reports is this exact string — the Hyprland window rule and
    // .desktop's StartupWMClass must match it verbatim.
    //
    // HANDLES_COMMAND_LINE: `breadhelp --onboard` while already running must
    // reach the running (primary) instance's argv, not just re-activate it
    // with no arguments — that's what lets a second launch re-trigger the
    // onboarding tour instead of only focusing the window.
    let app = gtk4::Application::builder()
        .application_id("com.breadway.breadhelp")
        .flags(ApplicationFlags::HANDLES_COMMAND_LINE)
        .build();

    app.connect_command_line(|app, cmdline| {
        let action = cli::parse(&cmdline.arguments());
        ui::window::present(app, action);
        glib::ExitCode::SUCCESS
    });

    // Best-effort revert of a tour step's temporarily-rebound keybind (see
    // `ui::tour`'s crash-safety notes) on a clean exit — `rebind_temp` is a
    // detached fire-and-forget `hyprctl` spawn, so it still completes even
    // though this process is exiting right after.
    app.connect_shutdown(|_| {
        ui::tour::self_heal();
    });

    app.run();
}
