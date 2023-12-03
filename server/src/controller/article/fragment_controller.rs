use std::sync::Arc;

use axum::{Form, Router};

use axum::response::Html;
use axum::routing::post;
use serde_json::json;

use shared::models::article::AddArticle;

use crate::{AppState, init_template};
use crate::controller::{R, render_fragment, S, Template};
use crate::controller::article::api_controller;

const result : Template = init_template!("article/fragments/add_article_result_fragment.html");

pub fn init() -> Router<Arc<AppState>> {
    Router::new()
        .route("/fragment/article/add", post(add_article))
}


async fn add_article(s: S, Form(q): Form<AddArticle>) -> R<Html<String>> {
    let r = api_controller::add_article(s.clone(), Form(q)).await?;

    render_fragment(&s, result, json!({"success":r })).await
}

