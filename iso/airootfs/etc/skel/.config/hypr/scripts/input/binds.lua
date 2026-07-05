-- scripts/input/binds.lua — loads binds.json into { default_mods, bindings }.
-- Simpler than the multi-keyboard-layout version some personal configs use
-- (BOS ships one fixed layout, no per-layout bind switching needed): just
-- reads `default_mods` + the flat `bindings` array.

local json = dofile(os.getenv("HOME") .. "/.config/hypr/scripts/lib/json.lua")

local function normalize_mods(value, fallback)
    local mods = {}
    if type(value) == "table" then
        for _, item in ipairs(value) do
            if type(item) == "string" and item ~= "" then
                mods[#mods + 1] = item
            end
        end
    end
    if #mods == 0 then
        for _, mod in ipairs(fallback or {}) do
            mods[#mods + 1] = mod
        end
    end
    return mods
end

return function(configPath)
    local parsed = json.load(configPath)
    if type(parsed) ~= "table" then
        return { default_mods = { "SUPER" }, bindings = {} }
    end

    return {
        default_mods = normalize_mods(parsed.default_mods, { "SUPER" }),
        bindings = type(parsed.bindings) == "table" and parsed.bindings or {},
    }
end
