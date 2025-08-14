use anyhow::{Result, Context};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{error, info, warn};

pub mod config;
pub mod tools;
mod registry;
mod metadata_loader;

pub use config::{McpConfig, ClientConfig, RetryConfig};
pub use tools::{
    Tool, ToolRegistry, ToolMetadata,
    DiskSpaceTool, DiskSpaceInput, DiskSpaceResult,
    EchoTool, SystemInfoTool,
    HttpRequestTool, HttpRequestInput,
    BilibiliDownloadTool, BilibiliDownloadInput, BilibiliDownloadResult,
    SysInfoTool, SysDiskTool, SysMemoryTool, SysProcessTool, SysCpuTool
};
pub use registry::register_default_tools;

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
    pub id: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub result: Option<Value>,
    pub error: Option<JsonRpcError>,
    pub id: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    pub data: Option<Value>,
}

async fn handle_server_request(request: JsonRpcRequest, registry: &ToolRegistry) -> Option<JsonRpcResponse> {
    match request.method.as_str() {
        "ping" => {
            Some(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: Some(json!({})),
                error: None,
                id: request.id,
            })
        }
        method if method.starts_with("notifications/") => {
            // Notifications don't require a response
            None
        }
        "tools/list" => {
            Some(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: Some(json!({
                    "tools": registry.list()
                })),
                error: None,
                id: request.id,
            })
        }
        "tools/call" => {
            if let Some(params) = request.params.clone() {
                if let Some(name) = params.get("name").and_then(|v| v.as_str()) {
                    if let Some(tool) = registry.get(name) {
                        let default_args = json!({});
                        let arguments = params.get("arguments").unwrap_or(&default_args);
                        
                        let result = tool.execute(arguments.clone()).await;
                        
                        match result {
                            Ok(result) => {
                                return Some(JsonRpcResponse {
                                    jsonrpc: "2.0".to_string(),
                                    result: Some(json!({
                                        "content": [
                                            {
                                                "type": "text",
                                                "text": serde_json::to_string_pretty(&result).unwrap()
                                            }
                                        ]
                                    })),
                                    error: None,
                                    id: request.id,
                                });
                            }
                            Err(e) => {
                                return Some(JsonRpcResponse {
                                    jsonrpc: "2.0".to_string(),
                                    result: None,
                                    error: Some(JsonRpcError {
                                        code: -32603,
                                        message: format!("Tool execution failed: {}", e),
                                        data: None,
                                    }),
                                    id: request.id,
                                });
                            }
                        }
                    }
                }
            }
            
            Some(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(JsonRpcError {
                    code: -32602,
                    message: "Invalid params or unknown tool".to_string(),
                    data: None,
                }),
                id: request.id,
            })
        }
        _ => {
            warn!("Unexpected method from server: {}", request.method);
            Some(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(JsonRpcError {
                    code: -32601,
                    message: "Method not found".to_string(),
                    data: None,
                }),
                id: request.id,
            })
        }
    }
}

async fn run_mcp_connection(url: String, client_config: &ClientConfig, registry: Arc<ToolRegistry>) -> Result<()> {
    info!("Connecting to MCP server at: {}", url);
    
    let (ws_stream, _) = connect_async(&url).await
        .context("Failed to connect to MCP server")?;
    info!("Connected to MCP server");
    
    let (mut write, mut read) = ws_stream.split();
    
    // Wait for server's initialize request
    if let Some(Ok(Message::Text(msg))) = read.next().await {
        info!("<<<< Received from server:\n{}", msg);
        
        // Parse and check if it's an initialize request
        if let Ok(request) = serde_json::from_str::<JsonRpcRequest>(&msg) {
            if request.method == "initialize" {
                // Send initialize response
                let init_response = JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: Some(json!({
                        "protocolVersion": "2024-11-05",
                        "capabilities": {
                            "tools": {}
                        },
                        "serverInfo": {
                            "name": client_config.name,
                            "version": client_config.version
                        }
                    })),
                    error: None,
                    id: request.id,
                };
                
                let response_msg = serde_json::to_string(&init_response)
                    .context("Failed to serialize initialize response")?;
                info!(">>>> Sending to server:\n{}", response_msg);
                write.send(Message::Text(response_msg.clone())).await
                    .context("Failed to send initialize response")?;
            }
        }
    }
    
    // Wait for tools/list request from server
    info!("Waiting for tools/list request from server...");
    
    // Handle incoming requests from server
    loop {
        tokio::select! {
            Some(msg) = read.next() => {
                match msg {
                    Ok(Message::Text(text)) => {
                        info!("<<<< Received from server:\n{}", text);
                        
                        match serde_json::from_str::<JsonRpcRequest>(&text) {
                            Ok(request) => {
                                // Handle the request from server
                                if let Some(response) = handle_server_request(request, &registry).await {
                                    let response_text = serde_json::to_string(&response)
                                        .context("Failed to serialize response")?;
                                    info!(">>>> Sending to server:\n{}", response_text);
                                    write.send(Message::Text(response_text)).await
                                        .context("Failed to send response")?;
                                }
                            }
                            Err(e) => {
                                error!("Failed to parse server request: {}", e);
                            }
                        }
                    }
                    Ok(Message::Close(_)) => {
                        info!("Server closed connection");
                        break;
                    }
                    Ok(Message::Ping(data)) => {
                        write.send(Message::Pong(data)).await
                            .context("Failed to send pong")?;
                    }
                    Err(e) => {
                        error!("WebSocket error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }
            _ = tokio::signal::ctrl_c() => {
                info!("Received interrupt signal, shutting down...");
                break;
            }
        }
    }
    
    // Send close message
    write.send(Message::Close(None)).await
        .context("Failed to send close message")?;
    
    Ok(())
}

/// Start MCP client service with default tools
pub async fn start_mcp_client(config: &McpConfig) -> Result<()> {
    let mut registry = if config.tool_name_prefix.is_empty() {
        ToolRegistry::new()
    } else {
        ToolRegistry::with_prefix(config.tool_name_prefix.clone())
    };
    registry::register_default_tools(&mut registry);
    
    start_mcp_client_with_tools(config, registry).await
}

/// Start MCP client service with custom tool registry
pub async fn start_mcp_client_with_tools(config: &McpConfig, registry: ToolRegistry) -> Result<()> {
    info!("Starting MCP client: {}", config.client.name);
    info!("Description: {}", config.client.description);
    info!("Connecting to: {}", config.url);
    
    if !config.tool_name_prefix.is_empty() {
        info!("Tool name prefix: '{}'", config.tool_name_prefix);
    }
    
    let registry = Arc::new(registry);
    
    let mut attempts = 0u32;
    loop {
        match run_mcp_connection(config.url.clone(), &config.client, registry.clone()).await {
            Ok(_) => {
                info!("MCP client disconnected normally");
                break;
            }
            Err(e) => {
                error!("MCP client error: {}", e);
                
                if !config.retry.enabled {
                    break;
                }
                
                attempts += 1;
                if config.retry.max_attempts > 0 && attempts >= config.retry.max_attempts {
                    error!("Max retry attempts ({}) reached", config.retry.max_attempts);
                    break;
                }
                
                info!("Retrying connection in {} seconds... (attempt {}/{})", 
                    config.retry.interval_seconds,
                    attempts,
                    if config.retry.max_attempts > 0 { 
                        config.retry.max_attempts.to_string() 
                    } else { 
                        "unlimited".to_string() 
                    }
                );
                tokio::time::sleep(tokio::time::Duration::from_secs(config.retry.interval_seconds)).await;
            }
        }
    }
    
    Ok(())
}