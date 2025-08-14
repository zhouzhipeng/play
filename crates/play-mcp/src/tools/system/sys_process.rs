use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use sysinfo::{System, ProcessRefreshKind, RefreshKind};

use crate::tools::{Tool, ToolMetadata};
use crate::register_mcp_tool;

pub struct SysProcessTool {
    metadata: ToolMetadata,
}

register_mcp_tool!(SysProcessTool, "sys_process");

#[async_trait]
impl Tool for SysProcessTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }
    
    async fn execute(&self, input: Value) -> Result<Value> {
        let filter = input.get("filter").and_then(|v| v.as_str());
        let sort_by = input.get("sort_by")
            .and_then(|v| v.as_str())
            .unwrap_or("cpu");
        let limit = input.get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(10) as usize;
        
        let mut sys = System::new_with_specifics(
            RefreshKind::new().with_processes(ProcessRefreshKind::everything())
        );
        
        let mut processes: Vec<_> = sys.processes()
            .iter()
            .filter(|(_, process)| {
                if let Some(f) = filter {
                    process.name().to_string_lossy().contains(f)
                } else {
                    true
                }
            })
            .map(|(pid, process)| {
                json!({
                    "pid": pid.as_u32(),
                    "name": process.name(),
                    "cpu_usage": process.cpu_usage(),
                    "memory_mb": (process.memory() as f64 / 1_048_576.0),
                    "virtual_memory_mb": (process.virtual_memory() as f64 / 1_048_576.0),
                    "status": format!("{:?}", process.status()),
                })
            })
            .collect();
        
        // Sort processes
        processes.sort_by(|a, b| {
            match sort_by {
                "memory" => {
                    let a_mem = a["memory_mb"].as_f64().unwrap_or(0.0);
                    let b_mem = b["memory_mb"].as_f64().unwrap_or(0.0);
                    b_mem.partial_cmp(&a_mem).unwrap()
                }
                "name" => {
                    let a_name = a["name"].as_str().unwrap_or("");
                    let b_name = b["name"].as_str().unwrap_or("");
                    a_name.cmp(b_name)
                }
                _ => { // cpu
                    let a_cpu = a["cpu_usage"].as_f64().unwrap_or(0.0);
                    let b_cpu = b["cpu_usage"].as_f64().unwrap_or(0.0);
                    b_cpu.partial_cmp(&a_cpu).unwrap()
                }
            }
        });
        
        processes.truncate(limit);
        
        Ok(json!({
            "processes": processes,
            "total_count": sys.processes().len()
        }))
    }
}