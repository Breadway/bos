#!/usr/bin/env lua
-- Checks that the Hyprland Lua loaders read their JSON from the config dir
-- resolved by scripts/lib/paths.lua: $XDG_CONFIG_HOME when it is an absolute
-- path, otherwise $HOME/.config. Runs under plain lua or luajit, with no
-- Hyprland, no session and no real $HOME: os.getenv is replaced by a fixture
-- environment and the `hl` API is stubbed.
--
--   lua scripts/test-hypr-config-home.lua
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

-- ---------------------------------------------------------------------------
-- Fixture environment. Only these variables are visible to the code under test.
-- ---------------------------------------------------------------------------
local env = {}
local real_getenv = os.getenv
os.getenv = function(name) return env[name] end

-- Records every path that was opened for reading, so a test can assert
-- which file a loader actually asked for.
local opened = {}
local real_open = io.open
io.open = function(path, mode)
    opened[#opened + 1] = path
    return real_open(path, mode)
end

-- Stub for the `hl` API: any field or call returns another stub, so the
-- top-level calls in hyprland.lua and the loaders do not error. The one
-- exception is hl.on(event, fn): the handler is recorded so the test can
-- fire "hyprland.start" the way the compositor would after load.
local handlers = {}
local function stub()
    local t = {}
    return setmetatable(t, {
        __index = function(_, k)
            local child = stub()
            rawset(t, k, child)
            return child
        end,
        __call = function() return stub() end,
    })
end
hl = stub()
hl.on = function(event, fn) handlers[event] = fn end

local function fire(event)
    if handlers[event] then
        local ok, err = pcall(handlers[event])
        return ok, err
    end
    return false, "no handler for " .. event
end

local function reset(vars)
    env = vars
    opened = {}
end

local function loaded(path)
    for _, p in ipairs(opened) do
        if p == path then return true end
    end
    return false
end

local function expect(name, cond, why)
    if cond then ok(name) else bad(name, why) end
end

-- ---------------------------------------------------------------------------
-- 1. The helper itself.
-- ---------------------------------------------------------------------------
local paths = dofile(SKEL_HYPR .. "scripts/lib/paths.lua")

local function getenv_from(t) return function(k) return t[k] end end

expect("paths: XDG_CONFIG_HOME absolute is used",
    paths.config_home(getenv_from({ HOME = "/home/u", XDG_CONFIG_HOME = "/srv/cfg" })) == "/srv/cfg",
    "got " .. tostring(paths.config_home(getenv_from({ HOME = "/home/u", XDG_CONFIG_HOME = "/srv/cfg" }))))
expect("paths: XDG_CONFIG_HOME unset falls back to HOME/.config",
    paths.config_home(getenv_from({ HOME = "/home/u" })) == "/home/u/.config")
expect("paths: XDG_CONFIG_HOME empty falls back to HOME/.config",
    paths.config_home(getenv_from({ HOME = "/home/u", XDG_CONFIG_HOME = "" })) == "/home/u/.config")
expect("paths: XDG_CONFIG_HOME relative is ignored (XDG spec)",
    paths.config_home(getenv_from({ HOME = "/home/u", XDG_CONFIG_HOME = "rel/cfg" })) == "/home/u/.config")
expect("paths: trailing slash on XDG_CONFIG_HOME is trimmed",
    paths.config_home(getenv_from({ HOME = "/home/u", XDG_CONFIG_HOME = "/srv/cfg//" })) == "/srv/cfg")
expect("paths: hypr_dir has trailing slash",
    paths.hypr_dir(getenv_from({ HOME = "/home/u", XDG_CONFIG_HOME = "/srv/cfg" })) == "/srv/cfg/hypr/")

-- ---------------------------------------------------------------------------
-- 2. Full load of hyprland.lua and its loaders, with XDG_CONFIG_HOME set.
--    Every JSON read must come from the XDG dir, never from $HOME/.config.
-- ---------------------------------------------------------------------------
-- hyprland.lua runs top to bottom from the config dir it was loaded from, so
-- the copy staged under `root` is what gets loaded. It must not error out.
local function load_config(vars, root)
    reset(vars)
    handlers = {}
    local chunk_ok, err = pcall(dofile, root .. "/hypr/hyprland.lua")
    if chunk_ok then
        -- autostart.json is read from the startup handler, as on a real session.
        local fired_ok, fired_err = fire("hyprland.start")
        if not fired_ok then
            return false, "hyprland.start: " .. tostring(fired_err)
        end
    end
    return chunk_ok, err
end

-- The loaders are found relative to the config dir they run from, so the
-- shipped tree is copied into a fixture config dir first, the same way the
-- ISO installs it. Both the XDG dir and the HOME dir get their own copy.
local TMP = os.tmpname()
os.remove(TMP)
local FIX = TMP .. "-bos-hypr-test"
local XDG = FIX .. "/xdg/config"
local HOME = FIX .. "/home"
local function stage(dst)
    local rc = os.execute(string.format("mkdir -p '%s' && cp -r '%s'/. '%s/hypr/' 2>/dev/null",
        dst .. "/hypr", SKEL_HYPR, dst))
    return rc == true or rc == 0
end
local staged_xdg = stage(XDG)
local staged_home = stage(HOME .. "/.config")
if not (staged_xdg and staged_home) then
    bad("could not stage fixture config trees under " .. FIX)
end

local ok_xdg, err_xdg = load_config({ HOME = HOME, XDG_CONFIG_HOME = XDG }, XDG)
expect("hyprland.lua loads with XDG_CONFIG_HOME set", ok_xdg, tostring(err_xdg))

local wanted = {
    XDG .. "/hypr/settings.json",
    XDG .. "/hypr/monitors.json",
    XDG .. "/hypr/autostart.json",
    XDG .. "/hypr/binds.json",
}
for _, p in ipairs(wanted) do
    -- Missing files are fine: the loaders fall back to defaults. We only
    -- care which path was opened.
    expect("XDG load reads " .. p, loaded(p))
end

local leaked = {}
for _, p in ipairs(opened) do
    if p:find(HOME .. "/.config/", 1, true) then leaked[#leaked + 1] = p end
end
expect("no read from $HOME/.config when XDG_CONFIG_HOME is set",
    #leaked == 0, table.concat(leaked, ", "))

-- ---------------------------------------------------------------------------
-- 3. XDG_CONFIG_HOME unset: behaviour must be unchanged ($HOME/.config).
-- ---------------------------------------------------------------------------
local ok_home, err_home = load_config({ HOME = HOME }, HOME .. "/.config")
expect("hyprland.lua loads with XDG_CONFIG_HOME unset", ok_home, tostring(err_home))
for _, name in ipairs({ "settings.json", "monitors.json", "autostart.json", "binds.json" }) do
    expect("fallback reads " .. HOME .. "/.config/hypr/" .. name,
        loaded(HOME .. "/.config/hypr/" .. name))
end

-- ---------------------------------------------------------------------------
-- 4. Every loader must go through the shared helper. A stray literal
--    "/.config/hypr" in the skel tree is the bug this card describes.
-- ---------------------------------------------------------------------------
local grep = io.popen and io.popen(
    "grep -rln 'getenv(\"HOME\").*\\.config' " .. SKEL_HYPR .. " --include='*.lua' 2>/dev/null")
local offenders = {}
if grep then
    for line in grep:lines() do
        -- paths.lua is the one place allowed to build the $HOME/.config fallback.
        if not line:find("scripts/lib/paths.lua", 1, true) then
            offenders[#offenders + 1] = line
        end
    end
    grep:close()
    expect("no loader builds $HOME/.config itself", #offenders == 0, table.concat(offenders, ", "))
else
    bad("grep check unavailable (io.popen missing)")
end

os.getenv = real_getenv
io.open = real_open
os.execute(string.format("rm -rf '%s'", FIX))

print(string.format("\nResult: %d passed, %d failed", pass, fail))
os.exit(fail == 0 and 0 or 1)
