use anyhow::Result;
use play_mcp::{Tool, ToolRegistry, McpConfig, start_mcp_client_with_tools};
use serde_json::{json, Value};
use std::future::Future;
use std::pin::Pin;

struct CalculatorTool;

impl Tool for CalculatorTool {
    fn name(&self) -> &str {
        "calculator"
    }
    
    fn description(&self) -> &str {
        "Perform basic mathematical operations"
    }
    
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["add", "subtract", "multiply", "divide"],
                    "description": "The operation to perform"
                },
                "a": {
                    "type": "number",
                    "description": "First operand"
                },
                "b": {
                    "type": "number",
                    "description": "Second operand"
                }
            },
            "required": ["operation", "a", "b"]
        })
    }
    
    fn execute(&self, input: Value) -> Pin<Box<dyn Future<Output = Result<Value>> + Send + '_>> {
        Box::pin(async move {
            let operation = input.get("operation")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Missing operation"))?;
            
            let a = input.get("a")
                .and_then(|v| v.as_f64())
                .ok_or_else(|| anyhow::anyhow!("Missing or invalid 'a' value"))?;
            
            let b = input.get("b")
                .and_then(|v| v.as_f64())
                .ok_or_else(|| anyhow::anyhow!("Missing or invalid 'b' value"))?;
            
            let result = match operation {
                "add" => a + b,
                "subtract" => a - b,
                "multiply" => a * b,
                "divide" => {
                    if b == 0.0 {
                        return Err(anyhow::anyhow!("Division by zero"));
                    }
                    a / b
                }
                _ => return Err(anyhow::anyhow!("Unknown operation: {}", operation)),
            };
            
            Ok(json!({
                "result": result,
                "operation": operation,
                "a": a,
                "b": b
            }))
        })
    }
}

// Now you can directly use Box<dyn Tool> to register any tool
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging (optional - requires tracing-subscriber feature)
    // tracing_subscriber::fmt::init();
    
    // Create tool registry with built-in tools
    let mut registry = ToolRegistry::new();
    
    // Register built-in tools using Box<dyn Tool>
    registry.register(Box::new(play_mcp::DiskSpaceTool));
    registry.register(Box::new(play_mcp::SystemInfoTool));
    registry.register(Box::new(play_mcp::EchoTool));
    
    // You can also register custom tools directly!
    // registry.register(Box::new(CalculatorTool));
    
    // Create MCP config
    let config = McpConfig {
        url: "ws://localhost:3000/mcp".to_string(),
        client: play_mcp::ClientConfig {
            name: "custom-mcp-client".to_string(),
            version: "1.0.0".to_string(),
            description: "MCP client with custom tools".to_string(),
        },
        retry: play_mcp::RetryConfig {
            enabled: true,
            max_attempts: 5,
            interval_seconds: 5,
        },
        tool_name_prefix: String::new(), // No prefix for this example
    };
    
    // Start MCP client with custom tools
    start_mcp_client_with_tools(&config, registry).await?;
    
    Ok(())
}