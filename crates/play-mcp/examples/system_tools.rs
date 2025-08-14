use anyhow::Result;
use play_mcp::tools::{Tool, SysInfoTool, SysMemoryTool, SysCpuTool, SysDiskTool, SysProcessTool};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<()> {
    println!("System Tools Example\n");
    
    // Example 1: System Info
    println!("Example 1: System Info");
    let sys_info = SysInfoTool::new();
    let result = sys_info.execute(json!({})).await?;
    println!("System Info: {}\n", serde_json::to_string_pretty(&result)?);
    
    // Example 2: Memory Info
    println!("Example 2: Memory Info (detailed)");
    let sys_memory = SysMemoryTool::new();
    let result = sys_memory.execute(json!({
        "detailed": true
    })).await?;
    println!("Memory: {}\n", serde_json::to_string_pretty(&result)?);
    
    // Example 3: CPU Info
    println!("Example 3: CPU Info");
    let sys_cpu = SysCpuTool::new();
    let result = sys_cpu.execute(json!({
        "per_core": false
    })).await?;
    println!("CPU: {}\n", serde_json::to_string_pretty(&result)?);
    
    // Example 4: Disk Info
    println!("Example 4: Disk Info");
    let sys_disk = SysDiskTool::new();
    let result = sys_disk.execute(json!({})).await?;
    if let Some(disks) = result.get("disks").and_then(|d| d.as_array()) {
        if let Some(first_disk) = disks.first() {
            println!("First disk: {}\n", serde_json::to_string_pretty(&first_disk)?);
        }
    }
    
    // Example 5: Process Info
    println!("Example 5: Top 5 Processes by CPU");
    let sys_process = SysProcessTool::new();
    let result = sys_process.execute(json!({
        "sort_by": "cpu",
        "limit": 5
    })).await?;
    if let Some(processes) = result.get("processes").and_then(|p| p.as_array()) {
        for process in processes {
            if let (Some(name), Some(cpu)) = (
                process.get("name").and_then(|n| n.as_str()),
                process.get("cpu_usage").and_then(|c| c.as_f64())
            ) {
                println!("  - {}: {:.2}% CPU", name, cpu);
            }
        }
    }
    
    Ok(())
}