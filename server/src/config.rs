use std::{env, fs};
use std::path::Path;
use anyhow::anyhow;

use serde::{Deserialize, Serialize};
use tracing::{error, info};
use play_mcp::McpConfig;
use play_shared::{ file_path};
use play_shared::constants::DATA_DIR;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    #[serde(default="default_log_level")]
    pub log_level: String,
    pub server_port: u32,
    #[serde(default)]
    pub use_test_pool: bool,
    pub redis_uri: Vec<String>,
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

    #[cfg(feature = "play-mcp")]
    #[serde(default)]
    pub mcp_config: McpConfig,

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

pub fn read_config_file()->anyhow::Result<String>{
    let file_path = format!("config.toml");
    let final_path = Path::new(env::var(DATA_DIR)?.as_str()).join(file_path.as_str());

    // info!("config path : {:?}", final_path);

    let content = fs::read_to_string(&final_path)?;
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

pub fn init_config(use_memory: bool) -> Config {
    let config_content = if !use_memory {


        let file_path = format!("config.toml");
        let final_path = Path::new(env::var(DATA_DIR).unwrap().as_str()).join(file_path.as_str());

        // println!("config path : {:?}", final_path);

        if !final_path.exists() {
            //copy content to output dir.
            fs::write(&final_path, CONFIG).expect("write file error!");
        }


        fs::read_to_string(&final_path).expect(format!("config file : {}  not existed!", file_path).as_str())
    } else {
        let data_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("output_dir").as_path().to_str().unwrap().to_string();
        env::set_var(DATA_DIR, &data_dir);
        let file_path = format!("config.toml");
        let final_path = Path::new(env::var(DATA_DIR).unwrap().as_str()).join(file_path.as_str());

        // println!("config path : {:?}", final_path);

        if final_path.exists() {
            fs::read_to_string(&final_path).expect(format!("config file : {}  not existed!", file_path).as_str())
        }else{

            CONFIG.to_string()
        }



    };
    let r= toml::from_str(&config_content);
    if let Err(e) = r {
        let msg = format!("load config error >> {}",e.to_string());
        eprintln!("{}", msg);
        panic!("{}", msg);
    }
    let  config: Config = r.unwrap();
    //println!("using config file  content >>  {:?}",  config);
    config
}