use std::sync::Arc;

use axum::{Router, routing::get};
use axum::handler::HandlerWithoutStateExt;
use crossbeam_channel::bounded;
use tokio::spawn;

use play::{AppState, TemplateData};
use play::controller::index_controller;
use play::threads::py_runner;

#[tokio::main]
async fn main() {
    // initialize tracing
    tracing_subscriber::fmt::init();

    //

// Create a channel of unbounded capacity.
    let (req_sender, req_receiver) = bounded::<TemplateData>(0);
    let (res_sender, res_receiver) = bounded::<String>(1);


    // Create an instance of the shared state
    let app_state = Arc::new(AppState {
        req_sender,
        res_receiver,
    });

    //run a thread to run python code.
    spawn(async move {
        py_runner::run(req_receiver, res_sender).await;
    });

    // build our application with a route

    let app = Router::new()
        .route("/", get(index_controller::root))
        .with_state(app_state);

    // run it with hyper on localhost:3000
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

