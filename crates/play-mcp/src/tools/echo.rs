use anyhow::Result;
use serde_json::{json, Value};

use super::Tool;
use std::pin::Pin;
use std::future::Future;

pub struct EchoTool;

impl Tool for EchoTool {
    fn name(&self) -> &str {
        "echo"
    }
    
    fn description(&self) -> &str {
        "Echo back the input message"
    }
    
    fn input_schema(&self) -> Value {
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
    }
    
    fn execute(&self, input: Value) -> Pin<Box<dyn Future<Output = Result<Value>> + Send + '_>> {
        Box::pin(async move {
            let message = input.get("message")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Missing 'message' field"))?;
            
            Ok(json!({
                "echoed": message,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
        })
    }
}