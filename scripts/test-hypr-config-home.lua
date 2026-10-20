#!/usr/bin/env lua
-- Checks that the Hyprland Lua loaders read their JSON from the config dir
-- resolved by scripts/lib/paths.lua: $XDG_CONFIG_HOME when it is an absolute
-- path, otherwise $HOME/.config. Runs under plain lua or luajit, with no
-- Hyprland, no session and no real $HOME: os.getenv is replaced by a fixture
-- environment and the `hl` API is stubbed.
--
-- Also checks that hyprland.lua cannot be aborted by its helper: a missing,
-- unparsable or throwing scripts/lib/paths.lua, or a config chunk that has no
-- file path (source not starting with "@"), falls back to the inline rule.
--
--   lua scripts/test-hypr-config-home.lua
--   luajit scripts/test-hypr-config-home.lua
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

-- Records every loadfile() request, so a test can assert whether the helper
-- was searched for at all (a string-loaded config must not search).
local real_loadfile = loadfile
local loadfile_calls = {}
local loadfile_raises = false -- set by the "loadfile itself raises" case
loadfile = function(path, ...)
    loadfile_calls[#loadfile_calls + 1] = path
    if loadfile_raises and path and path:find("scripts/lib/paths.lua", 1, true) then
        error("loadfile raised for " .. path)
    end
    return real_loadfile(path, ...)
end

local function reset(vars)
    env = vars
    opened = {}
    loadfile_calls = {}
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
    "grep -rn 'getenv(\"HOME\").*\\.config' " .. SKEL_HYPR .. " --include='*.lua' 2>/dev/null")
