use std::sync::Arc;

use axum::{headers, Router, TypedHeader};
use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
};
use axum::routing::get;
//allows to split the websocket stream into separate TX and RX branches
use tracing::info;

use crate::AppState;

pub fn init() -> Router<Arc<AppState>> {
    Router::new()
        .route("/ws", get(ws_handler))
}



async fn ws_handler(
    ws: WebSocketUpgrade,
    user_agent: Option<TypedHeader<headers::UserAgent>>,
) -> impl IntoResponse {
    if let Some(TypedHeader(user_agent)) = user_agent {
        info!("`{}` connected", user_agent.as_str());
    }

    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    loop{
        if let Some(msg) = socket.recv().await {
            if let Ok(msg) = msg {
                match msg{
                    Message::Text(msg) => {
                        info!("Client says: {:?}", msg);
                        //客户端发什么，服务端就回什么（只是演示而已）
                        if let Err(e) = socket
                            .send(Message::Text(format!("{:?}", r#"
                             <div id="notifications" hx-swap-oob="beforeend">
  <p>New messages</p>
 </div>
                            "#)))
                            .await
                        {
                            info!("send msg error  : {:?}", e);
                            return;
                        }
                    }
                    Message::Close(e) => {
                        info!("client disconnected : {:?}", e);
                        return;
                    }
                    _=>{}
                }

            } else {
                info!("client disconnected");
                return;
            }
        }
    }

}