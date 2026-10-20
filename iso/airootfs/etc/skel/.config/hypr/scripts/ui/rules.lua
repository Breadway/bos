-- scripts/ui/rules.lua: layer-shell (compositor) rules for the bread shell.
--
-- Blur / ignore-alpha / per-namespace motion for breadbar (island + osd +
-- notif + panel + dismiss), the breadbox launcher, and the breadclip popup.
-- Window rules (float/centre for onboarding popups) live in hyprland.lua;
-- this file is layer rules only.
--
-- Theme-driven (bos-ui-demos THEME_SYSTEM_PLAN.md §9): `bread-theme
-- layerrules` writes ~/.config/hypr/layerrules.json from the active shell
-- theme's [compositor] table: blur / ignore_alpha / blur_popups /
-- animation / no_anim, keyed by layer-shell namespace. That table owns
-- *appearance* only; placement / workspace / focus are never touched here.
-- If the JSON is missing, unreadable, malformed, or anything goes wrong
-- while parsing it, apply_hardcoded_layer_rules() runs instead: the exact
-- set this file hardcoded before the theme system existed, so a broken or
-- absent theme file can never disable compositor blur.

local LAYERRULES_JSON = BOS_HYPR_DIR .. "layerrules.json"

local function apply_hardcoded_layer_rules()
    hl.layer_rule({
        name         = "breadbar-island",
        match        = { namespace = "^breadbar$" },
        blur         = true,
        ignore_alpha = 0.2,
        blur_popups  = true,
        animation    = "slide top",
    })

    hl.layer_rule({
        name         = "breadbar-osd",
        match        = { namespace = "^breadbar-osd$" },
        blur         = true,
        ignore_alpha = 0.2,
        animation    = "slide bottom",
    })

    hl.layer_rule({
        name         = "breadbar-notif",
        match        = { namespace = "^breadbar-notif$" },
        blur         = true,
        ignore_alpha = 0.2,
        animation    = "slide right",
    })

    hl.layer_rule({
        name         = "breadbar-panel",
        match        = { namespace = "^breadbar-panel$" },
        blur         = true,
        ignore_alpha = 0.2,
        animation    = "slide right",
    })

    hl.layer_rule({
        name    = "breadbar-dismiss",
        match   = { namespace = "^breadbar-dismiss$" },
        no_anim = true,
    })

    hl.layer_rule({
        match        = "breadbox",
        blur         = true,
        ignore_alpha = 0.2,
    })
end

-- Parses ~/.config/hypr/layerrules.json into a plain array of per-namespace
-- rule tables, sorted by namespace for a deterministic emission order.
-- Returns nil (not an error) for anything short of a well-formed, non-empty
-- JSON object, so the pcall wrapping this only guards against a genuine Lua
-- error (e.g. json.lua failing to load) rather than every malformed case.
local function parsed_layer_rules()
    local json = dofile(BOS_HYPR_DIR .. "scripts/lib/json.lua")
    local parsed = json.load(LAYERRULES_JSON)
    if type(parsed) ~= "table" then
        return nil
    end

    local namespaces = {}
    for ns, rule in pairs(parsed) do
        if type(ns) == "string" and type(rule) == "table" then
            namespaces[#namespaces + 1] = ns
        end
    end
    if #namespaces == 0 then
        return nil
    end
    table.sort(namespaces)

    local rules = {}
    for _, ns in ipairs(namespaces) do
        local r = parsed[ns]
        rules[#rules + 1] = {
            namespace    = ns,
            blur         = r.blur == true,
            ignore_alpha = r.ignore_alpha,
            blur_popups  = r.blur_popups == true,
            animation    = r.animation,
            no_anim      = r.no_anim == true,
        }
    end
    return rules
end

-- Emits one hl.layer_rule per parsed namespace. Built as pure data by
-- parsed_layer_rules() first (no hl.layer_rule calls during parsing), so a
-- parse failure can never leave a partial JSON-derived rule set applied
-- before the caller falls back to apply_hardcoded_layer_rules().
local function apply_json_layer_rules(rules)
    for _, r in ipairs(rules) do
        hl.layer_rule({
            name         = r.namespace,
            match        = { namespace = "^" .. r.namespace .. "$" },
            blur         = r.blur,
            ignore_alpha = r.ignore_alpha,
            blur_popups  = r.blur_popups,
            animation    = r.animation,
            no_anim      = r.no_anim,
        })
    end
end

local ok, rules = pcall(parsed_layer_rules)
if ok and rules then
    apply_json_layer_rules(rules)
else
    apply_hardcoded_layer_rules()
end

-- breadclip is not part of the shell theme's [compositor] table yet, always
-- hardcoded, regardless of the JSON above.
hl.layer_rule({
    match        = "breadclip",
    blur         = true,
    ignore_alpha = 0.0,
})
