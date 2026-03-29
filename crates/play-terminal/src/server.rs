use axum::{
    extract::{State, WebSocketUpgrade, Path, Query},
    response::{Html, IntoResponse, Response, Json},
    routing::{delete, get, post},
    Router,
    body::Body,
    http::{header, StatusCode},
};
use serde_json::json;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing::debug;

use crate::{session_manager::SessionManager, websocket::websocket_handler};

// Embed all static resources
const INDEX_HTML: &str = include_str!("../static/index.html");
const TERMINAL_JS: &str = include_str!("../static/terminal.js");
const XTERM_JS: &str = include_str!("../static/xterm.js");
const XTERM_CSS: &str = include_str!("../static/xterm.min.css");
const ADDON_FIT_JS: &str = include_str!("../static/addon-fit.js");

pub fn create_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let session_manager = Arc::new(SessionManager::new());

    Router::new()
        .route("/web-terminal", get(terminal_page))
        .route("/web-terminal/ws", get(ws_handler))
        .route("/web-terminal/assets/{*path}", get(serve_asset))
        .route("/web-terminal/api/sessions", get(list_sessions).post(create_session))
        .route("/web-terminal/api/sessions/{name}", delete(delete_session))
        .route("/web-terminal/api/sessions/{name}/cwd", get(get_session_cwd))
        .route("/web-terminal/{session_name}", get(terminal_page_with_session))
        .layer(cors)
        .with_state(session_manager)
}

async fn terminal_page() -> impl IntoResponse {
    Html(INDEX_HTML.to_string())
}

async fn terminal_page_with_session(Path(session_name): Path<String>) -> impl IntoResponse {
    // Inject the session name into the HTML
    let html = INDEX_HTML.replace(
        "</body>",
        &format!(
            r#"<script>
                window.targetSessionName = "{}";
            </script>
            </body>"#,
            session_name
        ),
    );
    Html(html)
}

async fn ws_handler(
    State(session_manager): State<Arc<SessionManager>>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    debug!("WebSocket upgrade handler called!");
    websocket_handler(ws, session_manager).await
}

async fn list_sessions(
    State(session_manager): State<Arc<SessionManager>>,
) -> impl IntoResponse {
    let sessions = session_manager.list_sessions();
    Json(json!({ "sessions": sessions }))
}

async fn create_session(
    State(session_manager): State<Arc<SessionManager>>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    let name = payload.get("name").and_then(|v| v.as_str()).map(|s| s.to_string());
    
    match session_manager.create_session(name) {
        Ok(session) => {
            (StatusCode::CREATED, Json(json!({ "session": session })))
        }
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() })))
        }
    }
}

async fn delete_session(
    State(session_manager): State<Arc<SessionManager>>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> impl IntoResponse {
    match session_manager.delete_session(&name) {
        Ok(_) => {
            (StatusCode::OK, Json(json!({ "message": "Session deleted" })))
        }
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() })))
        }
    }
}

async fn serve_asset(axum::extract::Path(path): axum::extract::Path<String>) -> impl IntoResponse {
    let path = path.trim_start_matches('/');
    
    let (content, content_type) = match path {
        "terminal.js" => (TERMINAL_JS, "application/javascript"),
        "xterm.js" => (XTERM_JS, "application/javascript"),
        "xterm.min.css" => (XTERM_CSS, "text/css"),
        "addon-fit.js" => (ADDON_FIT_JS, "application/javascript"),
        _ => {
            return Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("Not Found"))
                .unwrap();
        }
    };
    
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CACHE_CONTROL, "public, max-age=31536000") // Cache for 1 year
        .body(Body::from(content))
        .unwrap()
}

async fn get_session_cwd(
    State(session_manager): State<Arc<SessionManager>>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    match session_manager.get_session_cwd(&name) {
        Ok(cwd) => (StatusCode::OK, Json(json!({ "cwd": cwd }))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}
