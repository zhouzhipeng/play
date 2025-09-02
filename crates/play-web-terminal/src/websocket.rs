use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info, debug};

use crate::{local_terminal::LocalTerminal, session_manager::SessionManager, Result};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TerminalMessage {
    Connect,
    ConnectToSession {
        session_name: String,
    },
    CreateSession {
        name: Option<String>,
    },
    ListSessions,
    DeleteSession {
        name: String,
    },
    Input {
        data: String,
    },
    Resize {
        cols: u32,
        rows: u32,
    },
    Disconnect,
    Ping,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum TerminalResponse {
    Connected {
        session_name: Option<String>,
        tmux_available: bool,
    },
    SessionCreated {
        session: crate::session_manager::TmuxSession,
    },
    SessionList {
        sessions: Vec<crate::session_manager::TmuxSession>,
    },
    SessionDeleted {
        name: String,
    },
    Output { data: String },
    Error { message: String },
    Disconnected,
    Pong,
}

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    session_manager: Arc<SessionManager>,
) -> Response {
    debug!("websocket_handler called, upgrading connection...");
    ws.on_upgrade(move |socket| handle_socket(socket, session_manager))
}

async fn handle_socket(socket: WebSocket, session_manager: Arc<SessionManager>) {
    debug!("handle_socket called - WebSocket connection established!");
    let (mut sender, mut receiver) = socket.split();
    // Increase channel capacity for large data transfers (e.g., cat large files)
    let (tx, mut rx) = mpsc::channel::<TerminalResponse>(10000);
    
    let tx_clone = tx.clone();
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let json = match serde_json::to_string(&msg) {
                Ok(json) => json,
                Err(e) => {
                    error!("Failed to serialize message: {}", e);
                    continue;
                }
            };
            
            if let Err(e) = sender.send(Message::Text(json.into())).await {
                debug!("Failed to send WebSocket message: {}", e);
                break;
            }
        }
        debug!("WebSocket sender task completed");
    });
    
    let mut terminal: Option<LocalTerminal> = None;
    let mut current_session: Option<String> = None;
    
    while let Some(msg) = receiver.next().await {
        if let Ok(msg) = msg {
            if let Message::Text(text) = msg {
                match serde_json::from_str::<TerminalMessage>(&text) {
                    Ok(terminal_msg) => {
                        debug!("Received message: {:?}", terminal_msg);
                        match terminal_msg {
                            TerminalMessage::Connect => {
                                debug!("Creating local terminal");
                                
                                match LocalTerminal::new(None).await {
                                    Ok(mut local_term) => {
                                        let output_tx = tx_clone.clone();
                                        local_term.start(output_tx).await;
                                        terminal = Some(local_term);
                                        debug!("Local terminal created and started");
                                        let _ = tx_clone.send(TerminalResponse::Connected {
                                            session_name: None,
                                            tmux_available: session_manager.is_tmux_available(),
                                        }).await;
                                    }
                                    Err(e) => {
                                        error!("Failed to create terminal: {}", e);
                                        let _ = tx_clone.send(TerminalResponse::Error {
                                            message: e.to_string(),
                                        }).await;
                                    }
                                }
                            }
                            TerminalMessage::ConnectToSession { session_name } => {
                                debug!("Connecting to tmux session: {}", session_name);
                                
                                if let Some(mut term) = terminal.take() {
                                    term.disconnect().await;
                                }
                                
                                match LocalTerminal::new(Some(session_name.clone())).await {
                                    Ok(mut local_term) => {
                                        let output_tx = tx_clone.clone();
                                        local_term.start(output_tx).await;
                                        
                                        if session_manager.is_tmux_available() {
                                            let _ = session_manager.attach_to_session(&session_name);
                                        }
                                        
                                        terminal = Some(local_term);
                                        current_session = Some(session_name.clone());
                                        debug!("Connected to tmux session: {}", session_name);
                                        let _ = tx_clone.send(TerminalResponse::Connected {
                                            session_name: Some(session_name),
                                            tmux_available: session_manager.is_tmux_available(),
                                        }).await;
                                    }
                                    Err(e) => {
                                        error!("Failed to connect to session: {}", e);
                                        let _ = tx_clone.send(TerminalResponse::Error {
                                            message: e.to_string(),
                                        }).await;
                                    }
                                }
                            }
                            TerminalMessage::CreateSession { name } => {
                                debug!("Creating new tmux session");
                                
                                match session_manager.create_session(name) {
                                    Ok(session) => {
                                        let _ = tx_clone.send(TerminalResponse::SessionCreated {
                                            session,
                                        }).await;
                                    }
                                    Err(e) => {
                                        error!("Failed to create session: {}", e);
                                        let _ = tx_clone.send(TerminalResponse::Error {
                                            message: e.to_string(),
                                        }).await;
                                    }
                                }
                            }
                            TerminalMessage::ListSessions => {
                                debug!("Listing tmux sessions");
                                let sessions = session_manager.list_sessions();
                                let _ = tx_clone.send(TerminalResponse::SessionList {
                                    sessions,
                                }).await;
                            }
                            TerminalMessage::DeleteSession { name } => {
                                debug!("Deleting tmux session: {}", name);
                                
                                if current_session.as_ref() == Some(&name) {
                                    if let Some(mut term) = terminal.take() {
                                        term.disconnect().await;
                                    }
                                    current_session = None;
                                }
                                
                                match session_manager.delete_session(&name) {
                                    Ok(_) => {
                                        let _ = tx_clone.send(TerminalResponse::SessionDeleted {
                                            name,
                                        }).await;
                                    }
                                    Err(e) => {
                                        error!("Failed to delete session: {}", e);
                                        let _ = tx_clone.send(TerminalResponse::Error {
                                            message: e.to_string(),
                                        }).await;
                                    }
                                }
                            }
                            TerminalMessage::Input { data } => {
                                debug!("Sending input to terminal: {:?}", data);
                                if let Some(ref mut term) = terminal {
                                    if let Err(e) = term.send_input(&data).await {
                                        error!("Failed to send input: {}", e);
                                        let _ = tx_clone.send(TerminalResponse::Error {
                                            message: e.to_string(),
                                        }).await;
                                    } else {
                                        debug!("Input sent successfully");
                                    }
                                } else {
                                    error!("No terminal available to send input");
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
                                if let Some(session_name) = current_session.as_ref() {
                                    let _ = session_manager.detach_from_session(session_name);
                                }
                                if let Some(mut term) = terminal.take() {
                                    term.disconnect().await;
                                }
                                let _ = tx_clone.send(TerminalResponse::Disconnected).await;
                                break;
                            }
                            TerminalMessage::Ping => {
                                debug!("Received ping, sending pong");
                                let _ = tx_clone.send(TerminalResponse::Pong).await;
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
    
    if let Some(session_name) = current_session.as_ref() {
        let _ = session_manager.detach_from_session(session_name);
    }
    if let Some(mut term) = terminal {
        term.disconnect().await;
    }
    
    debug!("WebSocket connection closed");
}