use anyhow::Result;
use serde_json::{json, Value};
use sysinfo::System;

crate::define_mcp_tool!(
    SysInfoTool,
    "sys_info",
    |_input: Value| async move {
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
);