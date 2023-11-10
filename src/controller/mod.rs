use std::sync::Arc;

use axum::Router;

use crate::AppState;

mod index_controller;
mod static_controller;
mod user_controller;

pub fn routers(app_state: Arc<AppState>) -> Router {
    Router::new()
        .merge(index_controller::init())
        .merge(user_controller::init())
        .with_state(app_state)
        .merge(static_controller::init())
}