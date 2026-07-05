//! argv handling for a `HANDLES_COMMAND_LINE` GApplication — every
//! invocation (including ones launched while breadhelp is already running,
//! forwarded over D-Bus to the primary instance) reaches
//! `connect_command_line`, so `--onboard` on a second launch re-triggers the
//! tour in the existing window instead of spawning a duplicate.

#[derive(Clone, Default)]
pub struct Action {
    /// Force-restart the onboarding tour from step 0, regardless of whether
    /// it was already completed or in progress.
    pub force_onboard: bool,
    /// Set by hyprland.lua's every-login autostart entry, as opposed to a
    /// user explicitly running `breadhelp` or pressing SUPER+/. Lets
    /// `present()` build the window (so the app is ready to respond the
    /// instant it's needed) without popping it open when onboarding is
    /// already done — autostart should only be visible on a genuine first
    /// run, never on every subsequent login.
    pub autostart: bool,
    /// From a breadd Lua module (e.g. `breadhelp-suggest.lua`) reacting to a
    /// system event. Resolved to banner text by `services::breadd::resolve`.
    pub suggest: Option<String>,
}

pub fn parse(args: &[std::ffi::OsString]) -> Action {
    let mut action = Action::default();
    let mut it = args.iter().skip(1);
    while let Some(arg) = it.next() {
        if arg == "--onboard" {
            action.force_onboard = true;
        } else if arg == "--autostart" {
            action.autostart = true;
        } else if arg == "--suggest" {
            action.suggest = it.next().and_then(|s| s.to_str()).map(str::to_string);
        }
    }
    action
}
