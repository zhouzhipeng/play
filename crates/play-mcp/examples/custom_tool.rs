use anyhow::Result;
use async_trait::async_trait;
use play_mcp::{Tool, ToolRegistry, ToolMetadata, McpConfig, start_mcp_client_with_tools};
use serde_json::{json, Value};

struct CalculatorTool {
    metadata: ToolMetadata,
}

impl CalculatorTool {
    pub fn new() -> Self {
        Self {
            metadata: ToolMetadata::new(
                "calculator",
                "Perform basic mathematical operations",
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
            ),
        }
    }
}

#[async_trait]
impl Tool for CalculatorTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
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

// Now you can directly use Box<dyn Tool> to register any tool
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging (optional - requires tracing-subscriber feature)
    // tracing_subscriber::fmt::init();
    
    // Create tool registry - all built-in tools are auto-registered
    let mut registry = ToolRegistry::new();
    
    // You can still register custom tools that are not using impl_tool_with_metadata!
    registry.register(Box::new(CalculatorTool::new()));
    
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