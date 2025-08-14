use crate::tools::*;

/// Register all default tools to the registry
pub fn register_default_tools(registry: &mut ToolRegistry) {
    // Basic tools
    registry.register(Box::new(DiskSpaceTool::new()));
    registry.register(Box::new(EchoTool::new()));
    registry.register(Box::new(SystemInfoTool::new()));
    registry.register(Box::new(HttpRequestTool::new()));
    registry.register(Box::new(BilibiliDownloadTool::new()));
    
    // System tools
    registry.register(Box::new(SysInfoTool::new()));
    registry.register(Box::new(SysDiskTool::new()));
    registry.register(Box::new(SysMemoryTool::new()));
    registry.register(Box::new(SysProcessTool::new()));
    registry.register(Box::new(SysCpuTool::new()));
}