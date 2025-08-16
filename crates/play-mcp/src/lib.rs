pub mod tools;
pub mod metadata_loader;

pub use tools::ToolRegistry;
// The define_mcp_tool macro is exported at the crate root via #[macro_export]

// Note: MCP protocol types (JsonRpcRequest, JsonRpcResponse, JsonRpcError) and 
// configuration types (McpConfig, ClientConfig, RetryConfig) have been moved to 
// play-integration-xiaozhi crate for better separation of concerns.
// 
// This crate now focuses exclusively on tool definitions and the tool registry.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_tool_names() {
        // Test valid tool names
        assert!(metadata_loader::validate_tool_name_chars("simple_tool"));
        assert!(metadata_loader::validate_tool_name_chars("tool_123"));
        assert!(metadata_loader::validate_tool_name_chars("v2_tool"));
        assert!(metadata_loader::validate_tool_name_chars("tool.with.dots"));
        assert!(metadata_loader::validate_tool_name_chars("tool:with:colons"));
        assert!(metadata_loader::validate_tool_name_chars("complex_tool.v2:feature_123"));
        assert!(metadata_loader::validate_tool_name_chars("api_v2"));
        assert!(metadata_loader::validate_tool_name_chars("tool_2024"));
    }

    #[test]
    fn test_invalid_tool_names() {
        // Test invalid tool names
        assert!(!metadata_loader::validate_tool_name_chars("Tool")); // uppercase not allowed
        assert!(!metadata_loader::validate_tool_name_chars("tool-with-dash")); // dash not allowed
        assert!(!metadata_loader::validate_tool_name_chars("tool with space")); // space not allowed
        assert!(!metadata_loader::validate_tool_name_chars("tool@special")); // @ not allowed
        assert!(!metadata_loader::validate_tool_name_chars("tool#hash")); // # not allowed
        assert!(!metadata_loader::validate_tool_name_chars("TOOL_UPPERCASE")); // uppercase not allowed
        assert!(!metadata_loader::validate_tool_name_chars("tool/slash")); // slash not allowed
        assert!(!metadata_loader::validate_tool_name_chars("tool(parens)")); // parentheses not allowed
    }
}