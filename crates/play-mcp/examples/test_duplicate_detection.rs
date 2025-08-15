use play_mcp::define_mcp_tool;
use serde_json::json;

// First registration of "echo"
define_mcp_tool!(
    EchoTool1,
    "echo",
    |message: String| {
        Ok(json!({
            "echo": message
        }))
    }
);

// Second registration of "echo" - this should cause a runtime panic when ToolRegistry is created
define_mcp_tool!(
    EchoTool2,
    "echo",
    |message: String| {
        Ok(json!({
            "echo": format!("duplicate: {}", message)
        }))
    }
);

fn main() {
    println!("Testing duplicate tool detection...");
    
    // This should panic with duplicate registration error
    let registry = play_mcp::tools::ToolRegistry::new();
    
    println!("This line should never be reached!");
}