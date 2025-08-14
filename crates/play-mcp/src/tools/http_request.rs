use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

use super::{Tool, ToolMetadata};
use crate::impl_tool_with_metadata;

pub struct HttpRequestTool {
    metadata: ToolMetadata,
}

impl_tool_with_metadata!(HttpRequestTool, "http_request");

#[derive(Debug, Serialize, Deserialize)]
pub struct HttpRequestInput {
    pub url: String,
    pub method: Option<String>,
    pub headers: Option<HashMap<String, String>>,
    pub body: Option<String>,
}

#[async_trait]
impl Tool for HttpRequestTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }
    
    async fn execute(&self, input: Value) -> Result<Value> {
        let input: HttpRequestInput = serde_json::from_value(input)?;
        
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
}