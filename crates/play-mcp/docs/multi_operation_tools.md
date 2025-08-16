# Multi-Operation Tools Guide

## Overview

Multi-operation tools allow a single tool to provide multiple related operations under a common prefix. This is useful for grouping related functionality together, such as all system-related operations under a `sys:` prefix.

## Key Components

### 1. ToolOperation Struct

Represents a single operation within a multi-operation tool:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOperation {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}
```

### 2. Extended Tool Trait

The `Tool` trait now includes methods for multi-operation support:

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    fn metadata(&self) -> &ToolMetadata;
    async fn execute(&self, input: Value) -> Result<Value>;
    
    // New methods for multi-operation support
    fn operations(&self) -> Option<Vec<ToolOperation>> {
        None  // Default: no operations (single-operation tool)
    }
    
    async fn execute_operation(&self, operation: &str, input: Value) -> Result<Value> {
        self.execute(input).await  // Default: call main execute
    }
}
```

## Creating a Multi-Operation Tool

Here's an example of creating a multi-operation tool:

```rust
pub struct SystemTool {
    metadata: ToolMetadata,
    operations: Vec<ToolOperation>,
}

impl SystemTool {
    pub fn new() -> Self {
        Self {
            metadata: ToolMetadata::new(
                "sys",  // Tool prefix
                "System information and monitoring tool",
                json!({
                    "type": "object",
                    "properties": {
                        "operation": {
                            "type": "string",
                            "enum": ["info", "disk", "memory", "process", "cpu"]
                        },
                        "args": {
                            "type": "object",
                            "description": "Operation-specific arguments"
                        }
                    },
                    "required": ["operation"]
                })
            ),
            operations: vec![
                ToolOperation::new(
                    "info",
                    "Get general system information",
                    json!({ /* operation-specific schema */ })
                ),
                ToolOperation::new(
                    "disk",
                    "Get disk usage information",
                    json!({ /* operation-specific schema */ })
                ),
                // ... more operations
            ]
        }
    }
}

#[async_trait]
impl Tool for SystemTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }
    
    fn operations(&self) -> Option<Vec<ToolOperation>> {
        Some(self.operations.clone())
    }
    
    async fn execute(&self, input: Value) -> Result<Value> {
        let operation = input.get("operation")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'operation' field"))?;
        
        let args = input.get("args").unwrap_or(&json!({}));
        self.execute_operation(operation, args.clone()).await
    }
    
    async fn execute_operation(&self, operation: &str, input: Value) -> Result<Value> {
        match operation {
            "info" => self.get_system_info().await,
            "disk" => self.get_disk_info(input).await,
            "memory" => self.get_memory_info(input).await,
            // ... handle other operations
            _ => Err(anyhow::anyhow!("Unknown operation: {}", operation))
        }
    }
}
```

## Using Multi-Operation Tools

### 1. Direct Usage

```rust
let sys_tool = SystemTool::new();

// Using execute with operation parameter
let result = sys_tool.execute(json!({
    "operation": "memory",
    "args": {
        "detailed": true
    }
})).await?;

// Using execute_operation directly
let result = sys_tool.execute_operation("disk", json!({
    "path": "/home"
})).await?;
```

### 2. Through Registry

The registry automatically handles multi-operation tools:

```rust
let mut registry = ToolRegistry::new();
registry.register(Box::new(SystemTool::new()));

// The registry lists each operation as a separate tool
let tools = registry.list();
// Returns: ["sys:info", "sys:disk", "sys:memory", ...]

// Get tool and operation
if let Some((tool, operation)) = registry.get_with_operation("sys:memory") {
    let result = if let Some(op) = operation {
        tool.execute_operation(&op, input).await?
    } else {
        tool.execute(input).await?
    };
}
```

### 3. MCP Protocol

When exposed through MCP, each operation appears as a separate tool:

```json
{
  "tools": [
    {
      "name": "sys:info",
      "description": "Get general system information",
      "inputSchema": { /* ... */ }
    },
    {
      "name": "sys:disk",
      "description": "Get disk usage information",
      "inputSchema": { /* ... */ }
    }
    // ... more operations
  ]
}
```

Clients can call these operations using the full name:

```json
{
  "method": "tools/call",
  "params": {
    "name": "sys:memory",
    "arguments": {
      "detailed": true
    }
  }
}
```

## Available System Operations

The `SystemTool` provides the following operations:

| Operation | Description | Key Parameters |
|-----------|-------------|----------------|
| `sys:info` | Get general system information | None |
| `sys:disk` | Get disk usage information | `path` (optional) |
| `sys:memory` | Get memory usage information | `detailed` (boolean) |
| `sys:process` | Get process information | `filter`, `sort_by`, `limit` |
| `sys:cpu` | Get CPU usage information | `per_core` (boolean) |

## Benefits

1. **Organization**: Related operations are grouped under a common prefix
2. **Discoverability**: Each operation appears as a separate tool in listings
3. **Flexibility**: Tools can have different input schemas for each operation
4. **Backwards Compatibility**: Single-operation tools continue to work unchanged
5. **Namespace Management**: Prefixes prevent naming conflicts

## Best Practices

1. **Use Clear Prefixes**: Choose short, descriptive prefixes (e.g., `sys:`, `db:`, `file:`)
2. **Document Operations**: Provide clear descriptions for each operation
3. **Consistent Schemas**: Use consistent parameter names across operations
4. **Error Handling**: Provide helpful error messages for unknown operations
5. **Operation Discovery**: Implement the `operations()` method to enable discovery