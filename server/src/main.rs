use std::io;
use axum::Router;
#[cfg(feature = "tower-livereload")]
use tower_livereload::LiveReloadLayer;
use tracing::info;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::filter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use play::controller::routers;
use play::init_app_state;
use macros::inspect_struct;

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
#[inspect_struct("hello")]
struct MyStruct {
    field1: i32,
    field2: String,
    field3: f64,
}


#[tokio::main]
async fn main() {
    // let from_dir = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().join(Path::new("client/pkg"));
    //
    // println!("dir : {:?}", from_dir);
    //
    // println!("test >>> {}", message());
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

    //init app_state
    let app_state = init_app_state(false).await;
    info!("app state init ok.");

    let mut router = routers(app_state);
    router = setup_layer(router);

    let server_port = 3000;

    info!("server start at port : {} ...", server_port);
    // run it with hyper on localhost:3000
    axum::Server::bind(&format!("0.0.0.0:{}", server_port).parse().unwrap())
        .serve(router.into_make_service())
        .await
        .unwrap();
}

