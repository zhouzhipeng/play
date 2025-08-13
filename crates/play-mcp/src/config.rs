use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ClientConfig {
    pub name: String,
    pub version: String,
    pub description: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RetryConfig {
    pub enabled: bool,
    pub interval_seconds: u64,
    pub max_attempts: u32,
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            url: "ws://localhost:8765".to_string(),
            client: ClientConfig::default(),
            retry: RetryConfig::default(),
            tool_name_prefix: String::new(),
        }
    }
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            name: "play-mcp-client".to_string(),
            version: "0.1.0".to_string(),
            description: "MCP Client".to_string(),
        }
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_seconds: 5,
            max_attempts: 0,
        }
    }
}