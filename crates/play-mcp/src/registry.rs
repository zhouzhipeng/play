use crate::tools::*;

/// Register all default tools to the registry
pub fn register_default_tools(registry: &mut ToolRegistry) {
    registry.register(Box::new(DiskSpaceTool));
    registry.register(Box::new(EchoTool));
    registry.register(Box::new(SystemInfoTool));
    registry.register(Box::new(HttpRequestTool));
}