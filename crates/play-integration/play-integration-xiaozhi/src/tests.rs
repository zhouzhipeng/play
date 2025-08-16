#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::{XiaozhiClient, XiaozhiClientBuilder};

    #[test]
    fn test_builder_default() {
        let builder = XiaozhiClientBuilder::default();
        let client = builder.build();
        
        assert_eq!(client.config().client.name, "xiaozhi-mcp-client");
        assert_eq!(client.config().client.version, "0.1.0");
        assert_eq!(client.config().url, "ws://localhost:5173/ws");
    }

    #[test]
    fn test_builder_custom() {
        let client = XiaozhiClient::builder()
            .name("test-client")
            .version("2.0.0")
            .description("Test client")
            .url("ws://test.example.com/ws")
            .tool_prefix("test:")
            .retry(false)
            .max_attempts(3)
            .build();
        
        assert_eq!(client.config().client.name, "test-client");
        assert_eq!(client.config().client.version, "2.0.0");
        assert_eq!(client.config().client.description, "Test client");
        assert_eq!(client.config().url, "ws://test.example.com/ws");
        assert_eq!(client.config().tool_name_prefix, "test:");
        assert!(!client.config().retry.enabled);
        assert_eq!(client.config().retry.max_attempts, 3);
    }

    #[test]
    fn test_builder_with_duration() {
        use std::time::Duration;
        
        let client = XiaozhiClient::builder()
            .retry_interval(Duration::from_secs(10))
            .build();
        
        assert_eq!(client.config().retry.interval_seconds, 10);
    }
}