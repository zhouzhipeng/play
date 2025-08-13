use anyhow::Result;
use play_mcp::{McpConfig, ClientConfig, RetryConfig, ToolsConfig, start_mcp_client};

#[tokio::main]
async fn main() -> Result<()> {
    // Example 1: Only enable specific tools
    let config_selective = McpConfig {
        url: "ws://localhost:3000/mcp".to_string(),
        client: ClientConfig {
            name: "selective-mcp-client".to_string(),
            version: "1.0.0".to_string(),
            description: "MCP client with selected tools".to_string(),
        },
        retry: RetryConfig {
            enabled: true,
            max_attempts: 5,
            interval_seconds: 5,
        },
        tools: ToolsConfig {
            // Only enable echo and system_info tools
            enabled: vec![
                "echo".to_string(),
                "system_info".to_string(),
            ],
        },
    };
    
    println!("Starting MCP client with selective tools (echo, system_info)...");
    // start_mcp_client(&config_selective).await?;
    
    // Example 2: Use all default tools (empty means all)
    let config_all = McpConfig {
        url: "ws://localhost:3000/mcp".to_string(),
        client: ClientConfig {
            name: "all-tools-mcp-client".to_string(),
            version: "1.0.0".to_string(),
            description: "MCP client with all tools".to_string(),
        },
        retry: RetryConfig::default(),
        tools: ToolsConfig::default(), // Uses default which includes all tools
    };
    
    println!("Starting MCP client with all default tools...");
    start_mcp_client(&config_all).await?;
    
    Ok(())
}