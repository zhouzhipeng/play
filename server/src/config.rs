use std::{env, fs};
use std::path::{Path, PathBuf};
use include_dir::{Dir, include_dir};

use serde::Deserialize;
use tracing::info;
use tracing_subscriber::fmt::format;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub database: Database,
}

#[derive(Deserialize, Debug)]
pub struct Database {
    pub url: String,
}

static CONFIG_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/config");

pub fn init_config() -> Config {
    let file_path = format!("config_{}.toml", option_env!("ENV").unwrap_or("dev"));


    let output_dir = "output_dir";
    let target_dir = PathBuf::from(output_dir); // Doesn't need to exist

    if !target_dir.exists() {
        info!("output_dir not existed , ready to create it.");
        fs::create_dir(target_dir).expect(format!("create dir : {} failed", output_dir).as_str());

        //python stdlib
        let data = CONFIG_DIR.get_file(file_path.as_str()).unwrap().contents();

        //copy custom python files to output dir.
        fs::write(Path::new(output_dir).join(file_path.as_str()), data).expect("write file error!");
    }


    let config_content = fs::read_to_string(Path::new(output_dir).join(file_path.as_str())).expect(format!("config file : {}  not existed!", file_path).as_str());
    let config: Config = toml::from_str(config_content.as_str()).unwrap();
    println!("init config file : {}, content >>  {:?}", file_path,  config);
    config
}