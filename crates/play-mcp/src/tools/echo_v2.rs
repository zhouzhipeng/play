use anyhow::Result;
use serde_json::{json, Value};

// Using the new define_mcp_tool! macro - much simpler!
crate::define_mcp_tool!(
    EchoToolV2,
    "echo",
    |input: Value| async move {
        let message = input.get("message")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'message' field"))?;
        
        Ok(json!({
            "echoed": message,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }))
    }
);