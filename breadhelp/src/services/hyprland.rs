use std::process::Command;

/// Runtime-only rebind via `hyprctl keyword bind`: takes effect immediately
/// and resets on the next Hyprland reload/login. `hyprland.lua` can't safely
/// be rewritten line-by-line by a program (hand-authored Lua, arbitrary
/// formatting), so this is the only rebind path — callers must label it
/// clearly as temporary in the UI.
///
/// `bind_value` is the full `<mods>,<key>,<dispatcher>,<args>` string, e.g.
/// `"SUPER,U,exec,breadpad"`.
pub fn rebind_temp(bind_value: &str) {
    if let Err(e) = Command::new("hyprctl").args(["keyword", "bind", bind_value]).spawn() {
        eprintln!("breadhelp: hyprctl rebind failed: {e}");
    }
}
