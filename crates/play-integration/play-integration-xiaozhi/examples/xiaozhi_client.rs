use play_integration_xiaozhi::{XiaozhiClient, quick_start};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging (requires tracing_subscriber to be added as dev dependency)
    // For now, we'll use println! for output

    // Example 1: Quick start with default settings
    // Uncomment to use:
    // quick_start("ws://localhost:5173/ws").await?;

    // Example 2: Using the builder pattern
    let client = XiaozhiClient::builder()
        .name("my-xiaozhi-client")
        .version("1.0.0")
        .description("Custom Xiaozhi integration client")
        .url("ws://localhost:5173/ws")
        .tool_prefix("play:")  // Add prefix to all tool names
        .retry(true)
        .max_attempts(10)
        .retry_interval(std::time::Duration::from_secs(3))
        .build();

    println!("Starting Xiaozhi MCP client...");
    println!("Configuration:");
    println!("  Name: {}", client.config().client.name);
    println!("  Version: {}", client.config().client.version);
    println!("  URL: {}", client.config().url);
    println!("  Tool prefix: {}", client.config().tool_name_prefix);
    println!();

    client.start().await?;

    Ok(())
}