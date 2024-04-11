use std::{env, fs, io, panic};
use std::env::set_var;
use std::net::{SocketAddr, TcpListener};
use std::path::Path;
use std::time::Duration;

use axum::body::Body;
use axum::http::Request;

use tracing::{error, info};
use tracing::level_filters::LevelFilter;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::filter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;


use play::{files_dir, init_app_state, shutdown_another_instance, start_server};
use play::config::init_config;
use play::routers;
use shared::constants::DATA_DIR;

#[tokio::main]
async fn main()->anyhow::Result<()> {
    #[cfg(feature = "debug")]
    set_var("RUST_BACKTRACE","1");

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
    let config = init_config(false);


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
        .build(data_dir) // try to build an appender that stores log files in `/var/log`
        .expect("initializing rolling file appender failed");

    let (writer, _guard) = tracing_appender::non_blocking(file_appender);

    let subscriber = tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer()
            .with_file(true)
            .with_line_number(true)
            .with_thread_names(true)
            .pretty()
            .with_writer(writer)
        );




    #[cfg(feature = "debug")]
    let subscriber = subscriber.with(tracing_subscriber::fmt::Layer::new()
        .with_file(true)
        .with_line_number(true)
        .with_thread_names(true)
        .with_writer(std::io::stdout)); // Console output

    subscriber.with(filter).init();

    info!("using log level : {}", log_level);

    //init config

    let server_port = config.server_port;

    //init app_state
    let app_state = init_app_state(&config, false).await;
    info!("app state init ok.");

    info!("current path : {}", env!("CARGO_MANIFEST_DIR"));

    #[allow(unused_mut)]
    let  mut router = routers(app_state.clone());
    #[cfg(feature = "debug")]  //to make `watcher` live longer.
    let mut watcher;
    #[cfg(feature = "debug")]
    {
        use tower_livereload::LiveReloadLayer;
        use tower_livereload::predicate::Predicate;
        #[derive(Copy, Clone, Debug)]
        pub struct NotHxRequest();
        impl<T> Predicate<Request<T>> for NotHxRequest {
            fn check(&mut self, request: &Request<T>) -> bool {
                let should_inject_js = request
                    .headers()
                    .get("HX-Request")
                    .is_none();
                // info!("should_inject_js : {}", should_inject_js);
                should_inject_js
            }
        }

        use notify::Watcher;
        info!("tower-livereload is enabled!");
        let  livereload = LiveReloadLayer::new();
        let livereload = livereload.reload_interval(Duration::from_secs(1));
        let livereload = livereload.request_predicate::<Body, NotHxRequest>(NotHxRequest {});
        let reloader = livereload.reloader();
        watcher = notify::recommended_watcher(move |_| {
            info!("reloading...");
            reloader.reload()
        }).unwrap();

        watcher.watch(&Path::new(env!("CARGO_MANIFEST_DIR")).join("static"), notify::RecursiveMode::Recursive).unwrap();
        watcher.watch(&Path::new(env!("CARGO_MANIFEST_DIR")).join("templates"), notify::RecursiveMode::Recursive).unwrap();
        router = router.layer(livereload);
    }


    if  !local_port_available( server_port as u16) {
        let local_url = format!("http://127.0.0.1:{}", server_port);
        shutdown_another_instance(&local_url).await;
    }



    #[cfg(feature = "debug")]
    info!("using debug mode, will auto reload templates and static pages.");


    //start a mail server
    #[cfg(feature = "mail_server")]
    {
        let copy_appstate = app_state.clone();
        info!("starting mail server...");
        let addr = SocketAddr::from(([0, 0, 0, 0], config.email_server_config.port));
        let (sever, rx) = mail_server::smtp::Builder::new().bind(addr).build(config.email_server_config.black_keywords.clone());
        tokio::spawn(async move {
            sever.serve().expect("create mail server failed!");
        });
        tokio::spawn(async move {
            loop{
                //handle message
                info!("ready to handle message");
                match rx.recv().await {
                    Ok(msg) => {
                        play::handle_email_message(&copy_appstate, &msg).await;
                    }
                    Err(e) => {
                        error!("recv mail message error : {:?}", e);
                    }
                }
            }

        });
    }

    //start job scheduler
    #[cfg(feature = "job")]
    tokio::spawn(job::run_job_scheduler(config.http_jobs.iter().map(|c|job::JobConfig{
        name: c.name.to_string(),
        cron: c.cron.to_string(),
        url: c.url.to_string(),
        params: c.params.clone(),
    }).collect()));



    #[cfg(not(feature = "ui"))]
    start_server( router, app_state).await;

    #[cfg(feature = "ui")]
    {
        tokio::spawn(async move{
            start_server(router, app_state).await.expect("start api server failed!");
        });

        ui::start_window(&format!("http://127.0.0.1:{}",server_port))?;

    }



    Ok(())

}



pub fn local_port_available(port: u16) -> bool {
    match TcpListener::bind(("0.0.0.0", port)) {
        Ok(_) => true,
        Err(_) => false,
    }
}