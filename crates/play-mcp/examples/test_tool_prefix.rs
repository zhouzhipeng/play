use play_mcp::define_mcp_tool;
use play_mcp::tools::ToolRegistry;
use serde_json::json;

// Define a simple test tool
define_mcp_tool!(
    EchoTool,
    "echo",
    |message: String| {
        Ok(json!({
            "echo": message
        }))
    }
);

fn main() {
    println!("Testing tool name prefix functionality\n");
    
    // Test 1: Registry without prefix
    println!("=== Test 1: No prefix ===");
    let registry_no_prefix = ToolRegistry::new();
    let tools_no_prefix = registry_no_prefix.list();
    
    for tool in &tools_no_prefix {
        if let Some(name) = tool.get("name").and_then(|v| v.as_str()) {
            if name == "echo" {
                println!("Found tool: {}", name);
                
                // Test getting the tool
                if let Some(_tool) = registry_no_prefix.get("echo") {
                    println!("✓ Successfully retrieved tool 'echo'");
                } else {
                    println!("✗ Failed to retrieve tool 'echo'");
                }
                break;
            }
        }
    }
    
    // Test 2: Registry with prefix
    println!("\n=== Test 2: With prefix 'my_app:' ===");
    let registry_with_prefix = ToolRegistry::with_prefix("my_app:".to_string());
    let tools_with_prefix = registry_with_prefix.list();
    
    for tool in &tools_with_prefix {
        if let Some(name) = tool.get("name").and_then(|v| v.as_str()) {
            if name.contains("echo") {
                println!("Found tool with prefix: {}", name);
                
                // Test getting the tool with full prefixed name
                if let Some(_tool) = registry_with_prefix.get("my_app:echo") {
                    println!("✓ Successfully retrieved tool 'my_app:echo'");
                } else {
                    println!("✗ Failed to retrieve tool 'my_app:echo'");
                }
                
                // Test getting the tool without prefix (should not work)
                if let Some(_tool) = registry_with_prefix.get("echo") {
                    println!("✗ Incorrectly retrieved tool with 'echo' (without prefix)");
                } else {
                    println!("✓ Correctly failed to retrieve tool 'echo' (without prefix)");
                }
                break;
            }
        }
    }
    
    // Test 3: List all tools with prefix
    println!("\n=== Test 3: List all tools with prefix 'test.' ===");
    let registry_test_prefix = ToolRegistry::with_prefix("test.".to_string());
    let all_tools = registry_test_prefix.list();
    
    println!("Total tools: {}", all_tools.len());
    println!("First 5 tools with prefix:");
    for (i, tool) in all_tools.iter().take(5).enumerate() {
        if let Some(name) = tool.get("name").and_then(|v| v.as_str()) {
            println!("  {}. {}", i + 1, name);
        }
    }
    
    println!("\n✅ Tool name prefix testing completed!");
}