use anyhow::Result;
use serde_json::{json, Value};
use std::sync::Arc;
use std::pin::Pin;
use std::future::Future;

mod disk_space;
mod echo;
mod system_info;
mod http_request;

pub use disk_space::{DiskSpaceTool, DiskSpaceInput, DiskSpaceResult};
pub use echo::EchoTool;
pub use system_info::SystemInfoTool;
pub use http_request::{HttpRequestTool, HttpRequestInput};

pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    
    fn description(&self) -> &str;
    
    fn input_schema(&self) -> Value;
    
    fn execute(&self, input: Value) -> Pin<Box<dyn Future<Output = Result<Value>> + Send + '_>>;
}

/// Wrapper for tools with a name prefix
struct PrefixedTool {
    tool: Box<dyn Tool>,
    prefixed_name: String,
}

impl Tool for PrefixedTool {
    fn name(&self) -> &str {
        &self.prefixed_name
    }
    
    fn description(&self) -> &str {
        self.tool.description()
    }
    
    fn input_schema(&self) -> Value {
        self.tool.input_schema()
    }
    
    fn execute(&self, input: Value) -> Pin<Box<dyn Future<Output = Result<Value>> + Send + '_>> {
        self.tool.execute(input)
    }
}

pub struct ToolRegistry {
    tools: Vec<Arc<Box<dyn Tool>>>,
    name_prefix: String,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: Vec::new(),
            name_prefix: String::new(),
        }
    }
    
    pub fn with_prefix(prefix: String) -> Self {
        Self {
            tools: Vec::new(),
            name_prefix: prefix,
        }
    }
    
    pub fn register(&mut self, tool: Box<dyn Tool>) {
        let tool = if self.name_prefix.is_empty() {
            tool
        } else {
            let prefixed_name = format!("{}:{}", self.name_prefix, tool.name());
            Box::new(PrefixedTool {
                tool,
                prefixed_name,
            })
        };
        self.tools.push(Arc::new(tool));
    }
    
    pub fn get(&self, name: &str) -> Option<Arc<Box<dyn Tool>>> {
        self.tools.iter()
            .find(|t| t.name() == name)
            .cloned()
    }
    
    pub fn list(&self) -> Vec<Value> {
        self.tools.iter()
            .map(|tool| {
                json!({
                    "name": tool.name(),
                    "description": tool.description(),
                    "inputSchema": tool.input_schema(),
                })
            })
            .collect()
    }
}