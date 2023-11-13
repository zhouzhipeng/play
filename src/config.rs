use std::{env, fs};

use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub database: Database,
}

#[derive(Deserialize, Debug)]
pub struct Database {
    pub url: String,
}


pub fn init_config() -> Config {
    let file_path = format!("config/config_{}.toml", env::var("ENV").unwrap_or("dev".to_string()));
    let config_content = fs::read_to_string(&file_path).expect(format!("config file : {}  not existed!", file_path).as_str());
    let config: Config = toml::from_str(config_content.as_str()).unwrap();
    println!("init config file : {}, content >>  {:?}", file_path,  config);
    config
}