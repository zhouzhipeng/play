use std::sync::Arc;

use axum::Router;
use axum::extract::Path;
use axum::headers::HeaderMap;
use axum::response::Response;
use axum::routing::get;

use crate::AppState;
use crate::tables::article::Article;

use super::{auto_response, R, S};

pub fn init() -> Router<Arc<AppState>> {
    Router::new()
        .route("/article/index", get(index))
        .route("/article/:id", get(get_article))
        .route("/article/full/:id", get(get_article_full))
}

const PAGE: &str = "article/index.html";
const ARTICLES: &str = "article/fragments/articles.html";
const ARTICLE_DETAIL: &str = "article/fragments/article_detail.html";
const ARTICLE_DETAIL_FULL: &str = "article/fragments/article_detail_full.html";


async fn index(s: S, header_map: HeaderMap) -> R<Response> {
    let articles = Article::query_all(&s.db).await?;
    auto_response(s, &header_map,&articles, PAGE, ARTICLES, "articles")

}

async fn get_article(s: S, Path(id): Path<u32>, header_map: HeaderMap) -> R<Response> {
    let articles = Article::query_by_id(id, &s.db).await?;
    auto_response(s, &header_map,&articles[0], PAGE, ARTICLE_DETAIL, "article")

}

async fn get_article_full(s: S, Path(id): Path<u32>, header_map: HeaderMap) -> R<Response> {
    let articles = Article::query_by_id(id, &s.db).await?;
    auto_response(s, &header_map,&articles[0], PAGE, ARTICLE_DETAIL_FULL, "article")
}

