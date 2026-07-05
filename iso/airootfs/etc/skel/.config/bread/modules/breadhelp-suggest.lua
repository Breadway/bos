-- breadhelp-suggest — nudge breadhelp's Home tab banner when a bread event
-- suggests a relevant guide (e.g. a newly connected monitor -> breadmon
-- setup). Auto-discovered by breadd. If breadhelp isn't already running,
-- `--suggest <id>` launches it straight to Home with the banner focused; if
-- it's already running, the banner just updates silently (see
-- breadhelp's services/breadd.rs) rather than stealing focus on every event.

local M = bread.module({ name = "breadhelp-suggest", version = "1.0.0" })

function M.on_load()
    bread.on("bread.monitor.connected", function(event)
        bread.exec("breadhelp --suggest monitor-setup")
    end)
end

return M
