-- BOS Hyprland configuration — native Lua config (Hyprland 0.55+).
-- hyprlang (.conf) is deprecated; this uses the built-in `hl` API.
-- Mostly single-file by design (reference: https://wiki.hypr.land/) — the
-- exceptions are keybinds, appearance settings, monitor layout, and the
-- extra autostart list, each loaded from a JSON file (binds.json,
-- settings.json, monitors.json, autostart.json) so bread* apps can read/edit
-- them as structured data instead of parsing this Lua file.
--
-- Every loader below is wrapped in `pcall`: this file is loaded as a single
-- Lua chunk, so an uncaught error partway through would abort everything
-- *after* it too (no keybinds, no window rules, no autostart) — a bad or
-- hand-edited JSON file must degrade to that one section's hardcoded
-- defaults, never take down the rest of the session.
local script_dir = os.getenv("HOME") .. "/.config/hypr/scripts/"
local config_home = os.getenv("HOME") .. "/.config/hypr/"

-- ---------------------------------------------------------------------------
-- Monitors — from monitors.json, always falls back to a generic
-- any-hardware default (see scripts/display/monitors.lua).
-- ---------------------------------------------------------------------------
pcall(dofile, script_dir .. "display/monitors.lua")

-- ---------------------------------------------------------------------------
-- Core appearance/input settings — from settings.json (see
-- scripts/ui/settings.lua).
-- ---------------------------------------------------------------------------
pcall(dofile, script_dir .. "ui/settings.lua")

-- ---------------------------------------------------------------------------
-- Animations — snappy curves + per-leaf speeds (matches the reference laptop;
-- the hl.config default above is much slower).
-- ---------------------------------------------------------------------------
local curves = {
    easeOutQuint   = { type = "bezier", points = { { 0.23, 1 },    { 0.32, 1 } } },
    easeInOutCubic = { type = "bezier", points = { { 0.65, 0.05 }, { 0.36, 1 } } },
    almostLinear   = { type = "bezier", points = { { 0.5,  0.5 },  { 0.75, 1 } } },
    quick          = { type = "bezier", points = { { 0.15, 0 },    { 0.1,  1 } } },
}
for name, curve in pairs(curves) do
    hl.curve(name, curve)
end

local animations = {
    { leaf = "global",     enabled = true, speed = 10,   bezier = "default" },
    { leaf = "border",     enabled = true, speed = 5.39, bezier = "easeOutQuint" },
    { leaf = "windows",    enabled = true, speed = 4.79, bezier = "easeOutQuint" },
    { leaf = "windowsIn",  enabled = true, speed = 4.1,  bezier = "easeOutQuint", style = "popin 87%" },
    { leaf = "windowsOut", enabled = true, speed = 1.49, bezier = "linear",       style = "popin 87%" },
    { leaf = "fade",       enabled = true, speed = 3.03, bezier = "quick" },
    { leaf = "layers",     enabled = true, speed = 3.81, bezier = "easeOutQuint" },
    { leaf = "workspaces", enabled = true, speed = 1.94, bezier = "almostLinear", style = "fade" },
}
for _, animation in ipairs(animations) do
    hl.animation(animation)
end

-- ---------------------------------------------------------------------------
-- Window rules — float + centre the onboarding/help popups.
-- breadhelp replaces the old bos-welcome/bos-keybinds kitty+less popups.
-- bos-netsetup (nmtui, from bos-netcheck) is unrelated to breadhelp and
-- still floats the same way.
-- ---------------------------------------------------------------------------
hl.window_rule({ name = "breadhelp",    match = { class = "^(com\\.breadway\\.breadhelp)$" }, float = true, size = { 880, 600 } })
hl.window_rule({ name = "bos-netsetup", match = { class = "^(bos-netsetup)$" }, float = true, size = { 700, 560 } })

-- ---------------------------------------------------------------------------
-- Environment (vendor-neutral; no GPU-specific vars so it works on Intel/AMD).
-- ---------------------------------------------------------------------------
hl.env("XCURSOR_SIZE", "24")
hl.env("HYPRCURSOR_SIZE", "24")
hl.env("XCURSOR_THEME", "Bibata-Modern-Ice")
hl.env("MOZ_ENABLE_WAYLAND", "1")
hl.env("QT_QPA_PLATFORM", "wayland;xcb")
hl.env("QT_QPA_PLATFORMTHEME", "qt5ct")
hl.env("QT_WAYLAND_DISABLE_WINDOWDECORATION", "1")
hl.env("SDL_VIDEODRIVER", "wayland")
hl.env("ELECTRON_OZONE_PLATFORM_HINT", "auto")
hl.env("_JAVA_AWT_WM_NONREPARENTING", "1")

-- kitty sets its own background_opacity (see kitty.conf), so the global blur
-- above blurs behind the terminal while keeping text fully opaque.

-- ---------------------------------------------------------------------------
-- Standard BOS keybinds — data-driven from binds.json (apps, windows,
-- screenshots, focus/move/resize, workspaces, mouse, media keys), loaded via
-- scripts/input/{binds,keybinds}.lua. breadhelp's keybind viewer reads
-- binds.json directly as its source of truth (no separately-maintained
-- cheatsheet to keep in sync), and a future bos-settings editor can
-- read/write the same file.
pcall(function()
    local binds = dofile(script_dir .. "input/binds.lua")(config_home .. "binds.json")
    dofile(script_dir .. "input/keybinds.lua")({
        default_mods = binds.default_mods,
        bindings     = binds.bindings,
    })
end)
-- 3-finger horizontal trackpad swipe → workspace switch (1:1 gesture)
hl.gesture({
    fingers   = 3,
    direction = "horizontal",
    action    = "workspace",
})

-- ---------------------------------------------------------------------------
-- Autostart. Core bootstrap sequence (polkit agent, dark theme, wallpaper
-- daemon, breadd's Wayland-env fix, breadclipd) stays hardcoded here — it's
-- timing/order-sensitive infrastructure, not something a settings UI should
-- expose for a user to disable or reorder. The extra, genuinely toggleable
-- apps (breadbar, hypridle, bos-netcheck, breadhelp) come from
-- autostart.json via scripts/system/autostart.lua, appended after.
-- (bos-live-setup appends the live-installer launch below this on the ISO.)
-- ---------------------------------------------------------------------------
hl.on("hyprland.start", function()
    local core_startup = {
        -- Generate the shared bread GUI stylesheet first, so breadbar/breadbox/
        -- bos-settings load it on start (they also live-reload if it changes).
        "bread-theme generate",
        -- Global dark theme: GTK4/libadwaita + GTK3 theme + icon + cursor.
        "gsettings set org.gnome.desktop.interface color-scheme prefer-dark",
        "gsettings set org.gnome.desktop.interface gtk-theme Adwaita-dark",
        "gsettings set org.gnome.desktop.interface icon-theme Papirus-Dark",
        "gsettings set org.gnome.desktop.interface cursor-theme Bibata-Modern-Ice",
        "gsettings set org.gnome.desktop.interface cursor-size 24",
        -- Clipboard history is breadclipd, a bakery-managed systemd --user
        -- service (auto-started via skel — see build-local.sh's service bake)
        -- rather than an exec-once here.
        "/usr/lib/polkit-gnome/polkit-gnome-authentication-agent-1",
        "awww-daemon",
        -- Set the default wallpaper once the daemon is up (retry until ready).
        -- Raw `awww img`, NOT `breadpaper set` — breadpaper set also runs real
        -- pywal against the image, which would clobber the curated black-base
        -- colors.json baked into skel (.cache/wal/colors.json: #0c0c0c bg,
        -- bread-toned browns reserved for accent slots only) with colors
        -- actually extracted from bread-background.png — which is an all-beige
        -- photo, so every bread-theme app (breadbar included) turns brown.
        -- `breadpaper get` still works on a fresh install without ever running
        -- pywal: .cache/wal/wal (pywal's own "last image" marker, which is all
        -- breadpaper reads) is baked into skel too, right beside colors.json.
        -- pywal only runs for real once the user picks a wallpaper themselves.
        [[bash -c 'until awww img /usr/share/backgrounds/bos/bread-background.png 2>/dev/null; do sleep 0.3; done']],
        -- breadd runs as a systemd user service (~/.config/systemd/user/breadd.service,
        -- enabled in skel). It autostarts at login but before Hyprland exists, so
        -- push the compositor's Wayland env into the user manager and restart breadd
        -- to pick it up — that's how it gets HYPRLAND_INSTANCE_SIGNATURE to talk to Hyprland.
        "dbus-update-activation-environment --systemd WAYLAND_DISPLAY XDG_CURRENT_DESKTOP HYPRLAND_INSTANCE_SIGNATURE",
        "systemctl --user restart breadd",
        -- graphical-session.target ships with RefuseManualStart=yes (systemd
        -- convention — only a session manager like uwsm is meant to activate
        -- it), confirmed on real hardware: `systemctl --user start
        -- graphical-session.target` fails outright ("Operation refused").
        -- BOS doesn't use uwsm, and nothing else activates that target, so
        -- breadclipd (WantedBy=graphical-session.target) never started.
        -- Start it directly instead. If more graphical-session.target
        -- services show up later, add them here too.
        "systemctl --user start breadclipd.service",
    }
    for _, cmd in ipairs(core_startup) do
        hl.dispatch(hl.dsp.exec_cmd(cmd))
    end

    -- breadbox-sync is a Type=oneshot systemd --user service
    -- (WantedBy=default.target, no Hyprland IPC dependency) — it already
    -- runs on login via the unit baked into skel, independent of this list.
    local ok, extra = pcall(function()
        return dofile(script_dir .. "system/autostart.lua")()
    end)
    if not ok or type(extra) ~= "table" then
        -- autostart.json/its loader broke — fall back to the same apps BOS
        -- has always started, so a bad JSON edit degrades to "normal
        -- desktop" rather than "no bar, no idle lock, no onboarding".
        extra = { "breadbar", "hypridle", "bos-netcheck", "breadhelp --autostart" }
    end
    for _, cmd in ipairs(extra) do
        hl.dispatch(hl.dsp.exec_cmd(cmd))
    end
end)
