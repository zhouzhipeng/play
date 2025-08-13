use anyhow::Result;
use play_mcp::{McpConfig, ClientConfig, RetryConfig, start_mcp_client};

#[tokio::main]
async fn main() -> Result<()> {
    // Example with tool name prefix
    let config = McpConfig {
        url: "ws://localhost:3000/mcp".to_string(),
        client: ClientConfig {
            name: "prefixed-mcp-client".to_string(),
            version: "1.0.0".to_string(),
            description: "MCP client with prefixed tool names".to_string(),
        },
        retry: RetryConfig {
            enabled: true,
            interval_seconds: 5,
            max_attempts: 3,
        },
        tool_name_prefix: "myapp".to_string(), // This will prefix all tools
    };
    
    // With the prefix "myapp", tools will be named:
    // - myapp:get_disk_space
    // - myapp:echo
    // - myapp:system_info
    // - myapp:http_request
    
    println!("Starting MCP client with tool prefix 'myapp'...");
    println!("Tools will be registered as:");
    println!("  - myapp:get_disk_space");
    println!("  - myapp:echo");
    println!("  - myapp:system_info");
    println!("  - myapp:http_request");
    
    start_mcp_client(&config).await?;
    
    Ok(())
}