use std::sync::Arc;

use axum::response::Html;
use axum::Router;
use axum::routing::get;
use serde_json::json;

use crate::{AppState, template};
use crate::controller::{HTML, S};
use crate::tables::article::Article;

pub fn init() -> Router<Arc<AppState>> {
    Router::new()
        .route("/page/article/add", get(add_article_page))
        .route("/page/article/list/v2", get(article_list_v2))
}

async fn add_article_page(s: S) -> HTML {
    template!(s, "article/index.html", "article/fragments/add_article_page_fragment.html",json!({}) )
}

async fn article_list_v2(s: S) -> HTML {
    let articles = Article::query_all(&s.db).await?;
    template!(s, "article/index.html", "article/fragments/articles.html",json!({"articles": articles, "name": "zzp"}) )
}

