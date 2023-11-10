use std::sync::Arc;

use axum::{body, Router, routing::get};
use axum::body::{Empty, Full};
use axum::extract::Path;
use axum::handler::HandlerWithoutStateExt;
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use crossbeam_channel::bounded;
use tokio::spawn;
use tracing::info;

use play::{AppState, STATIC_DIR, TemplateData};
use play::controller::index_controller;
use play::threads::py_runner;

#[tokio::main]
async fn main() {
    // initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

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
        .route("/test", get(index_controller::htmx_test))
        .route("/hello", get(index_controller::hello))
        .with_state(app_state);

    let static_router = axum::Router::new()
        .route("/static/*path", get(static_path));


    info!("server start...");
    // run it with hyper on localhost:3000
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.merge(static_router).into_make_service())
        .await
        .unwrap();
}


async fn static_path(Path(path): Path<String>) -> impl IntoResponse {
    let path = path.trim_start_matches('/');
    let mime_type = mime_guess::from_path(path).first_or_text_plain();

    match STATIC_DIR.get_file(path) {
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(body::boxed(Empty::new()))
            .unwrap(),
        Some(file) => Response::builder()
            .status(StatusCode::OK)
            .header(
                header::CONTENT_TYPE,
                HeaderValue::from_str(mime_type.as_ref()).unwrap(),
            )
            .body(body::boxed(Full::from(file.contents())))
            .unwrap(),
    }
}
