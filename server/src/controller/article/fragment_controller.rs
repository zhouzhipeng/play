use std::sync::Arc;

use axum::{Form, Router};
use axum::response::Html;
use axum::routing::post;
use serde_json::json;

use shared::models::article::AddArticle;

use crate::{AppState, template};
use crate::controller::{HTML, R, S};
use crate::controller::article::api_controller;

pub fn init() -> Router<Arc<AppState>> {
    Router::new()
        .route("/fragment/article/add", post(add_article))
}


async fn add_article(s: S, Form(q): Form<AddArticle>) -> HTML {
    let r = api_controller::add_article(s.clone(), Form(q)).await?;
    template!(s,"article/fragments/add_article_result_fragment.html", json!({"success":*r }) )
}

