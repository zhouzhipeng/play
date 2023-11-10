use std::sync::Arc;

use axum::Router;
use axum::routing::get;

use crate::AppState;

pub mod index_controller;
pub mod static_controller;

pub fn routers(app_state: Arc<AppState>) -> Router {
    let app = Router::new()
        .route("/", get(index_controller::root))
        .route("/test", get(index_controller::htmx_test))
        .route("/hello", get(index_controller::hello))
        .with_state(app_state);

    let static_router = axum::Router::new()
        .route("/static/*path", get(static_controller::static_path));

    app.merge(static_router)
}