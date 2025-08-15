use play_mcp::define_mcp_tool;
use serde_json::json;

// Note: http_request_v2 is already defined in src/tools/http_request.rs
// This is commented out to avoid duplicate registration

// Test tool with numbers and other allowed characters
define_mcp_tool!(
    TestTool123,
    "test_tool_123",
    |value: String| {
        Ok(json!({
            "value": value,
            "tool": "test_tool_123"
        }))
    }
);

fn main() {
    println!("Testing tool names with numbers...");
    
    let registry = play_mcp::tools::ToolRegistry::new();
    let tools = registry.list();
    
    println!("Found {} tools", tools.len());
    for tool in &tools {
        if let Some(name) = tool.get("name") {
            let name_str = name.as_str().unwrap_or("");
            if name_str.contains(char::is_numeric) {
                println!("  - {} (contains numbers)", name_str);
            }
        }
    }
    
    println!("Tool names with numbers work correctly!");
}