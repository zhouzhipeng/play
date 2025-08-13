use anyhow::Result;
use serde_json::{json, Value};
use std::sync::Arc;

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
    
    async fn execute(&self, input: Value) -> Result<Value>;
}

pub enum AnyTool {
    DiskSpace(DiskSpaceTool),
    Echo(EchoTool),
    SystemInfo(SystemInfoTool),
    HttpRequest(HttpRequestTool),
}

impl Tool for AnyTool {
    fn name(&self) -> &str {
        match self {
            AnyTool::DiskSpace(t) => t.name(),
            AnyTool::Echo(t) => t.name(),
            AnyTool::SystemInfo(t) => t.name(),
            AnyTool::HttpRequest(t) => t.name(),
        }
    }
    
    fn description(&self) -> &str {
        match self {
            AnyTool::DiskSpace(t) => t.description(),
            AnyTool::Echo(t) => t.description(),
            AnyTool::SystemInfo(t) => t.description(),
            AnyTool::HttpRequest(t) => t.description(),
        }
    }
    
    fn input_schema(&self) -> Value {
        match self {
            AnyTool::DiskSpace(t) => t.input_schema(),
            AnyTool::Echo(t) => t.input_schema(),
            AnyTool::SystemInfo(t) => t.input_schema(),
            AnyTool::HttpRequest(t) => t.input_schema(),
        }
    }
    
    async fn execute(&self, input: Value) -> Result<Value> {
        match self {
            AnyTool::DiskSpace(t) => t.execute(input).await,
            AnyTool::Echo(t) => t.execute(input).await,
            AnyTool::SystemInfo(t) => t.execute(input).await,
            AnyTool::HttpRequest(t) => t.execute(input).await,
        }
    }
}

pub struct ToolRegistry {
    tools: Vec<Arc<AnyTool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: Vec::new(),
        }
    }
    
    pub fn register(&mut self, tool: AnyTool) {
        self.tools.push(Arc::new(tool));
    }
    
    pub fn register_by_names(&mut self, tool_names: &[String]) {
        for name in tool_names {
            if let Some(tool) = create_tool_by_name(name) {
                self.register(tool);
            } else {
                eprintln!("Warning: Unknown tool '{}', skipping", name);
            }
        }
    }
    
    pub fn get(&self, name: &str) -> Option<Arc<AnyTool>> {
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

/// Create a tool instance by its name
pub fn create_tool_by_name(name: &str) -> Option<AnyTool> {
    match name {
        "get_disk_space" | "disk_space" => Some(AnyTool::DiskSpace(DiskSpaceTool)),
        "echo" => Some(AnyTool::Echo(EchoTool)),
        "system_info" => Some(AnyTool::SystemInfo(SystemInfoTool)),
        "http_request" => Some(AnyTool::HttpRequest(HttpRequestTool)),
        _ => None,
    }
}

/// Get all available tool names
pub fn get_available_tools() -> Vec<&'static str> {
    vec![
        "get_disk_space",
        "echo",
        "system_info",
        "http_request",
    ]
}