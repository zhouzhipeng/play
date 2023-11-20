use std::sync::Arc;

use axum::{Form, Json, Router};
use axum::extract::Query;
use axum::routing::{get, post};

use shared::constants::{API_ARTICLE_ADD, API_ARTICLE_LIST};
use shared::models::article::{AddArticle, QueryArticle};

use crate::AppState;
use crate::controller::{R, S};
use crate::tables::article::Article;

pub fn init() -> Router<Arc<AppState>> {
    Router::new()
        .route(API_ARTICLE_ADD, post(add_article))
        .route(API_ARTICLE_LIST, get(query_articles))
}

pub async fn add_article(s: S, Form(q): Form<AddArticle>) -> R<String> {
    let _r = Article::insert(q, &s.db).await?;
    Ok("ok".to_string())
}


pub async fn query_articles(s: S, Query(q): Query<QueryArticle>) -> R<Json<Vec<Article>>> {
    let r = Article::query(q, &s.db).await?;
    Ok(Json(r))
}

