use serde::{Deserialize, Serialize};

/// Main MCP configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct McpConfig {
    pub url: String,
    #[serde(default)]
    pub client: ClientConfig,
    #[serde(default)]
    pub retry: RetryConfig,
    #[serde(default)]
    pub tool_name_prefix: String,
}

/// Client information configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ClientConfig {
    pub name: String,
    pub version: String,
    pub description: String,
}

/// Retry configuration for connection resilience
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RetryConfig {
    pub enabled: bool,
    pub interval_seconds: u64,
    pub max_attempts: u32,
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            url: "ws://localhost:5173/ws".to_string(),  // Default to Xiaozhi endpoint
            client: ClientConfig::default(),
            retry: RetryConfig::default(),
            tool_name_prefix: String::new(),
        }
    }
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            name: "xiaozhi-mcp-client".to_string(),
            version: "0.1.0".to_string(),
            description: "Xiaozhi MCP Integration Client".to_string(),
        }
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_seconds: 5,
            max_attempts: 0,  // 0 means unlimited retries
        }
    }
}

impl McpConfig {
    pub fn builder() -> McpConfigBuilder {
        McpConfigBuilder::default()
    }
}

/// Builder for McpConfig
#[derive(Default)]
pub struct McpConfigBuilder {
    url: Option<String>,
    client: ClientConfig,
    retry: RetryConfig,
    tool_name_prefix: String,
}

impl McpConfigBuilder {
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    pub fn client_name(mut self, name: impl Into<String>) -> Self {
        self.client.name = name.into();
        self
    }

    pub fn client_version(mut self, version: impl Into<String>) -> Self {
        self.client.version = version.into();
        self
    }

    pub fn client_description(mut self, description: impl Into<String>) -> Self {
        self.client.description = description.into();
        self
    }

    pub fn tool_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.tool_name_prefix = prefix.into();
        self
    }

    pub fn retry_enabled(mut self, enabled: bool) -> Self {
        self.retry.enabled = enabled;
        self
    }

    pub fn retry_interval(mut self, seconds: u64) -> Self {
        self.retry.interval_seconds = seconds;
        self
    }

    pub fn max_attempts(mut self, attempts: u32) -> Self {
        self.retry.max_attempts = attempts;
        self
    }

    pub fn build(self) -> McpConfig {
        McpConfig {
            url: self.url.unwrap_or_else(|| McpConfig::default().url),
            client: self.client,
            retry: self.retry,
            tool_name_prefix: self.tool_name_prefix,
        }
    }
}