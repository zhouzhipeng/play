use std::sync::Arc;

use axum::{Form, Router};
use axum::extract::Query;
use axum::response::Html;
use axum::routing::post;
use serde_json::json;

use shared::models::article::AddArticle;

use crate::{AppState, include_html};
use crate::controller::{ R, render_fragment, S};
use crate::controller::article::data_controller;

include_html!(ADD_NAME,ADD_CONTENT, "article/fragments/add_article_result_fragment.html");

pub fn init() -> Router<Arc<AppState>> {
    Router::new()
        .route("/fragment/article/add", post(add_article))
}


async fn add_article(s: S, Query(q): Query<AddArticle>) -> R<Html<String>> {
    let r = data_controller::add_article(s.clone(), Query(q)).await?;

    render_fragment(&s, ADD_NAME, ADD_CONTENT, json!({"success":r }))
}

