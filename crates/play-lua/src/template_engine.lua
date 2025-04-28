local template_engine = {}

-- Compiles a template string into a Lua function
function template_engine.compile(template)
    -- Build chunks of Lua code
    local chunks = {}
    table.insert(chunks, "local _ENV = ...\nlocal _result = {}\n")

    local pos = 1
    local len = #template
    local last_was_code_line = false

    while pos <= len do
        -- Handle {{ expression }}
        local expr_start, expr_end, expr = template:find("{{%s*(.-)%s*}}", pos)
        if expr_start and expr_start == pos then
            table.insert(chunks, "table.insert(_result, tostring(" .. expr .. "))\n")
            pos = expr_end + 1
            last_was_code_line = false
        else
            -- Check for % code line at beginning of line (possibly after whitespace)
            local is_code_line = false
            if pos == 1 or template:sub(pos-1, pos-1) == "\n" then
                -- Check if the line starts with whitespace followed by %
                local whitespace_end, percent_pos = template:find("^%s*%%", pos)
                if whitespace_end then
                    local line_end = template:find("\n", percent_pos) or (len + 1)
                    local code = template:sub(percent_pos + 1, line_end - 1)
                    table.insert(chunks, code .. "\n")
                    pos = line_end
                    last_was_code_line = true
                    is_code_line = true
                end
            end

            -- Handle <% code block %>
            if not is_code_line and template:sub(pos, pos+1) == "<%" then
                local block_end = template:find("%%>", pos)
                if block_end then
                    local code = template:sub(pos + 2, block_end - 1)
                    table.insert(chunks, code .. "\n")
                    pos = block_end + 2
                    last_was_code_line = false
                    is_code_line = true
                else
                    -- No closing %>, treat as plain text
                    table.insert(chunks, "table.insert(_result, \"<%\")\n")
                    pos = pos + 2
                    last_was_code_line = false
                end
            end

            -- Process plain text if not a code line
            if not is_code_line then
                -- Process plain text until next special sequence
                local next_expr = template:find("{{", pos) or (len + 1)
                local next_line = len + 1

                -- Find the next line that might have % as first non-whitespace character
                local nl_pos = pos
                while true do
                    local nl = template:find("\n", nl_pos)
                    if not nl then break end

                    -- Check if there's a % after any whitespace on the next line
                    local ws_end, perc_pos = template:find("^%s*%%", nl + 1)
                    if ws_end then
                        next_line = nl + 1
                        break
                    end

                    nl_pos = nl + 1
                end

                local next_block = template:find("<%%", pos) or (len + 1)

                local next_pos = math.min(next_expr, next_line, next_block)

                -- Add plain text
                local text = template:sub(pos, next_pos - 1)

                if text ~= "" then
                    -- Special case: if the last line was code and this text starts with a newline,
                    -- skip that initial newline to avoid empty lines after code
                    if last_was_code_line and text:sub(1, 1) == "\n" then
                        text = text:sub(2)
                    end

                    if text ~= "" then
                        -- Escape quotes and backslashes in the plain text
                        local escaped_text = text:gsub("\\", "\\\\"):gsub("\"", "\\\""):gsub("\n", "\\n")
                        table.insert(chunks, "table.insert(_result, \"" .. escaped_text .. "\")\n")
                    end
                    last_was_code_line = false
                end

                pos = next_pos
            end
        end
    end

    -- Return the final combined result
    table.insert(chunks, "return table.concat(_result)")

    -- Compile the function
    local func_str = table.concat(chunks)
    local func, err = load(func_str, "template", "t")

    if not func then
        return nil, err
    end

    return func
end

-- Renders a template with given environment variables
function template_engine.render(template, env)
    -- Compile the template
    local func, err = template_engine.compile(template)
    if not func then
        return nil, "Compilation error: " .. err
    end

    -- Set up the environment with fallback to _G
    local sandbox = setmetatable({}, {__index = function(t, k)
        return env and env[k] or _G[k]
    end})

    -- Execute the function with the environment
    local success, result = pcall(func, sandbox)

    if not success then
        return nil, "Execution error: " .. result
    end

    return result
end

-- Return the module
return template_engine