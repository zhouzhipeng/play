use std::{env, fs};
use std::path::Path;
use anyhow::anyhow;

use serde::Deserialize;
use tracing::info;

use shared::{ file_path};
use shared::constants::DATA_DIR;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub server_port: u32,
    #[serde(default)]
    pub use_test_pool: bool,
    pub redis_uri: Vec<String>,
    pub database: Database,
    #[serde(default)]
    pub upgrade_url: String,
    #[serde(default)]
    pub https_cert: HttpsCert,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct HttpsCert {
    pub https_port: u16,
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

    info!("config path : {:?}", final_path);

    let content = fs::read_to_string(&final_path)?;
    Ok(content)

}
pub fn save_config_file(content: &str)->anyhow::Result<()>{
    let file_path = format!("config.toml");
    let final_path = Path::new(env::var(DATA_DIR)?.as_str()).join(file_path.as_str());

    info!("config path : {:?}", final_path);

    fs::write(&final_path, content)?;
    Ok(())

}

pub fn get_config_path()->anyhow::Result<String>{
    let file_path = format!("config.toml");
    let final_path = Path::new(env::var(DATA_DIR)?.as_str()).join(file_path.as_str());

    info!("config path : {:?}", final_path);
    Ok(final_path.to_str().unwrap().to_string())
}

pub fn init_config(use_memory: bool) -> Config {
    let config_content = if !use_memory {


        let file_path = format!("config.toml");
        let final_path = Path::new(env::var(DATA_DIR).unwrap().as_str()).join(file_path.as_str());

        info!("config path : {:?}", final_path);

        if !final_path.exists() {
            //copy content to output dir.
            fs::write(&final_path, CONFIG).expect("write file error!");
        }


        fs::read_to_string(&final_path).expect(format!("config file : {}  not existed!", file_path).as_str())
    } else {
        CONFIG.to_string()
    };
    let  config: Config = toml::from_str(&config_content).unwrap();
    info!("using config file  content >>  {:?}",  config);
    config
}