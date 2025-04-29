local template_engine = require("template_engine")
local htmlutils = require("html_utils")


-- Example usage
local function test_template_engine()
    local template = [[
Hello, {{ name }}
test
 % local count = 10
    % local sum = 0
    % for i = 1, count do
%   sum = sum + i
% end

The sum of numbers 1 to {{ count }} is {{ sum }}.

<%
local products = {
    { name = "Apple", price = 1.5 },
    { name = "Banana", price = 0.5 },
    { name = "Orange", price = 1.2 }
}
%>

Products:
% for i, product in ipairs(products) do
- {{ product.name }}: ${{ product.price }}
% end
]]


    --template = "{{aa}}"

    local env = { name = "John" }
    local result, err = template_engine.render(template, env)

    if result then
        print(result)
    else
        print("Error: " .. err)
    end
end

test_template_engine()


print(11)
-- 使用模块
local escaped = htmlutils.escape("<Hello & World>")
print(escaped)
print(htmlutils.unescape(escaped))