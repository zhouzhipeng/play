use std::sync::Arc;

use axum::routing::get;
use axum::Router;
use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
};
//allows to split the websocket stream into separate TX and RX branches
use tracing::info;

use crate::AppState;

pub fn init() -> Router<Arc<AppState>> {
    Router::new().route("/ws", get(ws_handler))
}

async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    info!("WebSocket connected");

    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    loop {
        if let Some(msg) = socket.recv().await {
            if let Ok(msg) = msg {
                match msg {
                    Message::Text(text) => {
                        let msg = text.to_string();
                        info!("Client says: {:?}", msg);
                        //todo:
                    }
                    Message::Close(_) => {
                        info!("client disconnected");
                        return;
                    }
                    _ => {}
                }
            } else {
                info!("client disconnected");
                return;
            }
        }
    }
}
