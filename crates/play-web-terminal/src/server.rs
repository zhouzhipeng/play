use axum::{
    extract::WebSocketUpgrade,
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
    body::Body,
    http::{header, StatusCode},
};
use tower_http::cors::{Any, CorsLayer};

use crate::websocket::websocket_handler;

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

    Router::new()
        .route("/web-terminal", get(terminal_page))
        .route("/web-terminal/ws", get(ws_handler))
        .route("/web-terminal/assets/{*path}", get(serve_asset))
        .layer(cors)
}

async fn terminal_page() -> impl IntoResponse {
    Html(INDEX_HTML.to_string())
}

async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    websocket_handler(ws).await
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