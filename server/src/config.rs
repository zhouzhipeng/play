use std::fs;
use std::path::Path;

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
    #[serde(default)]
    pub upgrade_url: String,

}

#[derive(Deserialize, Debug)]
pub struct Database {
    pub url: String,
}


const CONFIG: &str = include_str!(file_path!("/config.toml"));

pub fn init_config(use_memory: bool) -> Config {
    let config_content = if !use_memory {
        //init output_dir
        fs::create_dir_all("output_dir").expect("create output_dir failed!");

        let file_path = format!("config.toml");


        let final_path = Path::new("output_dir").join(file_path.as_str());

        info!("config path : {:?}", final_path);

        if !final_path.exists() {
            //copy content to output dir.
            fs::write(&final_path, CONFIG).expect("write file error!");
        }


        fs::read_to_string(&final_path).expect(format!("config file : {}  not existed!", file_path).as_str())
    } else {
        CONFIG.to_string()
    };
    let config: Config = toml::from_str(&config_content).unwrap();
    info!("using config file  content >>  {:?}",  config);
    config
}