-- scripts/ui/settings.lua — loads settings.json over hardcoded defaults and
-- applies via hl.config(). Failsafe in two layers:
--   1. every leaf value is type-checked against its default individually, so
--      one bad field (wrong type, typo) falls back to just that field, not
--      the whole file;
--   2. the actual hl.config() application is pcall'd — if Hyprland itself
--      rejects a well-typed-but-semantically-bad value, we fall back to
--      re-applying pure hardcoded defaults, so the session always comes up
--      with a normal, usable layout instead of erroring out mid-config-load.
local json = dofile(os.getenv("HOME") .. "/.config/hypr/scripts/lib/json.lua")

local DEFAULTS = {
    gaps_in = 5,
    gaps_out = 10,
    border_size = 2,
    active_border = "rgba(88c0d0ff)",
    inactive_border = "rgba(4c566aff)",
    layout = "dwindle",
    resize_on_border = true,
    rounding = 8,
    blur_enabled = true,
    blur_size = 6,
    blur_passes = 2,
    shadow_enabled = true,
    shadow_range = 12,
    shadow_render_power = 3,
    kb_layout = "us",
    follow_mouse = 1,
    natural_scroll = true,
}

local function num(v, fallback)
    if type(v) == "number" then return v end
    return fallback
end

local function bool(v, fallback)
    if type(v) == "boolean" then return v end
    return fallback
end

local function str(v, fallback)
    if type(v) == "string" and v ~= "" then return v end
    return fallback
end

local function load_overrides()
    local path = os.getenv("HOME") .. "/.config/hypr/settings.json"
    local parsed = json.load(path)
    if type(parsed) ~= "table" then
        return {}
    end
    return parsed
end

local o = load_overrides()

local merged = {
    gaps_in             = num(o.gaps_in, DEFAULTS.gaps_in),
    gaps_out            = num(o.gaps_out, DEFAULTS.gaps_out),
    border_size         = num(o.border_size, DEFAULTS.border_size),
    active_border       = str(o.active_border, DEFAULTS.active_border),
    inactive_border     = str(o.inactive_border, DEFAULTS.inactive_border),
    layout              = str(o.layout, DEFAULTS.layout),
    resize_on_border    = bool(o.resize_on_border, DEFAULTS.resize_on_border),
    rounding            = num(o.rounding, DEFAULTS.rounding),
    blur_enabled        = bool(o.blur_enabled, DEFAULTS.blur_enabled),
    blur_size           = num(o.blur_size, DEFAULTS.blur_size),
    blur_passes         = num(o.blur_passes, DEFAULTS.blur_passes),
    shadow_enabled      = bool(o.shadow_enabled, DEFAULTS.shadow_enabled),
    shadow_range        = num(o.shadow_range, DEFAULTS.shadow_range),
    shadow_render_power = num(o.shadow_render_power, DEFAULTS.shadow_render_power),
    kb_layout           = str(o.kb_layout, DEFAULTS.kb_layout),
    follow_mouse        = num(o.follow_mouse, DEFAULTS.follow_mouse),
    natural_scroll      = bool(o.natural_scroll, DEFAULTS.natural_scroll),
}

local function build_hl_config(v)
    return {
        general = {
            gaps_in = v.gaps_in,
            gaps_out = v.gaps_out,
            border_size = v.border_size,
            col = {
                active_border = v.active_border,
                inactive_border = v.inactive_border,
            },
            layout = v.layout,
            resize_on_border = v.resize_on_border,
        },
        decoration = {
            rounding = v.rounding,
            active_opacity = 1.0,
            inactive_opacity = 1.0,
            blur = {
                enabled = v.blur_enabled,
                size = v.blur_size,
                passes = v.blur_passes,
                new_optimizations = true,
            },
            shadow = {
                enabled = v.shadow_enabled,
                range = v.shadow_range,
                render_power = v.shadow_render_power,
            },
        },
        input = {
            kb_layout = v.kb_layout,
            follow_mouse = v.follow_mouse,
            touchpad = { natural_scroll = v.natural_scroll },
        },
        dwindle = { preserve_split = true },
        animations = { enabled = true },
        misc = { disable_hyprland_logo = true, disable_splash_rendering = true },
    }
end

local ok = pcall(hl.config, build_hl_config(merged))
if not ok then
    pcall(hl.config, build_hl_config(DEFAULTS))
end
