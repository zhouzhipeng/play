use std::sync::Arc;

use axum::{Form, Router};

use axum::response::Html;
use axum::routing::post;
use serde_json::json;

use shared::models::article::AddArticle;

use crate::{AppState, include_html};
use crate::controller::{R, render_fragment, S, Template};
use crate::controller::article::api_controller;

include_html!(ADD_NAME,ADD_CONTENT, "article/fragments/add_article_result_fragment.html");

pub fn init() -> Router<Arc<AppState>> {
    Router::new()
        .route("/fragment/article/add", post(add_article))
}


async fn add_article(s: S, Form(q): Form<AddArticle>) -> R<Html<String>> {
    let r = api_controller::add_article(s.clone(), Form(q)).await?;

    render_fragment(&s, Template { name: ADD_NAME, content: ADD_CONTENT }, json!({"success":r }))
}

