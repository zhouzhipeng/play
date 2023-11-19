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

include_html!(f1,f2, "article/fragments/add_article_page_fragment.html");
include_html!(f3,f4, "article/fragments/articles.html");
include_html!(p1,p2, "article/index.html");
async fn add_article_page(s: S) -> R<Html<String>> {
    render_page(&s, Template { name: p1, content: p2 }, Template { name: f1, content: f2 }, json!({}))
}
async fn article_list(s: S) -> R<Html<String>> {
    let articles = Article::query_all(&s.db).await?;
    render_page(&s, Template { name: p1, content: p2 }, Template { name: f3, content: f4 }
                , json!({"articles": articles}))
}

