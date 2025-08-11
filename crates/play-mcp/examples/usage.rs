use play_mcp::{McpConfig, ClientConfig, RetryConfig, start_mcp_client};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::new("info")
        )
        .init();
    
    // Create MCP configuration
    let config = McpConfig {
        url: "wss://api.xiaozhi.me/mcp/?token=your_token_here".to_string(),
        client: ClientConfig {
            name: "example-mcp-client".to_string(),
            version: "1.0.0".to_string(),
            description: "Example MCP Client".to_string(),
        },
        retry: RetryConfig {
            enabled: true,
            interval_seconds: 5,
            max_attempts: 3,
        },
    };
    
    // Start MCP client
    start_mcp_client(&config).await?;
    
    println!("MCP client completed successfully");
    Ok(())
}