use tracing::info;

use play::controller::routers;
use play::init_app_state;

include!(concat!(env!("OUT_DIR"), "/hello.rs"));


#[tokio::main]
async fn main() {
    println!("test >>> {}", message());
    // initialize tracing
    tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).init();

    //init app_state
    let app_state = init_app_state().await;
    info!("app state init ok.");

    info!("server start...");
    // run it with hyper on localhost:3000
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(routers(app_state).into_make_service())
        .await
        .unwrap();
}

