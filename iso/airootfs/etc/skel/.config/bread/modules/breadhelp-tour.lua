-- breadhelp-tour — forwards bread/Hyprland events the live guided tour
-- (breadhelp's ui::tour) uses to detect "the user actually did the thing"
-- and auto-advance a step. Always forwards; breadhelp's `--tour-event`
-- handler is a hard no-op unless a tour is currently waiting for that exact
-- id, so this module doesn't need to know whether a tour is even running.
--
-- `bread.window.opened`/`bread.workspace.changed` are already normalized by
-- breadd. Layer-surface opens/closes (breadbox, breadclip, breadsearch) are
-- NOT normalized — `openlayer`/`closelayer` fall through to the generic
-- `bread.hyprland.event` topic, which `on_raw` filters by raw kind.
--
-- Fullscreen click-catcher launchers (breadbox et al.) report their surface
-- as covering the whole monitor while open, so the tour can't safely show a
-- clickable callout at the same time — a step targeting one of these should
-- key its success on the *close* event (implying the user picked something
-- and it dismissed), not the open event, so the callout only ever reappears
-- once the launcher's surface is already gone and input contention is moot.

local M = bread.module({ name = "breadhelp-tour", version = "1.0.0" })

-- `bread.exec` only takes a single shell command string — it always runs it
-- as `sh -lc <cmd>` (see breadd's Lua runtime), there's no array-exec form
-- that bypasses the shell. `event.data.class` (a Wayland window class) and
-- `event.data.data` (a layer-shell namespace) are both arbitrary strings a
-- client fully controls — a window/surface can name itself
-- `x; rm -rf ~ #` and have that land in a real shell command otherwise.
-- POSIX single-quoting neutralizes that: wrap the value in single quotes,
-- and turn any single quote *inside* it into `'\''` (close the quote, an
-- escaped literal quote, reopen the quote) — the one escaping rule `sh`
-- needs to treat the whole thing as inert data, never command syntax.
local function shell_quote(s)
    return "'" .. tostring(s):gsub("'", "'\\''") .. "'"
end

function M.on_load()
    bread.on("bread.window.opened", function(event)
        bread.exec("breadhelp --tour-event " .. shell_quote("window:" .. event.data.class))
    end)

    bread.on("bread.workspace.changed", function(event)
        bread.exec("breadhelp --tour-event workspace-changed")
    end)

    bread.hyprland.on_raw("openlayer", function(event)
        bread.exec("breadhelp --tour-event " .. shell_quote("layer:" .. event.data.data))
    end)

    bread.hyprland.on_raw("closelayer", function(event)
        bread.exec("breadhelp --tour-event " .. shell_quote("layer-closed:" .. event.data.data))
    end)
end

return M
