use anyhow::Result;
use async_trait::async_trait;
use linkme::distributed_slice;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;


mod http_request;


/// Tool metadata containing static information about a tool or operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetadata {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

impl ToolMetadata {
    pub fn new(name: impl Into<String>, description: impl Into<String>, input_schema: Value) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            input_schema,
        }
    }
}

/// Function type for creating tool instances
pub type ToolFactory = fn() -> Box<dyn Tool>;

/// Distributed slice for auto-registering tools
#[distributed_slice]
pub static TOOL_FACTORIES: [ToolFactory] = [..];

#[async_trait]
pub trait Tool: Send + Sync {
    /// Returns the metadata for this tool
    fn metadata(&self) -> &ToolMetadata;

    /// Execute the tool with the given input
    async fn execute(&self, input: Value) -> Result<Value>;
}

pub struct ToolRegistry {
    tools: Vec<Arc<dyn Tool>>,
    name_prefix: String,
}

impl ToolRegistry {
    pub fn new() -> Self {
        // Auto-register all tools from the distributed slice
        let mut tools = Vec::new();
        let mut seen_names = std::collections::HashSet::new();
        
        for factory in TOOL_FACTORIES {
            let tool = factory();
            let tool_name = &tool.metadata().name;
            
            // Check for duplicate tool registrations
            if !seen_names.insert(tool_name.clone()) {
                panic!(
                    "Duplicate tool registration detected: '{}'. Each tool can only be registered once.",
                    tool_name
                );
            }
            
            tools.push(Arc::from(tool));
        }
        
        Self {
            tools,
            name_prefix: String::new(),
        }
    }
    
    pub fn with_prefix(prefix: String) -> Self {
        // Auto-register all tools from the distributed slice
        let mut tools = Vec::new();
        let mut seen_names = std::collections::HashSet::new();
        
        for factory in TOOL_FACTORIES {
            let tool = factory();
            let tool_name = &tool.metadata().name;
            
            // Check for duplicate tool registrations
            if !seen_names.insert(tool_name.clone()) {
                panic!(
                    "Duplicate tool registration detected: '{}'. Each tool can only be registered once.",
                    tool_name
                );
            }
            
            tools.push(Arc::from(tool));
        }
        
        Self {
            tools,
            name_prefix: prefix,
        }
    }
    
    pub fn register(&mut self, tool: Box<dyn Tool>) {
        self.tools.push(Arc::from(tool));
    }
    
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        if !self.name_prefix.is_empty() {
            // When prefix is set, only accept names that start with the prefix
            if !name.starts_with(&self.name_prefix) {
                return None;
            }
            // Strip the prefix and search for the base tool name
            let search_name = &name[self.name_prefix.len()..];
            self.tools.iter()
                .find(|t| t.metadata().name == search_name)
                .cloned()
        } else {
            // No prefix, direct name match
            self.tools.iter()
                .find(|t| t.metadata().name == name)
                .cloned()
        }
    }
    
    pub fn list(&self) -> Vec<Value> {
        self.tools.iter().map(|tool| {
            let metadata = tool.metadata();
            // Apply prefix to the tool name when listing
            let name_with_prefix = if self.name_prefix.is_empty() {
                metadata.name.clone()
            } else {
                format!("{}{}", self.name_prefix, metadata.name)
            };
            
            json!({
                "name": name_with_prefix,
                "description": metadata.description,
                "inputSchema": metadata.input_schema,
            })
        }).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_without_prefix() {
        let registry = ToolRegistry::new();
        let tools = registry.list();
        
        // Should have some tools registered
        assert!(!tools.is_empty());
        
        // Tool names should not have any prefix
        for tool in &tools {
            if let Some(name) = tool.get("name").and_then(|v| v.as_str()) {
                // Check a known tool
                if name == "echo" {
                    // Should be able to get it by exact name
                    assert!(registry.get("echo").is_some());
                    // Should not be able to get it with a prefix
                    assert!(registry.get("prefix:echo").is_none());
                    break;
                }
            }
        }
    }

    #[test]
    fn test_registry_with_prefix() {
        let prefix = "myapp:";
        let registry = ToolRegistry::with_prefix(prefix.to_string());
        let tools = registry.list();
        
        // Should have some tools registered
        assert!(!tools.is_empty());
        
        // All tool names should have the prefix
        for tool in &tools {
            if let Some(name) = tool.get("name").and_then(|v| v.as_str()) {
                assert!(name.starts_with(prefix), "Tool name '{}' should start with prefix '{}'", name, prefix);
            }
        }
        
        // Test getting tools with prefix
        if let Some(first_tool) = tools.first() {
            if let Some(prefixed_name) = first_tool.get("name").and_then(|v| v.as_str()) {
                // Should be able to get with full prefixed name
                assert!(registry.get(prefixed_name).is_some(), "Should find tool with prefixed name");
                
                // Should NOT be able to get without prefix
                let unprefixed_name = &prefixed_name[prefix.len()..];
                assert!(registry.get(unprefixed_name).is_none(), "Should not find tool without prefix");
            }
        }
    }

    #[test]
    fn test_prefix_stripping() {
        let prefix = "test.v1.";
        let registry = ToolRegistry::with_prefix(prefix.to_string());
        
        // Assuming we have an "echo" tool registered
        let tools = registry.list();
        let has_echo = tools.iter().any(|t| {
            t.get("name")
                .and_then(|v| v.as_str())
                .map(|name| name == "test.v1.echo")
                .unwrap_or(false)
        });
        
        if has_echo {
            // Should work with full prefixed name
            assert!(registry.get("test.v1.echo").is_some());
            
            // Should not work with partial prefix
            assert!(registry.get("test.echo").is_none());
            assert!(registry.get("v1.echo").is_none());
            
            // Should not work without any prefix
            assert!(registry.get("echo").is_none());
        }
    }
}