use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;

#[derive(Debug,Serialize, Deserialize)]
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



#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::Tool;

    #[tokio::test]
    async fn test_http_request()->anyhow::Result<()> {
        let r = HttpRequestTool::new().execute(serde_json::to_value(HttpRequestInput {
            url: "/a/ab".to_string(),
            method: None,
            headers: None,
            body: None,
        })?).await;
        println!("{:?}", r);
        assert!(r.is_ok());
        Ok(())
    }

}