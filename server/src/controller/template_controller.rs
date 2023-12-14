use std::sync::Arc;

use axum::response::Html;
use axum::{Form, Router};
use axum::routing::{get, post};
use serde_json::json;
use shared::models::article::AddArticle;

use crate::{AppState, template};
use crate::controller::{HTML, S};
use crate::controller::article::api_controller;
use crate::tables::article::Article;

pub fn init() -> Router<Arc<AppState>> {
    Router::new()
        .route("/page/str-joiner", get(str_joiner))
        .route("/page/py-runner", get(py_runner))
}

async fn str_joiner(s: S) -> HTML {
    template!(s, "str-joiner.html",json!({}) )
}
async fn py_runner(s: S) -> HTML {
    template!(s, "py-runner.html",json!({}) )
}
