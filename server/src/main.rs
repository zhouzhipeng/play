use std::{env, fs, io, panic};
use std::env::set_var;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use async_channel::RecvError;
use axum::body::Body;
use axum::http::Request;


use axum::Router;
use directories::ProjectDirs;


use tracing::{error, info};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::filter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use mail_server::models::message::Message;

use play::{AppState, handle_email_message, init_app_state, shutdown_another_instance, start_server};
use play::config::init_config;
use play::routers;
use play::tables::email_inbox::EmailInbox;
use shared::constants::DATA_DIR;



#[tokio::main]
async fn main()->anyhow::Result<()> {
    #[cfg(feature = "debug")]
    set_var("RUST_BACKTRACE","1");

    // Set the custom panic hook
    panic::set_hook(Box::new(|panic_info| {
        println!("panic occurred : {:?}", panic_info);
    }));

    #[cfg(not(feature = "debug"))]
    let data_dir  = match ProjectDirs::from("com", "zhouzhipeng",  "play"){
        None => env::var(DATA_DIR)?,
        Some(s) => s.data_dir().to_str().unwrap().to_string(),
    };

    #[cfg(feature = "debug")]
    let data_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("output_dir").as_path().to_str().unwrap().to_string();


    env::set_var(DATA_DIR, &data_dir);
    println!("using data dir : {:?}", data_dir);

    fs::create_dir_all(&data_dir).expect("create data dir failed.");



    // initialize tracing
    let filter = filter::Targets::new()
        // .with_target("rustpython_vm", LevelFilter::ERROR)
        .with_default(LevelFilter::INFO)
    ;

    #[cfg(feature = "debug")]
    let writer = io::stdout;
    #[cfg(not(feature = "debug"))]
    let file_appender = tracing_appender::rolling::never(data_dir, "play.log.txt");
    #[cfg(not(feature = "debug"))]
    let (writer, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_thread_names(true)
            .pretty()
            .with_writer(writer)
        )
        .with(filter)
        .init();

    //init config
    // init config
    let config = init_config(false);

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
                request
                    .headers()
                    .get("HX-Request")
                    .is_none()
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

    let local_url = format!("http://127.0.0.1:{}", server_port);
    shutdown_another_instance(&local_url).await;

    #[cfg(feature = "debug")]
    info!("using debug mode, will auto reload templates and static pages.");


    //start a mail server
    let copy_appstate = app_state.clone();
    tokio::spawn(async move {
        info!("starting mail server...");
        let addr = SocketAddr::from(([0, 0, 0, 0], 25));
        let (sever, rx) = mail_server::smtp::Builder::new().bind(addr).build();
        tokio::spawn(async move {
            loop{
                //handle message
                match rx.recv().await {
                    Ok(msg) => {
                        handle_email_message(&copy_appstate, &msg).await;
                    }
                    Err(e) => {
                        error!("recv mail message error : {:?}", e);
                    }
                }
            }

        });
        sever.serve().expect("create mail server failed!");
    });


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

