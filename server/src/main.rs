
use std::fmt::{Debug, Formatter, write};
use std::{fs, io};
use axum::Router;
use rustpython_vm::pyclass;
#[cfg(feature = "tower-livereload")]
use tower_livereload::LiveReloadLayer;
use tracing::info;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::filter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use play::config::init_config;
use shared::{generate_code, increment, inspect_struct, MyTrait};

use play::controller::routers;
use play::init_app_state;

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

#[derive(MyTrait)]
#[inspect_struct("hello")]
struct MyStruct {
    field1: i32,
    field2: String,
    field3: f64,
}


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


generate_code!();

#[tokio::main]
async fn main() {

    //init output_dir
    fs::create_dir_all("output_dir").expect("Err: create output_dir failed.");

    // initialize tracing
    let filter = filter::Targets::new()
        .with_target("rustpython_vm", LevelFilter::ERROR)
        .with_default(LevelFilter::INFO)
    ;
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer()
            .pretty()
            .with_writer(io::stdout)
        )
        .with(filter)
        .init();

    //init config
    // init config
    let config = init_config();

    let server_port = config.server_port;

    //init app_state
    let app_state = init_app_state(config, false).await;
    info!("app state init ok.");

    let mut router = routers(app_state);
    router = setup_layer(router);



    info!("server start at port : {} ...", server_port);
    // run it with hyper on localhost:3000
    axum::Server::bind(&format!("0.0.0.0:{}", server_port).parse().unwrap())
        .serve(router.into_make_service())
        .await
        .unwrap();
}

