use std::fs;
use std::path::{Path, PathBuf};
use serde::Deserialize;
use tracing::info;
use crate::file_path;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub server_port: u32,
    #[serde(default)]
    pub use_test_pool: bool,
    pub redis_uri: Vec<String>,
    pub database: Database,

}

#[derive(Deserialize, Debug)]
pub struct Database {
    pub url: String,
}

#[cfg(ENV = "dev")]
const CONFIG: &str = include_str!(file_path!("/config/config_dev.toml"));
#[cfg(ENV = "prod")]
const CONFIG: &str = include_str!(file_path!("/config/config_prod.toml"));

pub fn init_config() -> Config {
    let file_path = format!("config_{}.toml", env!("ENV"));

    let final_path = Path::new("output_dir").join(file_path.as_str());
    //copy content to output dir.
    fs::write(&final_path, CONFIG).expect("write file error!");


    let config_content = fs::read_to_string(&final_path).expect(format!("config file : {}  not existed!", file_path).as_str());
    let config: Config = toml::from_str(config_content.as_str()).unwrap();
    info!("init config file : {}, content >>  {:?}", file_path,  config);
    config
}