use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};

use super::{Tool, ToolMetadata};

pub struct EchoTool {
    metadata: ToolMetadata,
}

impl EchoTool {
    pub fn new() -> Self {
        Self {
            metadata: ToolMetadata::new(
                "echo",
                "Echo back the input message",
                json!({
                    "type": "object",
                    "properties": {
                        "message": {
                            "type": "string",
                            "description": "The message to echo back"
                        }
                    },
                    "required": ["message"]
                })
            ),
        }
    }
}

impl Default for EchoTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for EchoTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }
    
    async fn execute(&self, input: Value) -> Result<Value> {
        let message = input.get("message")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'message' field"))?;
        
        Ok(json!({
            "echoed": message,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }))
    }
}