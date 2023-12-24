use std::{fs, io};
use std::path::Path;
use std::time::Duration;
use axum::body::Body;
use axum::http::Request;


use axum::Router;


use tracing::info;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::filter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use play::{CONFIG, file_path, init_app_state, start_server};
use play::routers;




#[tokio::main]
async fn main()->anyhow::Result<()> {

    //init output_dir
    fs::create_dir_all("output_dir").expect("Err: create output_dir failed.");

    // initialize tracing
    let filter = filter::Targets::new()
        // .with_target("rustpython_vm", LevelFilter::ERROR)
        .with_default(LevelFilter::INFO)
    ;

    #[cfg(feature = "debug")]
    let writer = io::stdout;
    #[cfg(not(feature = "debug"))]
    let file_appender = tracing_appender::rolling::never("output_dir", "play.log.txt");
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
    let config = &CONFIG;

    let _server_port = config.server_port;

    //init app_state
    let app_state = init_app_state(config, false).await;
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
        let livereload = livereload.reload_interval(Duration::from_secs(3));
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

    #[cfg(feature = "debug")]
    info!("using debug mode, will auto reload templates and static pages.");

    #[cfg(not(feature = "ui"))]
    start_server( router, app_state).await;

    #[cfg(feature = "ui")]
    {
        tokio::spawn(async move{
            start_server( router, app_state).await;
        });

        ui::start_window("http://127.0.0.1:3000")?;

    }



    Ok(())

}

