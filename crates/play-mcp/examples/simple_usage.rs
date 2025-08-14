use play_mcp::{ToolRegistry, McpConfig, start_mcp_client};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Example 1: List all auto-registered tools
    println!("All available tools (auto-registered):\n");
    
    let registry = ToolRegistry::new(); // All tools are automatically included!
    
    for tool in registry.list() {
        if let Some(name) = tool.get("name").and_then(|v| v.as_str()) {
            println!("  - {}", name);
        }
    }
    
    println!("\n{} tools are automatically available!", registry.list().len());
    
    // Example 2: Start MCP client with auto-registered tools
    println!("\nTo start an MCP client, just use:");
    println!("  let config = McpConfig {{ ... }};");
    println!("  start_mcp_client(&config).await?;");
    println!("\nNo manual registration needed!");
    
    Ok(())
}