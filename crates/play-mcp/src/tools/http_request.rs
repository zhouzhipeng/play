use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

use super::Tool;
use std::pin::Pin;
use std::future::Future;

pub struct HttpRequestTool;

#[derive(Debug, Serialize, Deserialize)]
pub struct HttpRequestInput {
    pub url: String,
    pub method: Option<String>,
    pub headers: Option<HashMap<String, String>>,
    pub body: Option<String>,
}

impl Tool for HttpRequestTool {
    fn name(&self) -> &str {
        "http_request"
    }
    
    fn description(&self) -> &str {
        "Make an HTTP request to a URL"
    }
    
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "The URL to request"
                },
                "method": {
                    "type": "string",
                    "description": "HTTP method (GET, POST, PUT, DELETE, etc.)",
                    "default": "GET"
                },
                "headers": {
                    "type": "object",
                    "description": "Optional HTTP headers",
                    "additionalProperties": {
                        "type": "string"
                    }
                },
                "body": {
                    "type": "string",
                    "description": "Optional request body"
                }
            },
            "required": ["url"]
        })
    }
    
    fn execute(&self, input: Value) -> Pin<Box<dyn Future<Output = Result<Value>> + Send + '_>> {
        Box::pin(async move {
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
        })
    }
}