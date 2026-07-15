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

function M.on_load()
    bread.on("bread.window.opened", function(event)
        bread.exec("breadhelp --tour-event window:" .. event.data.class)
    end)

    bread.on("bread.workspace.changed", function(event)
        bread.exec("breadhelp --tour-event workspace-changed")
    end)

    bread.hyprland.on_raw("openlayer", function(event)
        bread.exec("breadhelp --tour-event layer:" .. event.data.data)
    end)

    bread.hyprland.on_raw("closelayer", function(event)
        bread.exec("breadhelp --tour-event layer-closed:" .. event.data.data)
    end)
end

return M
