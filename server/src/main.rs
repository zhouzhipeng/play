use std::{fs, io};
use std::path::Path;


use axum::Router;

use tracing::info;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::filter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use play::{CONFIG, file_path, init_app_state, start_server};
use play::controller::routers;




#[tokio::main]
async fn main() {

    //init output_dir
    fs::create_dir_all("output_dir").expect("Err: create output_dir failed.");

    // initialize tracing
    let filter = filter::Targets::new()
        // .with_target("rustpython_vm", LevelFilter::ERROR)
        .with_default(LevelFilter::INFO)
    ;
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_thread_names(true)
            .pretty()
            .with_writer(io::stdout)
        )
        .with(filter)
        .init();

    //init config
    // init config
    let config = &CONFIG;

    let _server_port = config.server_port;

    //init app_state
    let app_state = init_app_state(config, false).await;
    info!("app state init ok.");

    let  mut router = routers(app_state);
    #[cfg(feature = "debug")]  //to make `watcher` live longer.
    let mut watcher;
    #[cfg(feature = "debug")]
    {
        use tower_livereload::LiveReloadLayer;
        use notify::Watcher;
        info!("tower-livereload is enabled!");
        let livereload = LiveReloadLayer::new();
        let reloader = livereload.reloader();
        watcher = notify::recommended_watcher(move |_| {
            info!("reloading...");
            reloader.reload()
        }).unwrap();
        watcher.watch(Path::new("static"), notify::RecursiveMode::Recursive).unwrap();
        watcher.watch(Path::new("templates"), notify::RecursiveMode::Recursive).unwrap();
        router = router.layer(livereload);
    }



    start_server( router).await;



}

