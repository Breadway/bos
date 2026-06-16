-- BOS Hyprland configuration — native Lua config (Hyprland 0.55+).
-- hyprlang (.conf) is deprecated; this uses the built-in `hl` API.
-- Single-file and non-modular by design. Reference: https://wiki.hypr.land/

local mod = "SUPER"

-- ---------------------------------------------------------------------------
-- Monitors — generic default that works on any hardware.
-- ---------------------------------------------------------------------------
hl.monitor({ output = "", mode = "preferred", position = "auto", scale = "auto" })

-- ---------------------------------------------------------------------------
-- Core settings.
-- ---------------------------------------------------------------------------
hl.config({
    general = {
        gaps_in          = 5,
        gaps_out         = 10,
        border_size      = 2,
        col = {
            active_border   = "rgba(88c0d0ff)",
            inactive_border = "rgba(4c566aff)",
        },
        layout           = "dwindle",
        resize_on_border = true,
    },
    decoration = {
        rounding         = 8,
        active_opacity   = 1.0,
        inactive_opacity = 1.0,
        blur = {
            enabled           = true,
            size              = 6,
            passes            = 2,
            new_optimizations = true,
        },
        shadow = {
            enabled      = true,
            range        = 12,
            render_power = 3,
        },
    },
    input = {
        kb_layout    = "us",
        follow_mouse = 1,
        touchpad     = { natural_scroll = true },
    },
    dwindle = {
        preserve_split = true,
    },
    animations = {
        enabled = true,
    },
    misc = {
        disable_hyprland_logo    = true,
        disable_splash_rendering = true,
    },
})

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
-- Standard BOS keybinds (SUPER = mod).
-- ---------------------------------------------------------------------------
-- Apps / window management
hl.bind(mod .. " + RETURN",    hl.dsp.exec_cmd("kitty"))
hl.bind(mod .. " + BACKSPACE", hl.dsp.window.close())
hl.bind(mod .. " + SPACE",     hl.dsp.exec_cmd("breadbox"))
hl.bind(mod .. " + E",         hl.dsp.exec_cmd("nautilus"))
hl.bind(mod .. " + B",         hl.dsp.exec_cmd("zen-browser"))
hl.bind(mod .. " + U",         hl.dsp.exec_cmd("breadpad"))
hl.bind(mod .. " + M",         hl.dsp.exec_cmd("breadman"))
hl.bind(mod .. " + L",         hl.dsp.exec_cmd("loginctl lock-session"))
hl.bind(mod .. " + F",         hl.dsp.window.fullscreen({ action = "toggle" }))
hl.bind(mod .. " + V",         hl.dsp.window.float({ action = "toggle" }))
hl.bind(mod .. " + SHIFT + V", hl.dsp.exec_cmd([[bash -c 'cliphist list | fzf --reverse --prompt="Clipboard > " | cliphist decode | wl-copy']]))
hl.bind(mod .. " + T",         hl.dsp.layout("togglesplit"))
hl.bind(mod .. " + Tab",       hl.dsp.focus({ urgent_or_last = true }))
hl.bind(mod .. " + N",         hl.dsp.exit())

-- Screenshots (grim + slurp + wl-clipboard)
hl.bind(mod .. " + SHIFT + S", hl.dsp.exec_cmd([[bash -c 'mkdir -p ~/Pictures/Screenshots && grim -g "$(slurp)" ~/Pictures/Screenshots/$(date +%Y%m%d-%H%M%S).png']]))
hl.bind(mod .. " + SHIFT + C", hl.dsp.exec_cmd([[bash -c 'grim -g "$(slurp)" - | wl-copy']]))
hl.bind(mod .. " + SHIFT + P", hl.dsp.exec_cmd([[bash -c 'mkdir -p ~/Pictures/Screenshots && grim ~/Pictures/Screenshots/$(date +%Y%m%d-%H%M%S).png']]))

-- Focus (directional)
hl.bind(mod .. " + left",  hl.dsp.focus({ direction = "left" }))
hl.bind(mod .. " + right", hl.dsp.focus({ direction = "right" }))
hl.bind(mod .. " + up",    hl.dsp.focus({ direction = "up" }))
hl.bind(mod .. " + down",  hl.dsp.focus({ direction = "down" }))

-- Move window (directional, vim keys)
hl.bind(mod .. " + SHIFT + h", hl.dsp.window.move({ direction = "left" }))
hl.bind(mod .. " + SHIFT + j", hl.dsp.window.move({ direction = "down" }))
hl.bind(mod .. " + SHIFT + k", hl.dsp.window.move({ direction = "up" }))
hl.bind(mod .. " + SHIFT + l", hl.dsp.window.move({ direction = "right" }))