local offenders = {}
-- The only $HOME/.config literal allowed outside paths.lua is hyprland.lua's
-- inline fallback, used when the helper cannot be loaded (section 5).
local INLINE_FALLBACK = '(os.getenv("HOME") or "") .. "/.config/hypr/"'
if grep then
    for line in grep:lines() do
        local file, content = line:match("^(.-):%d+:(.*)$")
        local is_helper = file and file:match("scripts/lib/paths%.lua$")
        local is_inline = file and file:match("([^/]*)$") == "hyprland.lua"
            and content:find(INLINE_FALLBACK, 1, true)
        if not (is_helper or is_inline) then
            offenders[#offenders + 1] = line
        end
    end
    grep:close()
    expect("no loader builds $HOME/.config itself", #offenders == 0, table.concat(offenders, ", "))
else
    bad("grep check unavailable (io.popen missing)")
end

-- ---------------------------------------------------------------------------
-- 5. hyprland.lua must never abort because of its helper. Each case loads the
--    config from a staged tree whose scripts/lib/paths.lua is broken in one
--    way, with XDG_CONFIG_HOME set and with only HOME set. Mode "file" loads
--    with a file chunkname (source "@..."), mode "string" loads from a string
--    whose chunkname is not a path (source is not "@...").
-- ---------------------------------------------------------------------------
local function has(list, p)
    for _, q in ipairs(list) do
        if q == p then return true end
    end
    return false
end

local function write_file(path, text)
    local f = assert(real_open(path, "w"))
    f:write(text)
    f:close()
end

-- Each environment gets its own config tree, placed where that environment
-- resolves BOS_HYPR_DIR to (as the ISO installs it). Variants are staged under
-- FIX/variants/<variant>/<env>, so no two cases share a tree.
local function make_env(kind, variant, mutate)
    local base = FIX .. "/variants/" .. variant .. "/" .. kind
    local vars, cfg, other
    if kind == "xdg" then
        vars = { HOME = base .. "/home", XDG_CONFIG_HOME = base .. "/cfg" }
        cfg = base .. "/cfg"
        other = base .. "/home/.config/"
    else
        vars = { HOME = base .. "/home" }
        cfg = base .. "/home/.config"
        other = base .. "/cfg/"
    end
    if not stage(cfg) then bad("could not stage variant " .. variant .. " (" .. kind .. ")") end
    if mutate then mutate(cfg .. "/hypr/scripts/lib/paths.lua") end
    return { name = kind == "xdg" and "XDG set" or "HOME only", vars = vars,
        root = cfg, dir = cfg .. "/hypr/", other = other }
end

-- Returns what a real session would see: whether the chunk ran to the end,
-- whether the hyprland.start handler ran, the BOS_HYPR_DIR it left behind, and
-- every file the loaders opened.
local function run_config(vars, root, mode)
    reset(vars)
    handlers = {}
    BOS_HYPR_DIR = nil
    local chunk, cerr
    if mode == "string" then
        local f = real_open(root .. "/hypr/hyprland.lua", "r")
        local src = f:read("*a")
        f:close()
        chunk, cerr = load(src, "=bos-hyprland-string", "t")
    else
        chunk, cerr = loadfile(root .. "/hypr/hyprland.lua")
    end
    local r = { chunk_ok = false, err = cerr }
    if chunk then
        r.chunk_ok, r.err = pcall(chunk)
        if r.chunk_ok then
            r.fired_ok, r.fired_err = fire("hyprland.start")
        end
    end
    r.bos_dir = BOS_HYPR_DIR
    r.opened = opened
    r.loadfile_calls = loadfile_calls
    return r
end

local JSON_FILES = { "settings.json", "monitors.json", "autostart.json", "binds.json" }

local function check_run(label, r, want_dir, other_dir)
    expect(label .. ": hyprland.lua does not abort", r.chunk_ok, tostring(r.err))
    expect(label .. ": hyprland.start handler runs", r.fired_ok == true, tostring(r.fired_err))
    expect(label .. ": BOS_HYPR_DIR is " .. want_dir, r.bos_dir == want_dir, "got " .. tostring(r.bos_dir))
    for _, name in ipairs(JSON_FILES) do
        expect(label .. ": reads " .. want_dir .. name, has(r.opened, want_dir .. name))
    end
    local leaked = {}
    for _, p in ipairs(r.opened) do
        if other_dir and p:find(other_dir, 1, true) then leaked[#leaked + 1] = p end
    end
    expect(label .. ": nothing read from the other config dir", #leaked == 0, table.concat(leaked, ", "))
end

local BROKEN_HELPERS = {
    { name = "helper-missing", mutate = function(p) os.remove(p) end },
    { name = "helper-syntax-error", mutate = function(p) write_file(p, "local M = {\n") end },
    { name = "helper-throws-at-load", mutate = function(p) write_file(p, "error('boom at load')\n") end },
    { name = "helper-throws-in-hypr_dir", mutate = function(p)
        write_file(p, "return { hypr_dir = function() error('boom in hypr_dir') end }\n") end },
    { name = "helper-wrong-type", mutate = function(p) write_file(p, "return 42\n") end },
}

for _, v in ipairs(BROKEN_HELPERS) do
    for _, kind in ipairs({ "xdg", "home" }) do
        local e = make_env(kind, v.name, v.mutate)
        check_run(v.name .. " (" .. e.name .. ")", run_config(e.vars, e.root, "file"), e.dir, e.other)
    end
end

-- Source not starting with "@": the chunk is loaded from a string, so there is
-- no path to search from. Helper present and helper missing both must work.
for _, kind in ipairs({ "xdg", "home" }) do
    local e = make_env(kind, "string-helper-present", nil)
    local re = run_config(e.vars, e.root, "string")
    check_run("source not '@' (" .. e.name .. ")", re, e.dir, e.other)
    expect("source not '@' (" .. e.name .. "): helper is not searched for",
        #re.loadfile_calls == 0, "loadfile called for " .. table.concat(re.loadfile_calls, ", "))
    local m = make_env(kind, "string-helper-missing", function(p) os.remove(p) end)
    local rm = run_config(m.vars, m.root, "string")
    check_run("source not '@', helper missing (" .. m.name .. ")", rm, m.dir, m.other)
    expect("source not '@', helper missing (" .. m.name .. "): helper is not searched for",
        #rm.loadfile_calls == 0, "loadfile called for " .. table.concat(rm.loadfile_calls, ", "))
end

-- loadfile() itself raises (not just the helper's code): only the outer pcall
-- in hyprland.lua can contain this.
do
    local e = make_env("xdg", "loadfile-raises", nil)
    loadfile_raises = true
    local r = run_config(e.vars, e.root, "file")
    loadfile_raises = false
    check_run("loadfile raises (" .. e.name .. ")", r, e.dir, e.other)
end

-- The helper's hypr_dir() returns a value that is not a usable path string.
do
    local e = make_env("home", "helper-empty-string", function(p)
        write_file(p, "return { hypr_dir = function() return '' end }\n") end)
    check_run("helper returns empty dir (" .. e.name .. ")", run_config(e.vars, e.root, "file"), e.dir, e.other)
end

-- A helper whose field lookup throws: only the outer pcall can contain this.
do
    local e = make_env("xdg", "helper-index-throws", function(p)
        write_file(p, "return setmetatable({}, { __index = function() error('boom in __index') end })\n") end)
    check_run("helper __index throws (" .. e.name .. ")", run_config(e.vars, e.root, "file"), e.dir, e.other)
end

-- HOME unset: no path can be built from the environment. The config must still
-- load and BOS_HYPR_DIR must still be a string.
local nh = make_env("home", "home-unset", nil)
local r_nohome = run_config({ XDG_CONFIG_HOME = "" }, nh.root, "file")
expect("HOME unset: hyprland.lua does not abort", r_nohome.chunk_ok, tostring(r_nohome.err))
expect("HOME unset: BOS_HYPR_DIR is still a string", type(r_nohome.bos_dir) == "string",
    "got " .. tostring(r_nohome.bos_dir))

-- ---------------------------------------------------------------------------
-- 6. The inline fallback in hyprland.lua must agree with scripts/lib/paths.lua
--    for every input in this table. The helper is removed from the tree so the
--    inline rule is what runs; the expected value comes from the helper itself.
-- ---------------------------------------------------------------------------
local AGREE = {
    { HOME = "/home/u" },
    { HOME = "/home/u", XDG_CONFIG_HOME = "" },
    { HOME = "/home/u", XDG_CONFIG_HOME = "rel/cfg" },
    { HOME = "/home/u", XDG_CONFIG_HOME = "~/cfg" },
    { HOME = "/home/u", XDG_CONFIG_HOME = "/srv/cfg" },
    { HOME = "/home/u", XDG_CONFIG_HOME = "/srv/cfg/" },
    { HOME = "/home/u", XDG_CONFIG_HOME = "/srv/cfg//" },
    { HOME = "/home/u", XDG_CONFIG_HOME = "/" },
    { HOME = "/home/u", XDG_CONFIG_HOME = "//" },
    { HOME = "/home/u", XDG_CONFIG_HOME = "/srv/cfg with space/" },
    { HOME = "/home/u/" },
}
-- Helper removed: the inline rule is what runs (string load, so no file search).
local agree_inline = make_env("xdg", "agree-inline", function(p) os.remove(p) end)
-- Helper present: paths.lua is what runs (file load, so it is found).
local agree_helper = make_env("xdg", "agree-helper", nil)
for _, vars in ipairs(AGREE) do
    local want = paths.hypr_dir(getenv_from(vars))
    local label = string.format("HOME=%q XDG_CONFIG_HOME=%q", vars.HOME, tostring(vars.XDG_CONFIG_HOME))
    local r_inline = run_config(vars, agree_inline.root, "string")
    expect("agree (inline): " .. label, r_inline.chunk_ok and r_inline.bos_dir == want,
        string.format("inline %q, paths.lua %q", tostring(r_inline.bos_dir), want))
    local r_helper = run_config(vars, agree_helper.root, "file")
    expect("agree (helper): " .. label, r_helper.chunk_ok and r_helper.bos_dir == want,
        string.format("helper %q, paths.lua %q", tostring(r_helper.bos_dir), want))
end

os.getenv = real_getenv
io.open = real_open
os.execute(string.format("rm -rf '%s'", FIX))

print(string.format("\nResult: %d passed, %d failed", pass, fail))
os.exit(fail == 0 and 0 or 1)
