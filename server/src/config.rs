

use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub database: Database,
    pub redis_uri: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct Database {
    pub url: String,
}

#[cfg(ENV = "dev")]
const CONFIG: &str = include_str!("../config/config_dev.toml");
#[cfg(ENV = "prod")]
const CONFIG: &str = include_str!("../config/config_prod.toml");

pub fn init_config() -> Config {
    let config: Config = toml::from_str(CONFIG).unwrap();
    println!("init config file :  content >>  {:?}", config);
    config
}