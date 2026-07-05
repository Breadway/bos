-- scripts/lib/json.lua
local M = {}

function M.parse(str)
    local s = str
    local pos = 1
    local len = #s

    local function skipws()
        while pos <= len and s:sub(pos,pos):match('%s') do pos = pos + 1 end
    end

    local function parse_string()
        if s:sub(pos,pos) ~= '"' then error('expected string') end
        pos = pos + 1
        local out = {}
        while pos <= len do
            local c = s:sub(pos,pos)
            if c == '"' then pos = pos + 1; return table.concat(out) end
            if c == '\\' then
                local n = s:sub(pos+1,pos+1)
                if n == '"' then out[#out+1] = '"'; pos = pos + 2
                elseif n == '\\' then out[#out+1] = '\\'; pos = pos + 2
                elseif n == '/' then out[#out+1] = '/'; pos = pos + 2
                elseif n == 'b' then out[#out+1] = '\b'; pos = pos + 2
                elseif n == 'f' then out[#out+1] = '\f'; pos = pos + 2
                elseif n == 'n' then out[#out+1] = '\n'; pos = pos + 2
                elseif n == 'r' then out[#out+1] = '\r'; pos = pos + 2
                elseif n == 't' then out[#out+1] = '\t'; pos = pos + 2
                elseif n == 'u' then
                    local hex = s:sub(pos+2, pos+5)
                    local code = tonumber(hex, 16)
                    if code then out[#out+1] = utf8.char(code) end
                    pos = pos + 6
                else
                    pos = pos + 2
                end
            else
                out[#out+1] = c
                pos = pos + 1
            end
        end
        error('unclosed string')
    end

    local function parse_value()
        skipws()
        local c = s:sub(pos,pos)
        if c == '"' then return parse_string()
        elseif c == '{' then
            return parse_object()
        elseif c == '[' then
            return parse_array()
        elseif c:match('[%d%-]') then
            local start = pos
            while s:sub(pos,pos):match('[%d+%-.eE]') do pos = pos + 1 end
            local num = tonumber(s:sub(start, pos-1))
            return num
        elseif s:sub(pos,pos+3) == 'null' then pos = pos + 4; return nil
        elseif s:sub(pos,pos+3) == 'true' then pos = pos + 4; return true
        elseif s:sub(pos,pos+4) == 'false' then pos = pos + 5; return false
        else
            error('unexpected value at ' .. pos)
        end
    end

    function parse_array()
        if s:sub(pos,pos) ~= '[' then error('expected [') end
        pos = pos + 1
        skipws()
        local arr = {}
        if s:sub(pos,pos) == ']' then pos = pos + 1; return arr end
        while true do
            skipws()
            local val = parse_value()
            table.insert(arr, val)
            skipws()
            local c = s:sub(pos,pos)
            if c == ']' then pos = pos + 1; break
            elseif c == ',' then pos = pos + 1; skipws()
            else error('expected , or ]') end
        end
        return arr
    end

    function parse_object()
        if s:sub(pos,pos) ~= '{' then error('expected {') end
        pos = pos + 1
        skipws()
        local obj = {}
        if s:sub(pos,pos) == '}' then pos = pos + 1; return obj end
        while true do
            skipws()
            local key = parse_string()
            skipws()
            if s:sub(pos,pos) ~= ':' then error('expected :') end
            pos = pos + 1
            skipws()
            local val = parse_value()
            obj[key] = val
            skipws()
            local c = s:sub(pos,pos)
            if c == '}' then pos = pos + 1; break
            elseif c == ',' then pos = pos + 1; skipws()
            else error('expected , or }') end
        end
        return obj
    end

    skipws()
    return parse_value()
end

function M.load(path)
    local ok, fh = pcall(io.open, path, "r")
    if not ok or not fh then
        return nil, "unable to open file"
    end

    local content = fh:read("*a")
    fh:close()

    local success, parsed = pcall(M.parse, content)
    if not success then
        return nil, parsed
    end

    return parsed
end

function M.encode(value, indent)
    indent = indent or 0
    local indent_str = string.rep("    ", indent)
    local next_indent_str = string.rep("    ", indent + 1)

    if value == nil then
        return "null"
    elseif type(value) == "boolean" then
        return value and "true" or "false"
    elseif type(value) == "number" then
        return tostring(value)
    elseif type(value) == "string" then
        return '"' .. value:gsub('\\', '\\\\'):gsub('"', '\\"'):gsub('\n', '\\n'):gsub('\r', '\\r'):gsub('\t', '\\t') .. '"'
    elseif type(value) == "table" then
        local is_array = true
        local max_idx = 0
        for k in pairs(value) do
            if type(k) ~= "number" then
                is_array = false
                break
            end
            max_idx = math.max(max_idx, k)
        end

        if is_array and max_idx == #value then
            if max_idx == 0 then
                return "[]"
            end
            local items = {}
            for i = 1, max_idx do
                table.insert(items, next_indent_str .. M.encode(value[i], indent + 1))
            end
            return "[\n" .. table.concat(items, ",\n") .. "\n" .. indent_str .. "]"
        else
            local items = {}
            for k, v in pairs(value) do
                table.insert(items, next_indent_str .. M.encode(tostring(k), 0) .. ": " .. M.encode(v, indent + 1))
            end
            if #items == 0 then
                return "{}"
            end
            table.sort(items)
            return "{\n" .. table.concat(items, ",\n") .. "\n" .. indent_str .. "}"
        end
    else
        return "null"
    end
end

return M
