use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use sysinfo::System;

use crate::tools::{Tool, ToolMetadata};
use crate::impl_tool_with_metadata;

pub struct SysMemoryTool {
    metadata: ToolMetadata,
}

impl_tool_with_metadata!(SysMemoryTool, "sys_memory");

#[async_trait]
impl Tool for SysMemoryTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }
    
    async fn execute(&self, input: Value) -> Result<Value> {
        let mut sys = System::new_all();
        sys.refresh_memory();
        
        let detailed = input.get("detailed")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        
        let mut result = json!({
            "total_memory_gb": (sys.total_memory() as f64 / 1_073_741_824.0),
            "used_memory_gb": (sys.used_memory() as f64 / 1_073_741_824.0),
            "available_memory_gb": (sys.available_memory() as f64 / 1_073_741_824.0),
            "free_memory_gb": (sys.free_memory() as f64 / 1_073_741_824.0),
            "total_swap_gb": (sys.total_swap() as f64 / 1_073_741_824.0),
            "used_swap_gb": (sys.used_swap() as f64 / 1_073_741_824.0),
            "free_swap_gb": (sys.free_swap() as f64 / 1_073_741_824.0),
        });
        
        if detailed {
            result["memory_percentage"] = json!((sys.used_memory() as f64 / sys.total_memory() as f64) * 100.0);
            result["swap_percentage"] = json!((sys.used_swap() as f64 / sys.total_swap() as f64) * 100.0);
        }
        
        Ok(result)
    }
}