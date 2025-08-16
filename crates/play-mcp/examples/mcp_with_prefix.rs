use play_mcp::{config::*, start_mcp_client};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("play_mcp=info".parse()?))
        .init();

    // Example 1: MCP client with namespace prefix
    println!("Example MCP client configurations with tool name prefix:\n");
    
    // Configuration with a namespace prefix
    let config_with_namespace = McpConfig {
        url: "ws://localhost:8080/mcp".to_string(),
        client: ClientConfig {
            name: "MyApp MCP Client".to_string(),
            version: "1.0.0".to_string(),
            description: "MCP client with namespaced tools".to_string(),
        },
        retry: RetryConfig {
            enabled: false,
            max_attempts: 3,
            interval_seconds: 5,
        },
        tool_name_prefix: "myapp:".to_string(),  // All tools will be prefixed with "myapp:"
    };
    
    println!("Configuration 1 - With namespace prefix 'myapp:':");
    println!("  Tools will be exposed as:");
    println!("    - myapp:echo");
    println!("    - myapp:http_request");
    println!("    - myapp:sys_info");
    println!("  etc.\n");
    
    // Configuration with version prefix
    let config_with_version = McpConfig {
        url: "ws://localhost:8080/mcp".to_string(),
        client: ClientConfig {
            name: "Versioned MCP Client".to_string(),
            version: "2.0.0".to_string(),
            description: "MCP client with versioned tools".to_string(),
        },
        retry: RetryConfig::default(),
        tool_name_prefix: "v2.".to_string(),  // All tools will be prefixed with "v2."
    };
    
    println!("Configuration 2 - With version prefix 'v2.':");
    println!("  Tools will be exposed as:");
    println!("    - v2.echo");
    println!("    - v2.http_request");
    println!("    - v2.sys_info");
    println!("  etc.\n");
    
    // Configuration with environment prefix
    let config_with_env = McpConfig {
        url: "ws://localhost:8080/mcp".to_string(),
        client: ClientConfig {
            name: "Production MCP Client".to_string(),
            version: "1.0.0".to_string(),
            description: "MCP client for production environment".to_string(),
        },
        retry: RetryConfig::default(),
        tool_name_prefix: "prod:tools:".to_string(),  // All tools will be prefixed with "prod:tools:"
    };
    
    println!("Configuration 3 - With environment prefix 'prod:tools:':");
    println!("  Tools will be exposed as:");
    println!("    - prod:tools:echo");
    println!("    - prod:tools:http_request");
    println!("    - prod:tools:sys_info");
    println!("  etc.\n");
    
    // Configuration without prefix (default behavior)
    let config_no_prefix = McpConfig {
        url: "ws://localhost:8080/mcp".to_string(),
        client: ClientConfig {
            name: "Standard MCP Client".to_string(),
            version: "1.0.0".to_string(),
            description: "MCP client without tool prefix".to_string(),
        },
        retry: RetryConfig::default(),
        tool_name_prefix: String::new(),  // No prefix
    };
    
    println!("Configuration 4 - Without prefix:");
    println!("  Tools will be exposed with original names:");
    println!("    - echo");
    println!("    - http_request");
    println!("    - sys_info");
    println!("  etc.\n");
    
    println!("Benefits of using tool name prefix:");
    println!("1. Namespace isolation: Avoid conflicts when multiple clients expose similar tools");
    println!("2. Version management: Run multiple versions of tools simultaneously");
    println!("3. Environment separation: Distinguish between dev/staging/prod tools");
    println!("4. Multi-tenancy: Support multiple applications in the same MCP server");
    
    println!("\nTo start the MCP client with a prefix, uncomment one of the following:");
    println!("// start_mcp_client(&config_with_namespace).await?;");
    println!("// start_mcp_client(&config_with_version).await?;");
    println!("// start_mcp_client(&config_with_env).await?;");
    println!("// start_mcp_client(&config_no_prefix).await?;");
    
    Ok(())
}