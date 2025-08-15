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
        self.tools.iter()
            .find(|t| t.metadata().name == name)
            .cloned()
    }
    
    pub fn list(&self) -> Vec<Value> {
        self.tools.iter().map(|tool| {
            let metadata = tool.metadata();
            json!({
                "name": metadata.name,
                "description": metadata.description,
                "inputSchema": metadata.input_schema,
            })
        }).collect()
    }
}