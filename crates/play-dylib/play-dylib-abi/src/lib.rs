pub mod http_abi;
pub mod server_abi;

use anyhow::Context;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};



/// env info provided by host
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Default)]
pub struct HostContext {
    /// host http url , eg. http://127.0.0.1:3000
    pub host_url: String,
    pub data_dir: String,
    pub config_text: Option<String>,
}

impl HostContext {
    pub  fn parse_config<T:DeserializeOwned>(&self) -> anyhow::Result<T> {
        let config_text = self.config_text.as_ref().context("config file is required!")?;
        let  config: T = toml::from_str(&config_text)?;
        Ok(config)
    }


    /// Create HostContext from environment variables
    /// Reads config directly from DATA_DIR/config.toml if load_config is true
    pub fn from_env(load_config: bool) -> anyhow::Result<Self> {
        let host_url = std::env::var("HOST")
            .unwrap_or_else(|_| "http://127.0.0.1:3000".to_string());
        let data_dir = std::env::var("DATA_DIR")
            .unwrap_or_else(|_| "".to_string());
        
        // Read config from DATA_DIR/config.toml if load_config is true
        let config_text = if load_config && !data_dir.is_empty() {
            let config_path = std::path::Path::new(&data_dir).join("config.toml");
            if config_path.exists() {
                Some(std::fs::read_to_string(&config_path)
                    .with_context(|| format!("Failed to read config file: {:?}", config_path))?)
            } else {
                None
            }
        } else {
            None
        };
        
        Ok(HostContext {
            host_url,
            data_dir,
            config_text,
        })
    }

}