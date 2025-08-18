use std::{env, fs, io, panic};
use std::env::set_var;
use std::net::{SocketAddr, TcpListener};
use std::os::unix::prelude::PermissionsExt;
use std::path::Path;
use std::time::Duration;

use axum::body::Body;
use axum::http::Request;
use tokio::process::Command;
use tokio::task::JoinHandle;
use tracing::{error, info};
use tracing::level_filters::LevelFilter;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::filter;
use tracing_subscriber::subscribe::CollectExt;
use tracing_subscriber::util::SubscriberInitExt;


use play_server::{files_dir, init_app_state, shutdown_another_instance, start_server, Config};
use play_server::config::{init_config, read_config_file, PluginConfig};
use play_server::routers;

use play_shared::constants::DATA_DIR;

#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main()->anyhow::Result<()> {

    // Set the custom panic hook
    panic::set_hook(Box::new(|panic_info| {
        let location = panic_info.location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown location".to_string());
        
        let message = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "unknown panic payload".to_string()
        };
        
        let backtrace = std::backtrace::Backtrace::capture();
        
        println!("PANIC at {}: {}", location, message);
        println!("Backtrace:\n{}", backtrace);
        error!("PANIC at {}: {}", location, message);
        error!("Backtrace:\n{}", backtrace);
    }));

    #[cfg(not(feature = "debug"))]
    let data_dir  = match  directories::ProjectDirs::from("com", "zhouzhipeng",  "play"){
        None => env::var(DATA_DIR)?,
        Some(s) => s.data_dir().to_str().unwrap().to_string(),
    };

    #[cfg(feature = "debug")]
    let data_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("output_dir").as_path().to_str().unwrap().to_string();


    env::set_var(DATA_DIR, &data_dir);
    println!("using data dir : {:?}", data_dir);

    //init dir
    fs::create_dir_all(&data_dir).expect("create data dir failed.");
    fs::create_dir_all(files_dir!()).expect("create files dir failed.");


    // init config
    let config = init_config(false).await?;


    play_server::start_server_with_config(data_dir, &config).await?;



    Ok(())

}

pub fn local_port_available(port: u16) -> bool {
    match TcpListener::bind(("0.0.0.0", port)) {
        Ok(_) => true,
        Err(_) => false,
    }
}


