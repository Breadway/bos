-- scripts/display/monitors.lua — loads monitors.json (an array of monitor
-- rules) and applies each via hl.monitor(). Failsafe: an empty or entirely
-- invalid monitors.json is treated the same as a missing one — falling back
-- to the single generic wildcard rule — because applying *zero* monitor
-- rules risks an unconfigured/black-screen session, unlike settings.json or
-- autostart.json where "apply nothing extra" is a legitimate user choice.
local json = dofile(os.getenv("HOME") .. "/.config/hypr/scripts/lib/json.lua")

local DEFAULT_MONITORS = {
    { output = "", mode = "preferred", position = "auto", scale = "auto" },
}

local function valid_entry(e)
    return type(e) == "table" and type(e.output) == "string"
end

local function load_monitors()
    local path = os.getenv("HOME") .. "/.config/hypr/monitors.json"
    local parsed = json.load(path)
    if type(parsed) ~= "table" or type(parsed.monitors) ~= "table" then
        return DEFAULT_MONITORS
    end

    local valid = {}
    for _, entry in ipairs(parsed.monitors) do
        if valid_entry(entry) then
            valid[#valid + 1] = {
                output = entry.output,
                mode = (type(entry.mode) == "string" and entry.mode) or "preferred",
                position = (type(entry.position) == "string" and entry.position) or "auto",
                scale = entry.scale ~= nil and entry.scale or "auto",
                mirror = type(entry.mirror) == "string" and entry.mirror or nil,
            }
        end
    end

    if #valid == 0 then
        return DEFAULT_MONITORS
    end
    return valid
end

local monitors = load_monitors()
local applied_any = false
for _, m in ipairs(monitors) do
    if pcall(hl.monitor, m) then
        applied_any = true
    end
end

-- Every entry failed to apply (e.g. Hyprland rejected values that still
-- passed our type checks) — guarantee a usable session rather than leaving
-- every monitor unconfigured.
if not applied_any then
    pcall(hl.monitor, DEFAULT_MONITORS[1])
end
