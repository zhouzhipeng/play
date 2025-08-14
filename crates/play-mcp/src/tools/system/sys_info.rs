use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use sysinfo::System;

use crate::tools::{Tool, ToolMetadata};
use crate::register_mcp_tool;

pub struct SysInfoTool {
    metadata: ToolMetadata,
}

register_mcp_tool!(SysInfoTool, "sys_info");

#[async_trait]
impl Tool for SysInfoTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }
    
    async fn execute(&self, _input: Value) -> Result<Value> {
        let mut sys = System::new_all();
        sys.refresh_all();
        
        Ok(json!({
            "os": System::name().unwrap_or_else(|| "Unknown".to_string()),
            "os_version": System::os_version().unwrap_or_else(|| "Unknown".to_string()),
            "kernel_version": System::kernel_version().unwrap_or_else(|| "Unknown".to_string()),
            "hostname": System::host_name().unwrap_or_else(|| "Unknown".to_string()),
            "uptime": System::uptime(),
            "boot_time": System::boot_time(),
        }))
    }
}