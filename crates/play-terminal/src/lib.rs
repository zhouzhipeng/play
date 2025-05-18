//! A web terminal implementation using Axum.
//!
//! This crate provides a web terminal that can be integrated into existing Axum projects.

pub mod config;
pub mod error;
pub mod routes;
pub mod terminal;
pub mod utils;

use axum::Router;
use std::path::Path;
use std::sync::Arc;

use crate::config::TerminalConfig;
use crate::error::Result;
use crate::routes::create_routes;

/// The main terminal application.
pub struct WebTerminal {
    config: Arc<TerminalConfig>,
}

impl WebTerminal {
    /// Creates a new web terminal with the given configuration.
    pub fn new(config: TerminalConfig) -> Self {
        Self {
            config: Arc::new(config),
        }
    }

    /// Creates a new web terminal with default configuration.
    pub fn default() -> Self {
        Self::new(TerminalConfig::default())
    }

    /// Returns the Axum router for the web terminal.
    pub fn router(&self) -> Router {
        create_routes(self.config.clone())
    }
}

/// Creates a WebTerminal instance with the given static assets directory.
pub fn web_terminal_with_assets<P: AsRef<Path>>(assets_dir: P) -> Result<WebTerminal> {
    let config = TerminalConfig::with_assets_dir(assets_dir)?;
    Ok(WebTerminal::new(config))
}