-- Resize active window (arrows)
hl.bind(mod .. " + SHIFT + right", hl.dsp.window.resize({ x =  30, y =   0, relative = true }), { repeating = true })
hl.bind(mod .. " + SHIFT + left",  hl.dsp.window.resize({ x = -30, y =   0, relative = true }), { repeating = true })
hl.bind(mod .. " + SHIFT + up",    hl.dsp.window.resize({ x =   0, y = -30, relative = true }), { repeating = true })
hl.bind(mod .. " + SHIFT + down",  hl.dsp.window.resize({ x =   0, y =  30, relative = true }), { repeating = true })

-- Workspaces 1–10 (0 = workspace 10)
for i = 1, 10 do
    local key = tostring(i % 10)
    hl.bind(mod .. " + " .. key,           hl.dsp.focus({ workspace = i }))
    hl.bind(mod .. " + SHIFT + " .. key,   hl.dsp.window.move({ workspace = i }))
end

-- Workspace cycling
hl.bind(mod .. " + bracketright",         hl.dsp.focus({ workspace = "e+1" }))
hl.bind(mod .. " + bracketleft",          hl.dsp.focus({ workspace = "e-1" }))
hl.bind(mod .. " + SHIFT + bracketright", hl.dsp.window.move({ workspace = "e+1" }))
hl.bind(mod .. " + SHIFT + bracketleft",  hl.dsp.window.move({ workspace = "e-1" }))

-- Mouse
hl.bind(mod .. " + mouse_down", hl.dsp.focus({ workspace = "e+1" }))
hl.bind(mod .. " + mouse_up",   hl.dsp.focus({ workspace = "e-1" }))
hl.bind(mod .. " + mouse:272",  hl.dsp.window.drag(),   { mouse = true })
hl.bind(mod .. " + mouse:273",  hl.dsp.window.resize(), { mouse = true })

-- Media / hardware keys (work locked, i.e. on the lock screen too)
hl.bind("XF86AudioRaiseVolume",  hl.dsp.exec_cmd("wpctl set-volume -l 1 @DEFAULT_AUDIO_SINK@ 5%+"), { locked = true, repeating = true })
hl.bind("XF86AudioLowerVolume",  hl.dsp.exec_cmd("wpctl set-volume @DEFAULT_AUDIO_SINK@ 5%-"),       { locked = true, repeating = true })
hl.bind("XF86AudioMute",         hl.dsp.exec_cmd("wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle"),       { locked = true })
hl.bind("XF86AudioMicMute",      hl.dsp.exec_cmd("wpctl set-mute @DEFAULT_AUDIO_SOURCE@ toggle"),     { locked = true })
hl.bind("XF86MonBrightnessUp",   hl.dsp.exec_cmd("brightnessctl -e4 -n2 set 5%+"),                    { locked = true, repeating = true })
hl.bind("XF86MonBrightnessDown", hl.dsp.exec_cmd("brightnessctl -e4 -n2 set 5%-"),                    { locked = true, repeating = true })
hl.bind("XF86AudioNext",         hl.dsp.exec_cmd("playerctl next"),                                   { locked = true })
hl.bind("XF86AudioPrev",         hl.dsp.exec_cmd("playerctl previous"),                               { locked = true })
hl.bind("XF86AudioPlay",         hl.dsp.exec_cmd("playerctl play-pause"),                             { locked = true })

-- ---------------------------------------------------------------------------
-- Autostart. polkit agent + the bread ecosystem + idle daemon + wallpaper.
-- (bos-live-setup appends the live-installer launch below this on the ISO.)
-- ---------------------------------------------------------------------------
hl.on("hyprland.start", function()
    local startup = {
        -- Global dark theme: GTK4/libadwaita + GTK3 theme + icon + cursor.
        "gsettings set org.gnome.desktop.interface color-scheme prefer-dark",
        "gsettings set org.gnome.desktop.interface gtk-theme Adwaita-dark",
        "gsettings set org.gnome.desktop.interface icon-theme Papirus-Dark",
        "gsettings set org.gnome.desktop.interface cursor-theme Bibata-Modern-Ice",
        "gsettings set org.gnome.desktop.interface cursor-size 24",
        -- Clipboard history daemon (feeds SUPER+V history picker via wl-paste).
        "wl-paste --type text --watch cliphist store",
        "/usr/lib/polkit-gnome/polkit-gnome-authentication-agent-1",
        "awww-daemon",
        -- set the default wallpaper once the daemon is up (retry until ready)
        [[bash -c 'until awww img /usr/share/backgrounds/bos/bread-background.png 2>/dev/null; do sleep 0.3; done']],
        "breadd",
        "breadbar",
        "breadbox-sync",
        "hypridle",
    }
    for _, cmd in ipairs(startup) do
        hl.dispatch(hl.dsp.exec_cmd(cmd))
    end
end)
