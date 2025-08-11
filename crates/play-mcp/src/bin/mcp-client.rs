use anyhow::{Result, Context};
use clap::Parser;
use config::{Config, File};
use play_mcp::{McpConfig, ClientConfig, RetryConfig, start_mcp_client};
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

#[derive(Debug, serde::Deserialize)]
struct AppConfig {
    mcp_server: McpServerConfig,
    client: ClientConfig,
    retry: RetryConfig,
}

#[derive(Debug, serde::Deserialize)]
struct McpServerConfig {
    url: String,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Config file path
    #[arg(short, long, default_value = "config.json")]
    config: PathBuf,
    
    /// Override WebSocket URL of the MCP server endpoint
    #[arg(short, long)]
    url: Option<String>,

    /// Override client name for identification
    #[arg(short, long)]
    name: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info"))
        )
        .init();
    
    let args = Args::parse();
    
    // Load configuration
    let config = Config::builder()
        .add_source(File::from(args.config.clone()))
        .build()
        .context("Failed to load config file")?;
    
    let app_config: AppConfig = config.try_deserialize()
        .context("Failed to parse config file")?;
    
    // Override with command line args if provided
    let url = args.url.unwrap_or(app_config.mcp_server.url);
    let mut client_config = app_config.client;
    if let Some(name) = args.name {
        client_config.name = name;
    }
    
    let mcp_config = McpConfig {
        url,
        client: client_config,
        retry: app_config.retry,
    };
    
    start_mcp_client(&mcp_config).await
        .context("MCP client failed")?;
    
    Ok(())
}