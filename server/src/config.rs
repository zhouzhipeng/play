use std::fs;
use std::path::{Path, PathBuf};
use serde::Deserialize;
use tracing::info;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub use_test_pool: bool,
    pub redis_uri: Vec<String>,
    pub database: Database,

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
    let file_path = format!("config_{}.toml", env!("ENV"));

    let output_dir = "output_dir";
    let target_dir = PathBuf::from(output_dir); // Doesn't need to exist

    if !target_dir.exists() {
        info!("output_dir not existed , ready to create it.");
        if let Ok(_)= fs::create_dir(target_dir){

            //copy content to output dir.
            fs::write(Path::new(output_dir).join(file_path.as_str()), CONFIG).expect("write file error!");

        }
    }


    let config_content = fs::read_to_string(Path::new(output_dir).join(file_path.as_str())).expect(format!("config file : {}  not existed!", file_path).as_str());
    let config: Config = toml::from_str(config_content.as_str()).unwrap();
    println!("init config file : {}, content >>  {:?}", file_path,  config);
    config

}