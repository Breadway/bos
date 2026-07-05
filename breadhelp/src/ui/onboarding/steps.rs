pub struct Step {
    pub title: &'static str,
    pub body: &'static str,
    /// What the step's "Try it" button runs, if any.
    pub demo_cmd: Option<&'static str>,
}

pub fn steps() -> &'static [Step] {
    &[
        Step {
            title: "Welcome to BOS",
            body: "You're running a complete Hyprland desktop with the bread ecosystem preinstalled. This quick tour covers the essentials — skip any time, and reopen it later with `breadhelp --onboard`.",
            demo_cmd: None,
        },
        Step {
            title: "The SUPER key",
            body: "SUPER (the Windows/Cmd key) is how almost everything starts in BOS. Hold it down, then press another key.",
            demo_cmd: None,
        },
        Step {
            title: "Essential keybinds",
            body: "SUPER + Space opens the app launcher. SUPER + Return opens a terminal. SUPER + / always shows the full keybind cheatsheet, right here in this app.",
            demo_cmd: Some("breadbox"),
        },
        Step {
            title: "Opening apps",
            body: "breadbox (launcher), breadpad (notes), breadman (tasks), and BOS Settings are all one SUPER shortcut away.",
            demo_cmd: Some("bos-settings"),
        },
        Step {
            title: "Make it yours",
            body: "Pick a wallpaper and the whole desktop recolors itself — bar, launcher, notes, and this help center all pick up the same palette.",
            demo_cmd: Some("breadpaper"),
        },
        Step {
            title: "Internet & updates",
            body: "bos-update refreshes both the system and the bread ecosystem in one go.",
            demo_cmd: Some("bos-update"),
        },
        Step {
            title: "Backups & safety",
            body: "Every package change snapshots your system automatically. If an update breaks something, roll it back from BOS Settings or pick a snapshot right from the GRUB menu at boot.",
            demo_cmd: None,
        },
        Step {
            title: "You're ready!",
            body: "That's everything to get started. Press SUPER + / any time you forget a shortcut.",
            demo_cmd: None,
        },
    ]
}
