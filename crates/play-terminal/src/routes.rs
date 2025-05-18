//! HTTP and WebSocket routes for the web terminal.

use std::sync::Arc;
use std::time::Duration;

use axum::extract::{ws::{Message, WebSocket, WebSocketUpgrade}, Path, Query, State};
use axum::response::{Html, IntoResponse};
use axum::routing::{get, get_service};
use axum::{Json, Router};
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::sync::mpsc;
use tower_http::services::ServeDir;

use crate::config::TerminalConfig;
use crate::terminal::{ClientMessage, TerminalMessage, TerminalSession};

/// The state for the terminal routes.
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<TerminalConfig>,
}

/// Parameters for creating a terminal.
#[derive(Deserialize)]
struct TerminalParams {
    /// The shell to use.
    #[serde(default = "default_shell")]
    shell: String,
    /// The number of columns.
    #[serde(default = "default_cols")]
    cols: u16,
    /// The number of rows.
    #[serde(default = "default_rows")]
    rows: u16,
}

fn default_shell() -> String {
    if cfg!(target_os = "windows") {
        "cmd.exe".into()
    } else {
        "/bin/bash".into()
    }
}

fn default_cols() -> u16 {
    80
}

fn default_rows() -> u16 {
    24
}

/// Creates the routes for the web terminal.
pub fn create_routes(config: Arc<TerminalConfig>) -> Router {
    let config_for_ws = config.clone();
    let mut router = Router::new()
        .route("/ws", get(move |ws, query| websocket_handler(ws, query, config_for_ws)));

    // Serve the static assets
    if let Some(assets_dir) = &config.assets_dir {
        router = router.nest_service(
            "/",
            get_service(ServeDir::new(assets_dir.as_ref())).handle_error(
                |error| async move {
                    (
                        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Error serving static asset: {}", error),
                    )
                },
            ),
        );
    } else if config.use_embedded_assets {
        let config_clone = config.clone();
        router = router
            .route("/", get(move || index_handler(config_clone.clone())))
            .route("/css/style.css", get(css_handler))
            .route("/js/terminal.js", get(js_handler));
    }

    router
}

/// Handler for the WebSocket connection.
async fn websocket_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<TerminalParams>,
    config: Arc<TerminalConfig>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, params, config))
}

/// Handles the WebSocket connection.
async fn handle_socket(socket: WebSocket, params: TerminalParams, config: Arc<TerminalConfig>) {
    // Split the socket into sender and receiver
    let (mut sender, mut receiver) = socket.split();

    // Create a channel for terminal messages
    let (tx, mut rx) = mpsc::channel::<TerminalMessage>(100);

    // Create a terminal session
    let terminal = match TerminalSession::new(&params.shell, params.cols, params.rows, tx).await {
        Ok(terminal) => terminal,
        Err(e) => {
            let _ = sender
                .send(Message::Text(
                    serde_json::to_string(&TerminalMessage::Error {
                        message: format!("Failed to create terminal: {}", e),
                    })
                        .unwrap(),
                ))
                .await;
            return;
        }
    };

    // Start the terminal session
    let mut terminal = Arc::new(tokio::sync::Mutex::new(terminal));
    {
        let mut terminal_guard = terminal.lock().await;
        terminal_guard.start().await;
    }

    // Clone for the terminal reading task
    let terminal_clone = terminal.clone();

    // Spawn a task to forward terminal messages to the WebSocket
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            // Convert the message to JSON
            let json = match serde_json::to_string(&msg) {
                Ok(json) => json,
                Err(e) => {
                    eprintln!("Failed to serialize terminal message: {}", e);
                    continue;
                }
            };

            // Send the message to the client
            if sender.send(Message::Text(json)).await.is_err() {
                break;
            }

            // If it's an exit message, break
            if matches!(msg, TerminalMessage::Exit { .. }) {
                break;
            }
        }
    });

    // Handle messages from the client
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    // Parse the message
                    let client_msg: ClientMessage = match serde_json::from_str(&text) {
                        Ok(msg) => msg,
                        Err(e) => {
                            eprintln!("Failed to parse client message: {}", e);
                            continue;
                        }
                    };

                    // Lock the terminal
                    let mut terminal = terminal_clone.lock().await;

                    // Process the message
                    match client_msg {
                        ClientMessage::Input { data } => {
                            if let Err(e) = terminal.send_input(data).await {
                                eprintln!("Failed to send input to terminal: {}", e);
                            }
                        }
                        ClientMessage::Resize { cols, rows } => {
                            if let Err(e) = terminal.resize(cols, rows).await {
                                eprintln!("Failed to resize terminal: {}", e);
                            }
                        }
                        ClientMessage::Terminate => {
                            if let Err(e) = terminal.terminate().await {
                                eprintln!("Failed to terminate terminal: {}", e);
                            }
                            break;
                        }
                    }
                }
                Message::Close(_) => {
                    // Client closed the connection
                    let mut terminal = terminal_clone.lock().await;
                    let _ = terminal.terminate().await;
                    break;
                }
                _ => {}
            }
        }
    });

    // Wait for both tasks to complete
    tokio::select! {
        _ = send_task => {}
        _ = recv_task => {}
    }

    // Terminate the terminal if it's still running
    let mut terminal = terminal.lock().await;
    if terminal.is_running().await {
        let _ = terminal.terminate().await;
    }
}

async fn index_handler(config: Arc<TerminalConfig>) -> Html<String> {
    let base_path = &config.base_path;
    let html = include_str!("../static/index.html")
        .replace("href=\"css/style.css\"", &format!("href=\"{}/css/style.css\"", base_path))
        .replace("src=\"js/terminal.js\"", &format!("src=\"{}/js/terminal.js\"", base_path));

    // 确保没有额外的输入框
    let html = html.replace("<textarea", "<!-- <textarea")
        .replace("</textarea>", "</textarea> -->")
        .replace("<input", "<!-- <input")
        .replace("</input>", "</input> -->");

    Html(html)
}

/// Handler for the CSS file.
async fn css_handler() -> impl IntoResponse {
    (
        [("Content-Type", "text/css")],
        include_str!("../static/css/style.css"),
    )
}

/// Handler for the JavaScript file.
async fn js_handler() -> impl IntoResponse {
    (
        [("Content-Type", "application/javascript")],
        include_str!("../static/js/terminal.js"),
    )
}