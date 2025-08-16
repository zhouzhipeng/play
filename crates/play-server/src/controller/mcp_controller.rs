use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use axum::{
    Router,
    extract::{Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response, Sse, sse::Event},
    routing::{get, post},
    Json,
};
use futures_util::stream::{self, Stream};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::RwLock;
use tokio::time::interval;
use tokio_stream::wrappers::IntervalStream;
use tokio_stream::StreamExt;
use tracing::{error, info, warn};
use uuid::Uuid;
use play_integration_xiaozhi::protocol;
use play_mcp::tools::ToolRegistry;

use crate::AppState;

#[derive(Debug, Clone)]
struct McpSession {
    id: String,
    created_at: chrono::DateTime<chrono::Utc>,
    last_event_id: Option<String>,
}

type SessionStore = Arc<RwLock<HashMap<String, McpSession>>>;

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "jsonrpc")]
struct JsonRpcMessage {
    #[serde(default = "default_jsonrpc_version")]
    jsonrpc: String,
    #[serde(flatten)]
    content: JsonRpcContent,
}

fn default_jsonrpc_version() -> String {
    "2.0".to_string()
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
enum JsonRpcContent {
    Request {
        id: Value,
        method: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        params: Option<Value>,
    },
    Response {
        id: Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        result: Option<Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<JsonRpcError>,
    },
    Notification {
        method: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        params: Option<Value>,
    },
}

#[derive(Debug, Deserialize, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

lazy_static::lazy_static! {
    static ref SESSIONS: SessionStore = Arc::new(RwLock::new(HashMap::new()));
    static ref TOOL_REGISTRY: Arc<ToolRegistry> = Arc::new(ToolRegistry::new());
}

pub fn init() -> Router<Arc<AppState>> {
    Router::new()
        .route("/mcp", post(handle_mcp_post).get(handle_mcp_sse))
}

async fn validate_origin(headers: &HeaderMap) -> Result<(), StatusCode> {
    if let Some(origin) = headers.get(header::ORIGIN) {
        let origin_str = origin.to_str().map_err(|_| StatusCode::BAD_REQUEST)?;
        
        if origin_str.starts_with("http://127.0.0.1") || 
           origin_str.starts_with("http://localhost") ||
           origin_str.starts_with("https://localhost") {
            return Ok(());
        }
        
        warn!("Rejected request from origin: {}", origin_str);
        return Err(StatusCode::FORBIDDEN);
    }
    
    Ok(())
}

fn get_or_create_session(headers: &HeaderMap) -> String {
    headers
        .get("mcp-session-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string())
}

fn get_protocol_version(headers: &HeaderMap) -> String {
    headers
        .get("mcp-protocol-version")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "2025-03-26".to_string())
}

async fn handle_mcp_post(
    State(_state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<JsonRpcMessage>,
) -> Result<Response, StatusCode> {
    validate_origin(&headers).await?;
    
    let session_id = get_or_create_session(&headers);
    let protocol_version = get_protocol_version(&headers);
    
    info!("MCP POST request - session: {}, protocol: {}", session_id, protocol_version);
    info!("Request payload: {:?}", payload);
    
    let mut sessions = SESSIONS.write().await;
    sessions.entry(session_id.clone()).or_insert_with(|| McpSession {
        id: session_id.clone(),
        created_at: chrono::Utc::now(),
        last_event_id: None,
    });
    
    let accept_header = headers.get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    
    match payload.content {
        JsonRpcContent::Request { id, method, params } => {
            if !accept_header.contains("text/event-stream") {
                warn!("Request missing SSE accept header");
                return Err(StatusCode::BAD_REQUEST);
            }
            
            info!("Processing request - id: {:?}, method: {}", id, method);
            
            let stream = create_response_stream(id, method, params, session_id);
            
            Ok(Sse::new(stream)
                .keep_alive(
                    axum::response::sse::KeepAlive::new()
                        .interval(Duration::from_secs(30))
                        .text("keep-alive")
                )
                .into_response())
        }
        JsonRpcContent::Response { id, result, error } => {
            info!("Received response - id: {:?}", id);
            if let Some(result) = &result {
                info!("Response result: {}", serde_json::to_string_pretty(result).unwrap_or_else(|_| result.to_string()));
            }
            if let Some(error) = &error {
                warn!("Response error: {:?}", error);
            }
            
            Ok(StatusCode::ACCEPTED.into_response())
        }
        JsonRpcContent::Notification { method, params } => {
            info!("Received notification - method: {}", method);
            if let Some(params) = &params {
                info!("Notification params: {}", serde_json::to_string_pretty(params).unwrap_or_else(|_| params.to_string()));
            }
            
            Ok(StatusCode::ACCEPTED.into_response())
        }
    }
}

fn create_response_stream(
    id: Value,
    method: String,
    params: Option<Value>,
    session_id: String,
) -> impl Stream<Item = Result<Event, axum::Error>> {
    let event_id = Uuid::new_v4().to_string();
    
    // Create the response based on the method
    let response_future = async move {
        let response = match method.as_str() {
        "ping" => {
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result":{}
            })
        }
        "initialize" => {
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "protocolVersion": "2025-03-26",
                    "capabilities": {
                        "tools": {},
                        "resources": {},
                        "prompts": {}
                    },
                    "serverInfo": {
                        "name": "play-server-mcp",
                        "version": "0.1.0"
                    }
                }
            })
        }
        "tools/list" => {
            let tools = TOOL_REGISTRY.list();
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "tools": tools
                }
            })
        }
        "tools/call" => {
            if let Some(params) = params.clone() {
                if let Some(name) = params.get("name").and_then(|v| v.as_str()) {
                    if let Some(tool) = TOOL_REGISTRY.get(name) {
                        let default_args = json!({});
                        let arguments = params.get("arguments").unwrap_or(&default_args).clone();
                        
                        match tool.execute(arguments).await {
                            Ok(result) => {
                                json!({
                                    "jsonrpc": "2.0",
                                    "id": id,
                                    "result": {
                                        "content": [
                                            {
                                                "type": "text",
                                                "text": serde_json::to_string_pretty(&result).unwrap_or_else(|_| "Error serializing result".to_string())
                                            }
                                        ]
                                    }
                                })
                            }
                            Err(e) => {
                                json!({
                                    "jsonrpc": "2.0",
                                    "id": id,
                                    "error": {
                                        "code": -32603,
                                        "message": format!("Tool execution failed: {}", e)
                                    }
                                })
                            }
                        }
                    } else {
                        json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "error": {
                                "code": -32602,
                                "message": format!("Unknown tool: {}", name)
                            }
                        })
                    }
                } else {
                    json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": {
                            "code": -32602,
                            "message": "Missing tool name in params"
                        }
                    })
                }
            } else {
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32602,
                        "message": "Invalid params for tools/call"
                    }
                })
            }
        }
        "resources/list" => {
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "resources": []
                }
            })
        }
        "prompts/list" => {
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "prompts": []
                }
            })
        }
        _ => {
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": -32601,
                    "message": "Method not found"
                }
            })
        }
        };
        
        // Log the response
        info!("Sending response - method: {}, id: {:?}, response: {}", 
              method, id, serde_json::to_string_pretty(&response).unwrap_or_else(|_| response.to_string()));
        
        // Update session with event ID
        let mut sessions = SESSIONS.write().await;
        if let Some(session) = sessions.get_mut(&session_id) {
            session.last_event_id = Some(event_id.clone());
        }
        
        Ok(Event::default()
            .id(event_id)
            .event("message")
            .data(response.to_string()))
    };
    
    // Convert the future into a stream
    stream::once(response_future)
}

