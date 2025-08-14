use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sysinfo::Disks;

#[derive(Debug, Serialize, Deserialize)]
pub struct DiskSpaceInput {
    pub path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DiskSpaceResult {
    pub path: String,
    pub total_gb: f64,
    pub available_gb: f64,
    pub used_gb: f64,
    pub used_percentage: f64,
}

crate::define_mcp_tool!(
    DiskSpaceTool,
    "get_disk_space",
    |input: Value| async move {
        let input: DiskSpaceInput = serde_json::from_value(input)?;
        let results = get_disk_space(input);
        Ok(serde_json::to_value(results)?)
    }
);

pub fn get_disk_space(input: DiskSpaceInput) -> Vec<DiskSpaceResult> {
    let disks = Disks::new_with_refreshed_list();
    let mut results = Vec::new();
    
    if let Some(path) = input.path {
        for disk in disks.list() {
            if disk.mount_point().to_string_lossy().contains(&path) {
                let total_gb = disk.total_space() as f64 / 1_073_741_824.0;
                let available_gb = disk.available_space() as f64 / 1_073_741_824.0;
                let used_gb = total_gb - available_gb;
                let used_percentage = (used_gb / total_gb) * 100.0;
                
                results.push(DiskSpaceResult {
                    path: disk.mount_point().to_string_lossy().to_string(),
                    total_gb: (total_gb * 100.0).round() / 100.0,
                    available_gb: (available_gb * 100.0).round() / 100.0,
                    used_gb: (used_gb * 100.0).round() / 100.0,
                    used_percentage: (used_percentage * 100.0).round() / 100.0,
                });
                break;
            }
        }
    } else {
        for disk in disks.list() {
            let total_gb = disk.total_space() as f64 / 1_073_741_824.0;
            let available_gb = disk.available_space() as f64 / 1_073_741_824.0;
            let used_gb = total_gb - available_gb;
            let used_percentage = (used_gb / total_gb) * 100.0;
            
            results.push(DiskSpaceResult {
                path: disk.mount_point().to_string_lossy().to_string(),
                total_gb: (total_gb * 100.0).round() / 100.0,
                available_gb: (available_gb * 100.0).round() / 100.0,
                used_gb: (used_gb * 100.0).round() / 100.0,
                used_percentage: (used_percentage * 100.0).round() / 100.0,
            });
        }
    }
    
    results
}