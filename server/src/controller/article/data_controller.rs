use std::sync::Arc;

use axum::{Form, Json, Router};
use axum::extract::Query;
use axum::routing::{get, post};

use shared::models::article::{AddArticle, QueryArticle};

use crate::AppState;
use crate::controller::{R, S};
use crate::tables::article::Article;
use crate::tables::DBQueryResult;

pub fn init() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/article/add", post(add_article))
        .route("/api/article/list", get(query_articles))
}

pub async fn add_article(s: S, Query(q): Query<AddArticle>) -> R<String> {
    let r = Article::insert(q, &s.db).await?;
    Ok("ok".to_string())
}


pub  async fn query_articles(s: S, Query(q): Query<QueryArticle>) -> R<Json<Vec<Article>>> {
    let r = Article::query(q, &s.db).await?;
    Ok(Json(r))
}

