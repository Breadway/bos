-- scripts/lib/paths.lua: the one place that resolves the Hyprland config dir.
--
-- Honours XDG_CONFIG_HOME (when set to an absolute path, per the XDG Base
-- Directory spec; empty or relative values are ignored) and falls back to
-- $HOME/.config otherwise. Every loader under hypr/ asks this module for the
-- directory instead of building "$HOME/.config/hypr" itself, so settings.json,
-- monitors.json, binds.json, autostart.json and layerrules.json are read from
-- the same place bos-settings writes them (dirs::config_dir()).
--
-- `getenv` is injectable so the rule can be checked without a real environment
-- (see scripts/test-hypr-config-home.lua).
local M = {}

function M.config_home(getenv)
    getenv = getenv or os.getenv
    local xdg = getenv("XDG_CONFIG_HOME")
    if xdg and xdg:sub(1, 1) == "/" then
        return (xdg:gsub("/+$", ""))
    end
    return getenv("HOME") .. "/.config"
end

-- Trailing slash included, so callers can write hypr_dir() .. "settings.json".
function M.hypr_dir(getenv)
    return M.config_home(getenv) .. "/hypr/"
end

return M
