use anyhow::Result;
use serde_json::{json, Value};
use sysinfo::Disks;

crate::define_mcp_tool!(
    SysDiskTool,
    "sys_disk",
    |input: Value| async move {
        let disks = Disks::new_with_refreshed_list();
        let path = input.get("path").and_then(|v| v.as_str());
        
        let mut disk_info = Vec::new();
        
        for disk in disks.list() {
            if let Some(p) = path {
                if !disk.mount_point().to_string_lossy().contains(p) {
                    continue;
                }
            }
            
            let total = disk.total_space();
            let available = disk.available_space();
            let used = total - available;
            
            disk_info.push(json!({
                "mount_point": disk.mount_point().to_string_lossy(),
                "total_gb": (total as f64 / 1_073_741_824.0),
                "available_gb": (available as f64 / 1_073_741_824.0),
                "used_gb": (used as f64 / 1_073_741_824.0),
                "used_percentage": ((used as f64 / total as f64) * 100.0),
                "file_system": disk.file_system().to_string_lossy(),
                "is_removable": disk.is_removable(),
            }));
        }
        
        Ok(json!({
            "disks": disk_info
        }))
    }
);