async fn handle_mcp_sse(
    State(_state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_origin(&headers).await?;
    
    let session_id = get_or_create_session(&headers);
    let protocol_version = get_protocol_version(&headers);
    let last_event_id = headers
        .get("last-event-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    
    info!("MCP SSE connection - session: {}, protocol: {}, last_event: {:?}", 
          session_id, protocol_version, last_event_id);
    
    let mut sessions = SESSIONS.write().await;
    let session = sessions.entry(session_id.clone()).or_insert_with(|| McpSession {
        id: session_id.clone(),
        created_at: chrono::Utc::now(),
        last_event_id: last_event_id.clone(),
    });
    
    if let Some(event_id) = last_event_id {
        session.last_event_id = Some(event_id);
    }
    
    let stream = IntervalStream::new(interval(Duration::from_secs(30)))
        .map(move |_| -> Result<Event, axum::Error> {
            Ok(Event::default()
                .event("ping")
                .data(""))
        });
    
    Ok(Sse::new(stream)
        .keep_alive(
            axum::response::sse::KeepAlive::new()
                .interval(Duration::from_secs(30))
                .text("keep-alive")
        ))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_validate_origin() {
        let mut headers = HeaderMap::new();
        headers.insert(header::ORIGIN, "http://127.0.0.1:3000".parse().unwrap());
        assert!(validate_origin(&headers).await.is_ok());
        
        headers.insert(header::ORIGIN, "http://evil.com".parse().unwrap());
        assert!(validate_origin(&headers).await.is_err());
    }
    
    #[test]
    fn test_get_protocol_version() {
        let mut headers = HeaderMap::new();
        assert_eq!(get_protocol_version(&headers), "2025-03-26");
        
        headers.insert("mcp-protocol-version", "2025-06-18".parse().unwrap());
        assert_eq!(get_protocol_version(&headers), "2025-06-18");
    }
}