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
    pub plugin_prefix_url: String,
    pub data_dir: String,
    pub config_text: Option<String>,
}

impl HostContext {
    pub  fn parse_config<T:DeserializeOwned>(&self) -> anyhow::Result<T> {
        let config_text = self.config_text.as_ref().context("config file is required!")?;
        let  config: T = toml::from_str(&config_text)?;
        Ok(config)
    }

    pub async fn render_template(&self, raw: &str, data : Value) -> anyhow::Result<String> {
        let resp = Client::new().post(&format!("{}/functions/render-template", self.host_url.as_str()))
            .form(&json!({
                "raw_content": raw,
                "data": data.to_string()
            }))
            .send().await?.text().await?;
        Ok(resp)
    }

    /// Create HostContext from environment variables and config file
    /// This simplifies server plugins by eliminating FFI data passing
    pub fn from_env() -> anyhow::Result<Self> {
        let host_url = std::env::var("HOST")
            .unwrap_or_else(|_| "http://127.0.0.1:3000".to_string());
        let plugin_prefix_url = std::env::var("PLUGIN_PREFIX_URL")
            .unwrap_or_else(|_| "".to_string());
        let data_dir = std::env::var("DATA_DIR")
            .unwrap_or_else(|_| "".to_string());
        
        // 从配置文件路径读取配置内容，而不是从环境变量
        let config_text = if let Ok(config_path) = std::env::var("CONFIG_FILE_PATH") {
            std::fs::read_to_string(&config_path)
                .with_context(|| format!("Failed to read config file: {}", config_path))?
                .into()
        } else {
            None
        };
        
        Ok(HostContext {
            host_url,
            plugin_prefix_url,
            data_dir,
            config_text,
        })
    }

}