use std::sync::Arc;

use axum::response::Html;
use axum::Router;
use axum::routing::get;
use serde_json::json;

use crate::{AppState, include_html};
use crate::controller::{R, render_page, S, Template};
use crate::tables::article::Article;

pub fn init() -> Router<Arc<AppState>> {
    Router::new()
        .route("/page/article/add", get(add_article_page))
        .route("/page/article/list", get(article_list))
}

include_html!(F1,F2, "article/fragments/add_article_page_fragment.html");
include_html!(F3,F4, "article/fragments/articles.html");
include_html!(P1,P2, "article/index.html");
async fn add_article_page(s: S) -> R<Html<String>> {
    render_page(&s, Template { name: P1, content: P2 }, Template { name: F1, content: F2 }, json!({}))
}
async fn article_list(s: S) -> R<Html<String>> {
    let articles = Article::query_all(&s.db).await?;
    render_page(&s, Template { name: P1, content: P2 }, Template { name: F3, content: F4 }
                , json!({"articles": articles}))
}

