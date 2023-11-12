use std::sync::Arc;

use crossbeam_channel::bounded;
use tokio::spawn;
use tracing::info;

use play::{AppState, tables};
use play::controller::routers;
use play::service::template_service::{TemplateData, TemplateService};
use play::threads::py_runner;

include!(concat!(env!("OUT_DIR"), "/hello.rs"));


#[tokio::main]
async fn main() {
    println!("test >>> {}", message());
    // initialize tracing
    tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).init();

    //create a group of channels to handle python code running
    let (req_sender, req_receiver) = bounded::<TemplateData>(0);
    let (res_sender, res_receiver) = bounded::<String>(1);

    // Create an instance of the shared state
    let app_state = Arc::new(AppState {
        template_service: TemplateService::new(req_sender, res_receiver),
        db: tables::init_pool().await,
    });


    //run a thread to run python code.
    spawn(async move { py_runner::run(req_receiver, res_sender).await; });


    info!("server start...");
    // run it with hyper on localhost:3000
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(routers(app_state).into_make_service())
        .await
        .unwrap();
}

