use std::sync::Arc;

use axum::Router;
use axum::routing::get;
use serde_json::json;

use crate::{AppState, template};
use crate::controller::{HTML, S};

pub fn init() -> Router<Arc<AppState>> {
    Router::new()
        // .route("/pages/api-manager", get(api_manager))
}


