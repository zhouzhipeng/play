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
        //pages
        .route("/page/article/add", get(add_article_page))
        .route("/page/article/list/v2", get(article_list_v2))
        //fragments
        .route("/fragment/article/add", post(add_article))
}

async fn add_article_page(s: S) -> HTML {
    template!(s, "article/index.html", "article/fragments/add_article_page_fragment.html",json!({}) )
}

async fn article_list_v2(s: S) -> HTML {
    let articles = Article::query_all(&s.db).await?;
    template!(s, "article/index.html", "article/fragments/articles.html",json!({"articles": articles, "name": "zzp"}) )
}


async fn add_article(s: S, Form(q): Form<AddArticle>) -> HTML {
    let r = api_controller::add_article(s.clone(), Form(q)).await?;
    template!(s,"article/fragments/add_article_result_fragment.html", json!({"success":*r }) )
}



