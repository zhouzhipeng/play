use crate::tools::*;

/// Register all default tools to the registry
pub fn register_default_tools(registry: &mut ToolRegistry) {
    registry.register(AnyTool::DiskSpace(DiskSpaceTool));
    registry.register(AnyTool::Echo(EchoTool));
    registry.register(AnyTool::SystemInfo(SystemInfoTool));
    registry.register(AnyTool::HttpRequest(HttpRequestTool));
}