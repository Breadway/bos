#!/usr/bin/env lua
-- Unit test for the BOS window dispatcher action shim (BOS-91).
--
-- Hyprland's Lua binding parses window.float actions with parseToggleStr,
-- which only understands toggle/enable/disable (and on/off). The documented
-- set/unset actions fall through to toggle, so they are not idempotent. This
-- test loads the shim with a recording stand-in for the real dispatcher and
-- checks the action mapping, passthrough, return value and reload marker.
--
-- It needs no compositor and no session. Run it with:
--
--   lua scripts/test-window-float.lua
--   luajit scripts/test-window-float.lua
--
-- Exit status is non-zero if any check fails.
local HERE = (arg and arg[0] or ""):match("^(.*)/[^/]*$") or "."
local SKEL_HYPR = HERE .. "/../iso/airootfs/etc/skel/.config/hypr/"

local pass, fail = 0, 0
local function ok(name) print("  PASS  " .. name); pass = pass + 1 end
local function bad(name, why)
    print("  FAIL  " .. name .. (why and (": " .. why) or ""))
    fail = fail + 1
end
local function expect(name, cond, why)
    if cond then ok(name) else bad(name, why) end
end

local install = dofile(SKEL_HYPR .. "scripts/lib/window.lua")

-- A stand-in for the real factory: it records each options argument (including
-- nil) and returns a unique table, so the test can prove the shim forwards the
-- dispatcher object unchanged.
local function fixture()
    local calls = {}
    local sentinel = {}
    local function raw(opts)
        calls[#calls + 1] = { opts = opts }
        return sentinel
    end
    local hl = { dsp = { window = { float = raw } } }
    return hl, calls, sentinel
end

local function action_of(call) return call and call.opts and call.opts.action end

-- ---------------------------------------------------------------------------
-- 1. The shim installs and returns the real dispatcher object.
-- ---------------------------------------------------------------------------
do
    local hl, calls, sentinel = fixture()
    local installed, err = install(hl)
    expect("install succeeds", installed == true, tostring(err))
    local result = hl.dsp.window.float({ action = "set", window = "address:0x1" })
    expect("returns the original dispatcher object", result == sentinel)
    expect("forwards one call to the original factory", #calls == 1, "got " .. tostring(#calls))
    expect("set maps to enable", action_of(calls[1]) == "enable", "got " .. tostring(action_of(calls[1])))
    expect("window selector is preserved", calls[1].opts.window == "address:0x1", "got " .. tostring(calls[1].opts.window))
end

-- ---------------------------------------------------------------------------
-- 2. action = set stays floating when dispatched twice.
-- ---------------------------------------------------------------------------
do
    local hl, calls = fixture()
    install(hl)
    hl.dsp.window.float({ action = "set", window = "address:0x2" })
    hl.dsp.window.float({ action = "set", window = "address:0x2" })
    expect("set twice maps both calls to enable",
        action_of(calls[1]) == "enable" and action_of(calls[2]) == "enable",
        table.concat({ tostring(action_of(calls[1])), tostring(action_of(calls[2])) }, ","))
end

-- ---------------------------------------------------------------------------
-- 3. action = unset stays tiled when dispatched twice.
-- ---------------------------------------------------------------------------
do
    local hl, calls = fixture()
    install(hl)
    hl.dsp.window.float({ action = "unset", window = "address:0x3" })
    hl.dsp.window.float({ action = "unset", window = "address:0x3" })
    expect("unset twice maps both calls to disable",
        action_of(calls[1]) == "disable" and action_of(calls[2]) == "disable",
        table.concat({ tostring(action_of(calls[1])), tostring(action_of(calls[2])) }, ","))
end

-- ---------------------------------------------------------------------------
-- 4. Every other action is passed through unchanged.
-- ---------------------------------------------------------------------------
do
    local cases = { "toggle", "enable", "disable", "on", "off", "unrecognised" }
    for _, action in ipairs(cases) do
        local hl, calls = fixture()
        install(hl)
        hl.dsp.window.float({ action = action, window = "address:0x4" })
        expect("action " .. action .. " is passed through",
            action_of(calls[1]) == action,
            "got " .. tostring(action_of(calls[1])))
    end
    local hl, calls = fixture()
    install(hl)
    hl.dsp.window.float({ window = "address:0x4" })
    expect("missing action is passed through",
        action_of(calls[1]) == nil,
        "got " .. tostring(action_of(calls[1])))
    hl.dsp.window.float(nil)
    expect("nil options are passed through", calls[2] ~= nil and calls[2].opts == nil)
end

-- ---------------------------------------------------------------------------
-- 5. The caller's table is not mutated.
-- ---------------------------------------------------------------------------
do
    local hl = fixture()
    install(hl)
    local opts = { action = "set", window = "address:0x5" }
    hl.dsp.window.float(opts)
    expect("caller table keeps action = set", opts.action == "set", "got " .. tostring(opts.action))
end

-- ---------------------------------------------------------------------------
-- 6. Installing twice does not wrap the shim around itself.
-- ---------------------------------------------------------------------------
do
    local hl, calls = fixture()
    install(hl)
    local first = hl.dsp.window.float
    local again, err = install(hl)
    expect("second install succeeds", again == true, tostring(err))
    expect("second install keeps the same wrapper", hl.dsp.window.float == first)
    hl.dsp.window.float({ action = "set", window = "address:0x6" })
    expect("second install still makes exactly one factory call", #calls == 1, "got " .. tostring(#calls))
    expect("second install still maps set to enable", action_of(calls[1]) == "enable")
end

-- ---------------------------------------------------------------------------
-- 7. A broken hl table is reported, not thrown.
-- ---------------------------------------------------------------------------
do
    local bad_values = { 42, {}, { dsp = {} }, { dsp = { window = {} } } }
    for _, value in ipairs(bad_values) do
        local installed, err = install(value)
        expect("invalid hl is rejected", installed == false and type(err) == "string")
    end
    local installed, err = install(nil)
    expect("nil hl is rejected", installed == false and type(err) == "string")
end

-- ---------------------------------------------------------------------------
-- 8. hyprland.lua must load the shim before it loads anything that can
--    dispatch a window.
-- ---------------------------------------------------------------------------
do
    local f = io.open(SKEL_HYPR .. "hyprland.lua", "r")
    local source = f and f:read("*a")
    if f then f:close() end
    expect("hyprland.lua loads scripts/lib/window.lua",
        source and source:find('lib/window%.lua', 1) ~= nil)
    local shim = io.open(SKEL_HYPR .. "scripts/lib/window.lua", "r")
    expect("the shim file exists", shim ~= nil)
    if shim then shim:close() end
end

print(string.format("\nResult: %d passed, %d failed", pass, fail))
os.exit(fail == 0 and 0 or 1)
