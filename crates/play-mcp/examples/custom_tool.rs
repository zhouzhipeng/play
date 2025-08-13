use anyhow::Result;
use play_mcp::{Tool, ToolRegistry, McpConfig, start_mcp_client_with_tools, AnyTool};
use serde_json::{json, Value};

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
    
    async fn execute(&self, input: Value) -> Result<Value> {
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
    }
}

// To use custom tools with the registry, you would need to extend the AnyTool enum
// For demonstration, we'll just show how the built-in tools work
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging (optional - requires tracing-subscriber feature)
    // tracing_subscriber::fmt::init();
    
    // Create tool registry with built-in tools
    let mut registry = ToolRegistry::new();
    
    // Register built-in tools
    registry.register(AnyTool::DiskSpace(play_mcp::DiskSpaceTool));
    registry.register(AnyTool::SystemInfo(play_mcp::SystemInfoTool));
    registry.register(AnyTool::Echo(play_mcp::EchoTool));
    
    // Note: To add custom tools like CalculatorTool, you would need to:
    // 1. Extend the AnyTool enum in tools.rs with a Calculator variant
    // 2. Add the corresponding match arms in the AnyTool impl
    // 3. Then you could do: registry.register(AnyTool::Calculator(CalculatorTool));
    
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
    };
    
    // Start MCP client with custom tools
    start_mcp_client_with_tools(&config, registry).await?;
    
    Ok(())
}