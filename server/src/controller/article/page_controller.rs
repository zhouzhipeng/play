use std::sync::Arc;

use axum::response::Html;
use axum::Router;
use axum::routing::get;
use serde_json::json;

use crate::{AppState, init_template};
use crate::controller::{R, render_page, render_page_v2, S, Template};
use crate::tables::article::Article;

pub fn init() -> Router<Arc<AppState>> {
    Router::new()
        .route("/page/article/add", get(add_article_page))
        .route("/page/article/list", get(article_list))
        .route("/page/article/list/v2", get(article_list_v2))
}

const ADD: Template = init_template!("article/fragments/add_article_page_fragment.html");
const ARTICLES : Template = init_template!( "article/fragments/articles.html");
const INDEX: Template = init_template!( "article/index.html");
async fn add_article_page(s: S) -> R<Html<String>> {
    render_page(&s, INDEX, ADD, json!({}))
}
async fn article_list(s: S) -> R<Html<String>> {
    let articles = Article::query_all(&s.db).await?;
    render_page(&s, INDEX, ARTICLES
                , json!({"articles": articles}))
}
async fn article_list_v2(s: S) -> R<Html<String>> {
    let articles = Article::query_all(&s.db).await?;
    render_page_v2(&s, INDEX, ARTICLES
                , json!({"articles": articles})).await
}

