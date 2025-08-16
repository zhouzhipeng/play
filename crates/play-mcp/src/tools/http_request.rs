use crate::define_mcp_tool;
use anyhow::bail;
use serde_json::json;
use std::collections::HashMap;
use crate::tools::Tool;

define_mcp_tool!(
    "http_request",
    |url: String,
     method: Option<String>,
     headers: Option<HashMap<String, String>>,
     body: Option<String>| {
        // This is a mock implementation
        // In a real implementation, you would use reqwest or similar

        Ok(json!({
            "status": 200,
            "headers": {},
            "body": format!("Mock response for {} request to {}",
                method.unwrap_or_else(|| "GET".to_string()),
                url),
            "error": null
        }))
    }
);
