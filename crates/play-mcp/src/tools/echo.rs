use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};

use super::{Tool, ToolMetadata};
use crate::impl_tool_with_metadata;

pub struct EchoTool {
    metadata: ToolMetadata,
}

impl_tool_with_metadata!(EchoTool, "echo");

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