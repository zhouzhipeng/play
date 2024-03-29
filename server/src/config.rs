use std::{env, fs};
use std::path::Path;
use anyhow::anyhow;

use serde::{Deserialize, Serialize};
use tracing::info;

use shared::{ file_path};
use shared::constants::DATA_DIR;

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
    pub email_server_config: EmailServerConfig,
    #[serde(default)]
    pub finance: Finance,
    #[serde(default)]
    pub shortlinks: Vec<ShortLink>,
    #[serde(default)]
    pub http_jobs: Vec<LocalJobConfig>,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct ShortLink {
    pub from: String,
    pub to: String,
    pub jump: bool,
}
#[derive(Deserialize, Debug, Clone, Default)]
pub struct LocalJobConfig {
    pub name: String,
    pub cron: String,
    pub url: String,
    pub params : Vec<(String/*key*/, String/*value*/)>
}


fn default_log_level()->String{
    "INFO".to_string()
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct HttpsCert {
    pub https_port: u16,
    #[serde(default)]
    pub auto_redirect : bool,
    pub domains: Vec<String>,
    pub emails: Vec<String>,

}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct Finance {
    #[serde(default)]
    pub alphavantage_apikeys: Vec<String>,
    #[serde(default)]
    pub rate: Vec<RateItem>,
    #[serde(default)]
    pub portfolio: Vec<PortfolioItem>,

}
#[derive(Deserialize,Serialize, Debug, Clone, Default)]
pub struct PortfolioItem {
    #[serde(default)]
    pub symbol: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub orders: Vec<(f64/*quantity*/, f64/*price*/, String/*date*/)>,
    #[serde(default)]
    pub quantity: f64,
    #[serde(default)]
    pub price: f64,
    #[serde(default)]
    pub tag: String,
    #[serde(default)]
    pub market: PortfolioMarket,
    #[serde(default)]
    pub positions: Vec<PortfolioItemPosition>,
    #[serde(default)]
    pub total_invest: f64,
}
#[derive(Deserialize,Serialize, Debug, Clone, Default)]
pub struct PortfolioItemPosition {
    #[serde(default)]
    pub quantity: f64,
    #[serde(default)]
    pub price: f64,
}
#[derive(Deserialize,Serialize, Debug, Clone, Default)]
pub struct RateItem {
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub target: String,
}

#[derive(Deserialize,Serialize, Debug, Clone, Default,PartialEq)]
pub enum PortfolioMarket {
    #[default]
    US_STOCK,
    HK_STOCK,
    CRYPTO
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct EmailServerConfig {
    #[serde(default="default_email_server_port")]
    pub port: u16,
    pub black_keywords: Vec<String>,
}

fn default_email_server_port()->u16{
    25
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

        println!("config path : {:?}", final_path);

        if !final_path.exists() {
            //copy content to output dir.
            fs::write(&final_path, CONFIG).expect("write file error!");
        }


        fs::read_to_string(&final_path).expect(format!("config file : {}  not existed!", file_path).as_str())
    } else {
        CONFIG.to_string()
    };
    let  config: Config = toml::from_str(&config_content).unwrap();
    println!("using config file  content >>  {:?}",  config);
    config
}