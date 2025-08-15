use play_mcp::define_mcp_tool;
use serde_json::json;

// First tool without explicit struct name
define_mcp_tool!(
    "echo",
    |message: String| {
        Ok(json!({
            "echo": message
        }))
    }
);

// Second tool without explicit struct name - tests that multiple auto-generated tools work
define_mcp_tool!(
    "sys_info",
    |_dummy: serde_json::Value| {
        Ok(json!({
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH
        }))
    }
);

// Tool with explicit struct name for comparison
define_mcp_tool!(
    MyExplicitTool,
    "http_request",
    |url: String, method: Option<String>, headers: Option<std::collections::HashMap<String, String>>, body: Option<String>| {
        Ok(json!({
            "url": url,
            "method": method.unwrap_or("GET".to_string()),
            "headers": headers,
            "body": body
        }))
    }
);

fn main() {
    println!("Multiple auto-generated tools compiled successfully!");
    
    // Verify tools are registered
    let registry = play_mcp::tools::ToolRegistry::new();
    let tools = registry.list();
    
    println!("Registered {} tools", tools.len());
    for tool in tools {
        if let Some(name) = tool.get("name") {
            println!("  - {}", name);
        }
    }
}