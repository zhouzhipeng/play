use serde::Deserialize;
use crate::file_path;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub server_port: u32,
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
    let config: Config = toml::from_str(CONFIG).unwrap();
    println!("init config  content >>  {:?}", config);
    config
}