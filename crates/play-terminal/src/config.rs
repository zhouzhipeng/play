//! Configuration for the web terminal.

use std::path::Path;
use std::sync::Arc;

use crate::error::Result;

/// Configuration options for the web terminal.
#[derive(Clone)]
pub struct TerminalConfig {
    /// The path to the static assets directory.
    pub assets_dir: Option<Arc<str>>,

    /// The maximum number of terminals per user.
    pub max_terminals_per_user: usize,

    /// The timeout in seconds for inactive terminals.
    pub terminal_timeout_secs: u64,

    /// The base path for the terminal API.
    pub base_path: Arc<str>,

    /// Whether to use embedded assets.
    pub use_embedded_assets: bool,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            assets_dir: None,
            max_terminals_per_user: 4,
            terminal_timeout_secs: 600,  // 10 minutes
            base_path: "/terminal".into(),
            use_embedded_assets: true,
        }
    }
}

impl TerminalConfig {
    /// Creates a new configuration with the given static assets directory.
    pub fn with_assets_dir<P: AsRef<Path>>(assets_dir: P) -> Result<Self> {
        let path = assets_dir.as_ref().to_str().ok_or_else(|| {
            crate::error::Error::Configuration("Assets path contains invalid UTF-8".into())
        })?;

        Ok(Self {
            assets_dir: Some(path.into()),
            use_embedded_assets: false,
            ..Default::default()
        })
    }

    /// Sets the maximum number of terminals per user.
    pub fn with_max_terminals(mut self, max: usize) -> Self {
        self.max_terminals_per_user = max;
        self
    }

    /// Sets the timeout for inactive terminals.
    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.terminal_timeout_secs = seconds;
        self
    }

    /// Sets the base path for the terminal API.
    pub fn with_base_path<S: Into<Arc<str>>>(mut self, path: S) -> Self {
        self.base_path = path.into();
        self
    }
}