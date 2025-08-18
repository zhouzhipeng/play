use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::{error, info, debug};

use crate::{local_terminal::LocalTerminal, Result};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TerminalMessage {
    Connect,
    Input {
        data: String,
    },
    Resize {
        cols: u32,
        rows: u32,
    },
    Disconnect,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum TerminalResponse {
    Connected,
    Output { data: String },
    Error { message: String },
    Disconnected,
}

pub async fn websocket_handler(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(socket: WebSocket) {
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::channel::<TerminalResponse>(100);
    
    let tx_clone = tx.clone();
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let json = serde_json::to_string(&msg).unwrap();
            if sender.send(Message::Text(json.into())).await.is_err() {
                break;
            }
        }
    });
    
    let mut terminal: Option<LocalTerminal> = None;
    
    while let Some(msg) = receiver.next().await {
        if let Ok(msg) = msg {
            if let Message::Text(text) = msg {
                match serde_json::from_str::<TerminalMessage>(&text) {
                    Ok(terminal_msg) => {
                        match terminal_msg {
                            TerminalMessage::Connect => {
                                info!("Creating local terminal");
                                
                                match LocalTerminal::new().await {
                                    Ok(mut local_term) => {
                                        let output_tx = tx_clone.clone();
                                        local_term.start(output_tx).await;
                                        terminal = Some(local_term);
                                    }
                                    Err(e) => {
                                        error!("Failed to create terminal: {}", e);
                                        let _ = tx_clone.send(TerminalResponse::Error {
                                            message: e.to_string(),
                                        }).await;
                                    }
                                }
                            }
                            TerminalMessage::Input { data } => {
                                if let Some(ref mut term) = terminal {
                                    if let Err(e) = term.send_input(&data).await {
                                        error!("Failed to send input: {}", e);
                                        let _ = tx_clone.send(TerminalResponse::Error {
                                            message: e.to_string(),
                                        }).await;
                                    }
                                }
                            }
                            TerminalMessage::Resize { cols, rows } => {
                                if let Some(ref mut term) = terminal {
                                    if let Err(e) = term.resize(cols as u16, rows as u16).await {
                                        error!("Failed to resize terminal: {}", e);
                                    }
                                }
                            }
                            TerminalMessage::Disconnect => {
                                if let Some(mut term) = terminal.take() {
                                    term.disconnect().await;
                                }
                                let _ = tx_clone.send(TerminalResponse::Disconnected).await;
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to parse message: {}", e);
                        let _ = tx_clone.send(TerminalResponse::Error {
                            message: format!("Invalid message format: {}", e),
                        }).await;
                    }
                }
            }
        } else {
            break;
        }
    }
    
    if let Some(mut term) = terminal {
        term.disconnect().await;
    }
    
    info!("WebSocket connection closed");
}