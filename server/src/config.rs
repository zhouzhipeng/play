use std::{env, fs};
use std::path::Path;
use anyhow::anyhow;

use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::info;

use play_shared::{ file_path};
use play_shared::constants::DATA_DIR;
use crate::render_template_new;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    #[serde(default="default_log_level")]
    pub log_level: String,
    pub server_port: u32,
    #[serde(default)]
    pub use_test_pool: bool,
    pub redis_uri: Vec<String>,
    #[serde(default)]
    pub redis_url: Option<String>,
    pub database: Database,
    #[serde(default)]
    pub upgrade_url: String,
    #[serde(default)]
    pub https_cert: HttpsCert,
    #[serde(default)]
    pub shortlinks: Vec<ShortLink>,
    #[serde(default)]
    pub auth_config: AuthConfig,
    #[serde(default)]
    pub domain_proxy: Vec<DomainProxy>,
    #[serde(default)]
    pub misc_config: MiscConfig,
    #[serde(default)]
    pub cache_config: CacheConfig,
    #[serde(default)]
    pub plugin_config: Vec<PluginConfig>,

}

#[derive(Deserialize,Serialize, Debug, Clone, Default)]
pub struct CacheConfig {
    #[serde(default)]
    pub cf_token: String,
    #[serde(default)]
    pub cf_purge_cache_url: String,
}

#[derive(Deserialize,Serialize, Debug, Clone, Default)]
pub struct PluginConfig {
    #[serde(default)]
    pub proxy_domain: String,
    #[serde(default)]
    pub url_prefix: String,
    #[serde(default)]
    pub file_path: String,
    #[serde(default)]
    pub name: String,
    /// if true, will load config.toml content and pass to plugin execution.
    #[serde(default)]
    pub need_config_file: bool,
    #[serde(default)]
    pub is_server: bool,
    #[serde(default)]
    pub disable: bool,
    #[serde(default)]
    pub create_process: bool,

}
#[derive(Deserialize,Serialize, Debug, Clone, Default)]
pub struct DomainProxy {
    #[serde(default)]
    pub proxy_domain: String,
    #[serde(default)]
    pub folder_path: String,
}

#[derive(Deserialize,Serialize, Debug, Clone, Default)]
pub struct ShortLink {
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub download: bool,
    #[serde(default)]
    pub auth: bool,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct AuthConfig {
    pub enabled: bool,
    pub fingerprints: Vec<String>,
    pub whitelist: Vec<String>,
    pub passcode: String,

}
#[derive(Deserialize, Debug, Clone, Default)]
pub struct MiscConfig {
    pub mail_notify_url: String,
}





fn default_log_level()->String{
    "INFO".to_string()
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct HttpsCert {
    pub https_port: u16,
    #[serde(default)]
    pub auto_redirect : bool,
    /// first domain is main domain ,other domain will serve folder under $files/$domain_name
    pub domains: Vec<String>,
    pub emails: Vec<String>,

}




#[derive(Deserialize, Debug, Clone)]
pub struct Database {
    pub url: String,
}


const CONFIG: &str = include_str!(file_path!("/config.toml"));

pub async  fn read_config_file(render_lua: bool)->anyhow::Result<String>{
    let file_path = format!("config.toml");
    let final_path = Path::new(env::var(DATA_DIR)?.as_str()).join(file_path.as_str());

    // info!("config path : {:?}", final_path);
    let mut content = fs::read_to_string(&final_path)?;

     if render_lua{
         // run lua template
        content = render_template_new(&content, json!({})).await?
    }

    Ok(content)

}
pub fn save_config_file(content: &str)->anyhow::Result<()>{
    let file_path = format!("config.toml");
    let final_path = Path::new(env::var(DATA_DIR)?.as_str()).join(file_path.as_str());

    // info!("config path : {:?}", final_path);

    fs::write(&final_path, content)?;
    Ok(())

}

pub fn get_config_path()->anyhow::Result<String>{
    let file_path = format!("config.toml");
    let final_path = Path::new(env::var(DATA_DIR)?.as_str()).join(file_path.as_str());

    // info!("config path : {:?}", final_path);
    Ok(final_path.to_str().unwrap().to_string())
}

pub async fn init_config(render_lua: bool) -> anyhow::Result<Config> {
    let config_content =  read_config_file(render_lua).await?;

    let  config: Config = toml::from_str(&config_content)?;
    //println!("using config file  content >>  {:?}",  config);
    Ok(config)
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.

    use play_shared::constants::HOST;
    use play_shared::get_workspace_root;
    use super::*;

    #[tokio::test]
    async  fn test_read_config_file() -> anyhow::Result<()> {
        env::set_var(DATA_DIR, format!("{}{}", get_workspace_root(), "/server/output_dir"));
        env::set_var("HOST", "http://localhost:3000");
        let content = read_config_file(true).await?;
        println!("{}", content);
        Ok(())
    }
}