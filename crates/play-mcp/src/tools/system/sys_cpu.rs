use anyhow::Result;
use serde_json::{json, Value};
use sysinfo::System;

crate::define_mcp_tool!(
    SysCpuTool,
    "sys_cpu",
    |input: Value| async move {
        let per_core = input.get("per_core")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        
        let mut sys = System::new_all();
        sys.refresh_cpu_all();
        
        // Wait a bit to get accurate CPU usage
        std::thread::sleep(std::time::Duration::from_millis(200));
        sys.refresh_cpu_all();
        
        let global_cpu = sys.global_cpu_usage();
        
        let mut result = json!({
            "global_cpu_usage": global_cpu,
            "cpu_count": sys.cpus().len(),
            "physical_core_count": sys.physical_core_count(),
        });
        
        if per_core {
            let cores: Vec<_> = sys.cpus()
                .iter()
                .enumerate()
                .map(|(i, cpu)| {
                    json!({
                        "core": i,
                        "name": cpu.name(),
                        "usage": cpu.cpu_usage(),
                        "frequency": cpu.frequency(),
                    })
                })
                .collect();
            result["cores"] = json!(cores);
        }
        
        Ok(result)
    }
);