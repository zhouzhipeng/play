use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;

crate::define_mcp_tool!(
    HttpRequestTool,
    "http_request",
    |url: String,
     method: Option<String>,
     headers: Option<HashMap<String, String>>,
     body: Option<String>| async move {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::Tool;

    #[tokio::test]
    async fn test_http_request() -> anyhow::Result<()> {
        let r = HttpRequestTool::new()
            .execute(json!( {
                "url": "/a/ab",
                "method": "GET",
            }))
            .await;
        println!("{:?}", r);
        assert!(r.is_ok());
        Ok(())
    }

    #[tokio::test]
    async fn test_http_request_minimal() -> anyhow::Result<()> {
        // Test with only required field (url), all Options should be None
        let r = HttpRequestTool::new()
            .execute(json!( {
                "url": "/api/test",
            }))
            .await;
        println!("Minimal test: {:?}", r);
        assert!(r.is_ok());
        assert!(r.unwrap()["body"].as_str().unwrap().contains("GET request"));
        Ok(())
    }
}
