use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Terminal error: {0}")]
    Terminal(String),
    
    #[error("WebSocket error: {0}")]
    WebSocket(#[from] axum::Error),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Invalid message format")]
    InvalidMessage,
    
    #[error("Session closed")]
    SessionClosed,
    
    #[error("{0}")]
    Custom(String),
}

pub type Result<T> = std::result::Result<T, Error>;