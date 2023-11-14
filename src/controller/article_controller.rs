use std::sync::Arc;

use axum::extract::State;
use axum::response::Html;
use axum::Router;
use axum::routing::get;
use serde_json::json;

use crate::AppState;
use crate::controller::{R, S};

pub fn init() -> Router<Arc<AppState>> {
    Router::new()
        .route("/article/index", get(index))
}

async fn index(State(s): S) -> R<Html<String>> {
    let contnet = s.template_service.render_template("article/articles.html", json!({}))?;
    Ok(Html(contnet))
}

