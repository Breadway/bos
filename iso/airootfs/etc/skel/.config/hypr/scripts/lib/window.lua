-- Hyprland's Lua window dispatchers document action = "set" / "unset", but
-- the binding's parseToggleStr only understands toggle, enable/disable and
-- on/off. Every other string, including set and unset, falls through to
-- toggle. A caller that dispatches action = "set" twice therefore floats the
-- window and then re-tiles it on the second call (BOS-91).
--
-- Install one shim at config load. It rewrites the two documented actions to
-- the underlying enable/disable actions and leaves every other value, toggle
-- included, untouched. The shim returns the real dispatcher object from the
-- original factory, so it still works with hl.dispatch and hl.bind.
--
-- The marker on the hl table keeps a config reload from wrapping the shim
-- around itself.
return function(hl)
    if type(hl) ~= "table" or type(hl.dsp) ~= "table" or type(hl.dsp.window) ~= "table" then
        return false, "hl.dsp.window is unavailable"
    end

    local raw_float = hl.dsp.window.float
    if type(raw_float) ~= "function" then
        return false, "hl.dsp.window.float is not callable"
    end
    if hl._bos_window_float_shim == raw_float then
        return true
    end

    local function float(opts)
        if type(opts) == "table" then
            local action = opts.action
            if action == "set" or action == "unset" then
                local mapped = {}
                for key, value in pairs(opts) do
                    mapped[key] = value
                end
                mapped.action = action == "set" and "enable" or "disable"
                opts = mapped
            end
        end
        return raw_float(opts)
    end

    hl.dsp.window.float = float
    hl._bos_window_float_shim = float
    return true
end
