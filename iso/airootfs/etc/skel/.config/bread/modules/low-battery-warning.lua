-- low-battery-warning — notify once when the battery runs low (zero-config).
-- Shipped active in BOS; auto-discovered by breadd. Safe on desktops too
-- (simply never fires without a battery).

local M = bread.module({ name = "low-battery-warning", version = "1.0.0" })

local warned = false

function M.on_load()
    bread.on("bread.power.battery.low", function(event)
        if warned then return end
        warned = true
        local pct = event.data.battery_percent or "?"
        bread.notify("Battery low (" .. pct .. "%). Plug in soon.", {
            urgency = "critical",
            title   = "Battery",
            timeout = 10000,
        })
    end)

    bread.on("bread.power.ac.connected", function()
        warned = false
    end)
end

return M
