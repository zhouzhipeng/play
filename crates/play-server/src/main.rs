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


use play_server::{ files_dir, init_app_state, shutdown_another_instance, start_server};
use play_server::config::{init_config, read_config_file, PluginConfig};
use play_server::routers;

use play_shared::constants::{DATA_DIR};

#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main()->anyhow::Result<()> {

    // Set the custom panic hook
    panic::set_hook(Box::new(|panic_info| {

        println!("panic occurred : {:?}", panic_info);
        error!("panic occurred : {:?}", panic_info);
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

    //inject env for py_runner
    set_var("HOST", format!("http://127.0.0.1:{}", config.server_port));

    set_var("FP", &config.auth_config.fingerprints.get(0).unwrap_or(&"".to_string()));

    let log_level = match config.log_level.as_str(){
        "TRACE"=> LevelFilter::TRACE,
        "DEBUG"=> LevelFilter::DEBUG,
        "INFO"=> LevelFilter::INFO,
        "ERROR"=> LevelFilter::ERROR,
        _=> LevelFilter::INFO,
    };



    // initialize tracing
    let filter = filter::Targets::new()
        .with_target("rustls_acme", LevelFilter::TRACE)
        .with_default(log_level)
    ;


    let file_appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY) // rotate log files once every hour
        .filename_prefix("play") // log file names will be prefixed with `myapp.`
        .filename_suffix("log") // log file names will be suffixed with `.log`
        .max_log_files(10)
        .max_file_size(100*1024*1024 /*100MB*/)
        .build(data_dir) // try to build an appender that stores log files in `/var/log`
        .expect("initializing rolling file appender failed");

    let (writer, _guard) = tracing_appender::non_blocking(file_appender);

    #[cfg(not(feature = "debug"))]
    tracing_subscriber::fmt()
        .with_file(true)
        .with_line_number(true)
        .with_thread_names(true)
        .pretty()
        .with_writer(writer)
        .finish()
        .init();



    #[cfg(feature = "debug")]
    tracing_subscriber::fmt()
        .with_file(true)
        .with_line_number(true)
        .with_thread_names(true)
        .pretty()
        .with_writer(std::io::stdout)
        .finish()
        .init();



    info!("using log level : {}", log_level);

    //init config

    let server_port = config.server_port;

    //init app_state
    let app_state = init_app_state(&config, false).await.unwrap();
    info!("app state init ok.");

    info!("current path : {}", env!("CARGO_MANIFEST_DIR"));

    #[allow(unused_mut)]
    let  mut router = routers(app_state.clone()).await.unwrap();




    if  !local_port_available( server_port as u16) {
        let local_url = format!("http://127.0.0.1:{}", server_port);
        shutdown_another_instance(&local_url).await;
    }




    #[cfg(feature = "play-dylib-loader")]
    {
        use play_dylib_loader::{load_and_run_server, HostContext};
        let copy_appstate = app_state.clone();
        let plugins : Vec<&PluginConfig> =  copy_appstate.config.plugin_config.iter().filter(|plugin|!plugin.disable && plugin.is_server).collect();
        for plugin in plugins {
            let path = plugin.file_path.to_string();
            let  create_process= plugin.create_process;
            let _:JoinHandle<anyhow::Result<()>> =  tokio::spawn(async move {
                info!("load_and_run_server >> {}", path);

                //support external process
                if create_process{
                    info!("plugin.create_process is true");
                    make_executable_if_needed(&path)?;
                    if let Err(e) = Command::new(&path)
                        .output()
                        .await{
                        error!(" plugin process load_and_run_server error: {:?}", e);
                    }
                }else{
                    let context = HostContext {
                        host_url: env::var("HOST")?,
                        plugin_prefix_url: "".to_string(),
                        data_dir: env::var(DATA_DIR)?,
                        config_text: Some(read_config_file(true).await?),
                    };

                    if let Err(e) = load_and_run_server(&path,context).await{
                        error!(" plugin load_and_run_server error: {:?}", e);
                    }
                }

                Ok(())
            });
        }

    }



    #[cfg(not(feature = "play-ui"))]
    start_server( router, app_state).await;

    #[cfg(feature = "play-ui")]
    {
        tokio::spawn(async move{
            start_server(router, app_state).await.expect("start api server failed!");
        });

        play_ui::start_window(&format!("http://127.0.0.1:{}",server_port))?;

    }



    Ok(())

}



pub fn local_port_available(port: u16) -> bool {
    match TcpListener::bind(("0.0.0.0", port)) {
        Ok(_) => true,
        Err(_) => false,
    }
}


fn make_executable_if_needed(file_path: &str) -> std::io::Result<bool> {
    let path = Path::new(file_path);

    // 获取当前文件权限
    let mut perms = fs::metadata(path)?.permissions();

    // 获取当前权限模式
    let mode = perms.mode();

    // 检查是否已经有执行权限
    // 检查所有者、组和其他用户的执行权限
    let has_execute_permission =
        (mode & 0o100 != 0) || // 所有者执行权限
            (mode & 0o010 != 0) || // 组执行权限
            (mode & 0o001 != 0);   // 其他用户执行权限

    if !has_execute_permission {
        // 如果没有执行权限，则添加
        let new_mode = mode | 0o111; // 为所有者、组和其他用户添加执行权限
        perms.set_mode(new_mode);

        // 应用新权限
        fs::set_permissions(path, perms)?;

        info!("Added execute permissions to {}", file_path);
        Ok(true)
    } else {
        info!("File already has execute permissions");
        Ok(false)
    }
}