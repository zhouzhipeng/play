use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct HttpRequestInput {
    pub url: String,
    pub method: Option<String>,
    pub headers: Option<HashMap<String, String>>,
    pub body: Option<String>,
}

crate::define_mcp_tool!(
    HttpRequestTool,
    "http_request",
    |input: HttpRequestInput| async move {
        // let input: HttpRequestInput = serde_json::from_value(input)?;
        
        // This is a mock implementation
        // In a real implementation, you would use reqwest or similar
        Ok(json!({
            "status": 200,
            "headers": {},
            "body": format!("Mock response for {} request to {}", 
                input.method.unwrap_or_else(|| "GET".to_string()), 
                input.url),
            "error": null
        }))
    }
);