//! Error types for the web terminal.

use std::io;
use thiserror::Error;

/// A result type for the web terminal.
pub type Result<T> = std::result::Result<T, Error>;

/// Error types for the web terminal.
#[derive(Error, Debug)]
pub enum Error {
    /// An I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// A serialization error.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// A terminal error.
    #[error("Terminal error: {0}")]
    Terminal(String),

    /// A configuration error.
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// A WebSocket error.
    #[error("WebSocket error: {0}")]
    WebSocket(String),

    /// An error from spawning a process.
    #[error("Process error: {0}")]
    Process(String),
}