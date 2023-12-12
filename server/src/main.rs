use std::{fs, io};
use std::fmt::Debug;

use axum::Router;
#[cfg(feature = "tower-livereload")]
use tower_livereload::LiveReloadLayer;
use tracing::info;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::filter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use play::{CONFIG, init_app_state, start_server};
use play::controller::routers;
use shared::{inspect_struct, MyTrait};

// include!(concat!(env!("OUT_DIR"), "/hello.rs"));
#[cfg(feature = "tower-livereload")]
fn setup_layer(router: Router) -> Router {
    info!("tower-livereload is enabled!");
    router.layer(LiveReloadLayer::new())
}

#[cfg(not(feature = "tower-livereload"))]
fn setup_layer(router: Router) -> Router {
    router
}

// #[macro_use]
// extern crate macros;


macro_rules! print_literal {
    (number: $val:expr) => {
        println!("Received a number: {}", $val);
    };
    (string: $val:expr) => {
        println!("Received a string: {}", $val);
    };
    (bool: $val:expr) => {
        println!("Received a boolean: {}", $val);
    };
}


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

    let server_port = config.server_port;

    //init app_state
    let app_state = init_app_state(config, false).await;
    info!("app state init ok.");

    let mut router = routers(app_state);
    router = setup_layer(router);


    start_server( router).await;



}

