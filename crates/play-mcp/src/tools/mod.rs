use anyhow::Result;
use serde_json::{json, Value};
use std::sync::Arc;
use std::future::Future;
use std::pin::Pin;

mod disk_space;
mod echo;
mod system_info;
mod http_request;
mod bilibili_download;

pub use disk_space::{DiskSpaceTool, DiskSpaceInput, DiskSpaceResult};
pub use echo::EchoTool;
pub use system_info::SystemInfoTool;
pub use http_request::{HttpRequestTool, HttpRequestInput};
pub use bilibili_download::{BilibiliDownloadTool, BilibiliDownloadInput, BilibiliDownloadResult};

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    
    fn description(&self) -> &str;
    
    fn input_schema(&self) -> Value;

    fn execute<'a>(&'a self, input: Value) -> BoxFuture<'a, Result<Value>>;
}

pub struct ToolRegistry {
    tools: Vec<Arc<dyn Tool>>,
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
        self.tools.push(Arc::from(tool));
    }
    
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
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