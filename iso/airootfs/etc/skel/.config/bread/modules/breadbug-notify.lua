-- breadbug-notify: offer to review a crash or freeze report.
-- Shipped active in BOS; auto-discovered by breadd. Reacts to the
-- bread.bug.* events breadd publishes when a bread app panics, crashes or
-- freezes. It only shows a notification; nothing is sent anywhere. The
-- notification offers a "Review report" action (when the notification
-- daemon supports actions) and always names the shortcut, SUPER + SHIFT + B.

local M = bread.module({ name = "breadbug-notify", version = "1.0.0" })

-- A crash loop should not bury the desktop in notifications.
local COOLDOWN_MS = 60 * 1000

local last_shown = {}   -- app -> event time (ms) of the last notification
local frozen = {}       -- app -> true while a freeze is outstanding

-- App ids come from breadd, but they end up inside a shell command, so
-- only ever accept the plain id shape.
local function safe_app(app)
    return type(app) == "string" and #app <= 32 and app:match("^[a-z][a-z0-9_-]*$") ~= nil
end

local function offer(app, headline)
    local cmd = string.format(
        "a=$(notify-send -A review='Review report' -w -a breadbug -u normal "
        .. "'%s' 'Nothing is sent unless you choose to. Or press SUPER + SHIFT + B.' 2>/dev/null); "
        .. "[ \"$a\" = review ] && exec kitty --class breadbug --title 'Report a problem' breadbug --app %s",
        headline, app)
    bread.exec(cmd)
end

local function due(app, t)
    local now = tonumber(t) or 0
    local prev = last_shown[app]
    if prev and now > 0 and now - prev < COOLDOWN_MS then
        return false
    end
    last_shown[app] = now
    return true
end

function M.on_load()
    bread.on("bread.bug.panic", function(event)
        local app = event.data.app
        if safe_app(app) and due(app, event.data.t) then
            offer(app, app .. " crashed")
        end
    end)

    bread.on("bread.bug.crash", function(event)
        local app = event.data.app
        if safe_app(app) and due(app, event.data.t) then
            offer(app, app .. " crashed")
        end
    end)

    bread.on("bread.bug.freeze", function(event)
        local app = event.data.app
        if safe_app(app) and not frozen[app] then
            frozen[app] = true
            offer(app, app .. " looks frozen")
        end
    end)

    bread.on("bread.bug.recovered", function(event)
        local app = event.data.app
        if safe_app(app) then
            frozen[app] = nil
        end
    end)
end

return M
