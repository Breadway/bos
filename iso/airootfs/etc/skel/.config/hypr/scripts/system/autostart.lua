-- scripts/system/autostart.lua — loads the *extra* (user-toggleable)
-- autostart list from autostart.json. Returns just the enabled commands, in
-- order. The core bootstrap sequence (theme generation, dark-mode gsettings,
-- polkit agent, wallpaper daemon, breadd's Wayland-env fix, breadclipd) is
-- deliberately NOT exposed here — it's timing/order-sensitive infrastructure,
-- not something a settings UI should let a user disable, so it stays
-- hardcoded in hyprland.lua itself.
--
-- Failsafe: unlike monitors.json, an empty or all-disabled result here is a
-- legitimate user choice (they don't want breadbar/hypridle/etc), so this
-- only falls back to defaults on a missing/malformed file — never just
-- because the valid result happens to be empty.
local json = dofile(os.getenv("HOME") .. "/.config/hypr/scripts/lib/json.lua")

local DEFAULT_EXTRA = {
    { command = "breadbar", enabled = true },
    { command = "hypridle", enabled = true },
    { command = "bos-netcheck", enabled = true },
    { command = "breadhelp --autostart", enabled = true },
}

return function()
    local path = os.getenv("HOME") .. "/.config/hypr/autostart.json"
    local parsed = json.load(path)
    local entries
    if type(parsed) ~= "table" or type(parsed.extra) ~= "table" then
        entries = DEFAULT_EXTRA
    else
        entries = parsed.extra
    end

    local commands = {}
    for _, entry in ipairs(entries) do
        if type(entry) == "table" and type(entry.command) == "string" and entry.command ~= "" then
            local enabled = entry.enabled
            if type(enabled) ~= "boolean" then
                enabled = true
            end
            if enabled then
                commands[#commands + 1] = entry.command
            end
        end
    end
    return commands
end
