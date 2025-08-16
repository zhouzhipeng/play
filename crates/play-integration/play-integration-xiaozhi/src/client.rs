use anyhow::Result;
use play_mcp::tools::ToolRegistry;
use crate::config::{McpConfig, ClientConfig, RetryConfig};
use std::time::Duration;

/// Builder for creating Xiaozhi MCP client
pub struct XiaozhiClientBuilder {
    name: String,
    version: String,
    description: String,
    url: String,
    tool_name_prefix: String,
    retry_enabled: bool,
    retry_max_attempts: u32,
    retry_interval: Duration,
    custom_registry: Option<ToolRegistry>,
}

impl Default for XiaozhiClientBuilder {
    fn default() -> Self {
        Self {
            name: "xiaozhi-mcp-client".to_string(),
            version: "0.1.0".to_string(),
            description: "Xiaozhi MCP Integration Client".to_string(),
            url: "ws://localhost:5173/ws".to_string(),
            tool_name_prefix: String::new(),
            retry_enabled: true,
            retry_max_attempts: 5,
            retry_interval: Duration::from_secs(5),
            custom_registry: None,
        }
    }
}

impl XiaozhiClientBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }
    
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }
    
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }
    
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = url.into();
        self
    }
    
    pub fn tool_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.tool_name_prefix = prefix.into();
        self
    }
    
    pub fn retry(mut self, enabled: bool) -> Self {
        self.retry_enabled = enabled;
        self
    }
    
    pub fn max_attempts(mut self, attempts: u32) -> Self {
        self.retry_max_attempts = attempts;
        self
    }
    
    pub fn retry_interval(mut self, interval: Duration) -> Self {
        self.retry_interval = interval;
        self
    }
    
    pub fn with_tools(mut self, registry: ToolRegistry) -> Self {
        self.custom_registry = Some(registry);
        self
    }
    
    pub fn build(self) -> XiaozhiClient {
        let config = McpConfig {
            client: ClientConfig {
                name: self.name,
                version: self.version,
                description: self.description,
            },
            url: self.url,
            tool_name_prefix: self.tool_name_prefix,
            retry: RetryConfig {
                enabled: self.retry_enabled,
                max_attempts: self.retry_max_attempts,
                interval_seconds: self.retry_interval.as_secs(),
            },
        };
        
        XiaozhiClient {
            config,
            custom_registry: self.custom_registry,
        }
    }
}

/// Xiaozhi MCP client
pub struct XiaozhiClient {
    config: McpConfig,
    custom_registry: Option<ToolRegistry>,
}

impl XiaozhiClient {
    /// Create a new client builder
    pub fn builder() -> XiaozhiClientBuilder {
        XiaozhiClientBuilder::new()
    }
    
    /// Start the client with default tools
    pub async fn start(self) -> Result<()> {
        if let Some(registry) = self.custom_registry {
            crate::start_xiaozhi_client_with_tools(&self.config, registry).await
        } else {
            crate::start_xiaozhi_client(&self.config).await
        }
    }
    
    /// Get the configuration
    pub fn config(&self) -> &McpConfig {
        &self.config
    }
}

/// Quick start function for Xiaozhi integration
pub async fn quick_start(url: &str) -> Result<()> {
    let client = XiaozhiClient::builder()
        .url(url)
        .name("play-xiaozhi-integration")
        .description("Play framework integration with Xiaozhi AI")
        .build();
    
    client.start().await
}