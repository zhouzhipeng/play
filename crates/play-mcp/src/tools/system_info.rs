use anyhow::Result;
use serde_json::{json, Value};
use sysinfo::System;

use super::Tool;

pub struct SystemInfoTool;

impl Tool for SystemInfoTool {
    fn name(&self) -> &str {
        "system_info"
    }
    
    fn description(&self) -> &str {
        "Get system information including OS, CPU, and memory"
    }
    
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }
    
    async fn execute(&self, _input: Value) -> Result<Value> {
        let mut sys = System::new_all();
        sys.refresh_all();
        
        Ok(json!({
            "os": System::name().unwrap_or_else(|| "Unknown".to_string()),
            "os_version": System::os_version().unwrap_or_else(|| "Unknown".to_string()),
            "kernel_version": System::kernel_version().unwrap_or_else(|| "Unknown".to_string()),
            "hostname": System::host_name().unwrap_or_else(|| "Unknown".to_string()),
            "cpu_count": sys.cpus().len(),
            "total_memory_gb": (sys.total_memory() as f64 / 1_073_741_824.0),
            "used_memory_gb": (sys.used_memory() as f64 / 1_073_741_824.0),
            "total_swap_gb": (sys.total_swap() as f64 / 1_073_741_824.0),
            "used_swap_gb": (sys.used_swap() as f64 / 1_073_741_824.0),
        }))
    }
}