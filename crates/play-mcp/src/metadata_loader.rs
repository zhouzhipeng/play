use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use super::ToolMetadata;

// Include the generated validation code
include!(concat!(env!("OUT_DIR"), "/tool_names.rs"));

/// Raw tool definition from JSON
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

/// Root structure of mcp_tools.json
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ToolsConfig {
    tools: HashMap<String, ToolDefinition>,
}

/// Static tools configuration loaded from mcp_tools.json
static TOOLS_CONFIG: Lazy<ToolsConfig> = Lazy::new(|| {
    let json_str = include_str!("mcp_tools.json");
    serde_json::from_str(json_str).expect("Failed to parse mcp_tools.json")
});

/// Load metadata for a specific tool
pub fn load_tool_metadata(tool_name: &str) -> Option<ToolMetadata> {
    TOOLS_CONFIG.tools.get(tool_name).map(|def| {
        ToolMetadata::new(
            def.name.clone(),
            def.description.clone(),
            def.input_schema.clone(),
        )
    })
}

/// Load a tool definition
pub fn load_tool_definition(tool_name: &str) -> Option<ToolDefinition> {
    TOOLS_CONFIG.tools.get(tool_name).cloned()
}

/// Helper macro to create a tool with metadata from mcp_tools.json and auto-register it
#[macro_export]
macro_rules! register_mcp_tool {
    ($struct_name:ident, $tool_key:expr) => {
        impl $struct_name {
            pub fn new() -> Self {
                // Validate tool name at compile time using const function
                const TOOL_NAME: &str = $crate::metadata_loader::validate_tool_name($tool_key);
                
                let metadata = $crate::metadata_loader::load_tool_metadata(TOOL_NAME)
                    .expect(concat!("Failed to load metadata for ", $tool_key));
                Self { metadata }
            }
        }

        impl Default for $struct_name {
            fn default() -> Self {
                Self::new()
            }
        }
        
        // Auto-register the tool factory
        #[linkme::distributed_slice($crate::tools::TOOL_FACTORIES)]
        static REGISTER_TOOL: $crate::tools::ToolFactory = || {
            Box::new($struct_name::new())
        };
    };
}

/// Get all available tool names
pub fn get_all_tool_names() -> Vec<String> {
    TOOLS_CONFIG.tools.keys().cloned().collect()
}

/// Check if a tool exists
pub fn tool_exists(tool_name: &str) -> bool {
    TOOLS_CONFIG.tools.contains_key(tool_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_tool_metadata() {
        let metadata = load_tool_metadata("echo");
        assert!(metadata.is_some());
        let metadata = metadata.unwrap();
        assert_eq!(metadata.name, "echo");
    }

    #[test]
    fn test_get_all_tool_names() {
        let names = get_all_tool_names();
        assert!(names.contains(&"echo".to_string()));
        assert!(names.contains(&"sys_info".to_string()));
    }
}