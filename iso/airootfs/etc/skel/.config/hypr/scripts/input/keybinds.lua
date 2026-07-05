-- scripts/input/keybinds.lua — turns binds.json entries into real hl.bind()
-- calls. Ported from a personal modular Hyprland config using the same
-- `hl` API; action_builders covers every dispatcher shape BOS's keybinds
-- actually use (exec, window management, focus/move/resize, workspaces,
-- mouse, media keys).
return function(ctx)
    local function split_mods(value)
        local mods = {}
        for raw_mod in tostring(value):gmatch("[^+]+") do
            local mod = raw_mod:gsub("^%s+", ""):gsub("%s+$", "")
            if mod ~= "" then
                mods[#mods + 1] = mod
            end
        end
        return mods
    end

    -- `allow_empty`: an explicit `"mods": []` in binds.json (media keys,
    -- which must bind with no modifier at all) must NOT fall back to
    -- default_mods — only an *omitted* `mods` field should.
    local function normalize_mods(value, fallback, allow_empty)
        if value == nil then
            return fallback
        end
        if type(value) ~= "table" then
            return fallback
        end
        if #value == 0 then
            return allow_empty and {} or fallback
        end
        local mods = {}
        for _, item in ipairs(value) do
            if type(item) == "string" then
                for _, mod in ipairs(split_mods(item)) do
                    mods[#mods + 1] = mod
                end
            end
        end
        if #mods == 0 then
            return fallback
        end
        return mods
    end

    local defaultMods = normalize_mods(ctx.default_mods, { "SUPER" }, false)
    local bindings = ctx.bindings or {}

    local function bind_string(mods, key)
        if type(key) ~= "string" or key == "" then
            return nil
        end
        if #mods == 0 then
            return key
        end
        return table.concat(mods, " + ") .. " + " .. key
    end

    local action_builders = {
        exec = function(entry)
            return hl.dsp.exec_cmd(entry.command)
        end,
        close = function()
            return hl.dsp.window.close()
        end,
        exit = function()
            return hl.dsp.exit()
        end,
        float = function()
            return hl.dsp.window.float({ action = "toggle" })
        end,
        fullscreen = function()
            return hl.dsp.window.fullscreen({ action = "toggle" })
        end,
        pseudo = function()
            return hl.dsp.window.pseudo({ action = "toggle" })
        end,
        layout = function(entry)
            return hl.dsp.layout(entry.layout)
        end,
        focus = function(entry)
            return hl.dsp.focus({ direction = entry.direction, workspace = entry.workspace })
        end,
        focus_last = function()
            return hl.dsp.focus({ urgent_or_last = true })
        end,
        move = function(entry)
            return hl.dsp.window.move({ workspace = entry.workspace })
        end,
        move_dir = function(entry)
            return hl.dsp.window.move({ direction = entry.direction })
        end,
        resize = function()
            return hl.dsp.window.resize()
        end,
        resize_dir = function(entry)
            return hl.dsp.window.resize({ x = entry.x or 0, y = entry.y or 0, relative = true })
        end,
        drag = function()
            return hl.dsp.window.drag()
        end,
    }

    -- One bad entry (unknown action, missing field a builder needs) must
    -- never take down every other bind — skip it and keep going instead of
    -- asserting/erroring, which would abort binds.json loading entirely.
    for _, entry in ipairs(bindings) do
        local builder = action_builders[entry.action]
        if builder then
            local ok, result = pcall(function()
                local mods = normalize_mods(entry.mods, defaultMods, true)
                local bind = bind_string(mods, entry.key)
                local action = builder(entry)
                if bind and action then
                    hl.bind(bind, action, entry.options)
                end
            end)
            if not ok then
                print("breadhelp/hyprland: skipping bad bind entry (" .. tostring(entry.action) .. "/" .. tostring(entry.key) .. "): " .. tostring(result))
            end
        else
            print("breadhelp/hyprland: skipping bind entry with unknown action: " .. tostring(entry.action))
        end
    end
end
