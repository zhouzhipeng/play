use std::sync::Arc;

use axum::{Json, Router};
use axum::extract::Path;
use axum::headers::HeaderMap;
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use serde_json::json;

use crate::AppState;
use crate::tables::article::Article;

use super::{R, render, S, should_return_json};

pub fn init() -> Router<Arc<AppState>> {
    Router::new()
        .route("/article/index", get(index))
        .route("/article/:id", get(get_article))
        .route("/article/full/:id", get(get_article_full))
}

async fn index(s: S, header_map: HeaderMap) -> R<Response> {
    let articles = Article::query_all(&s.db).await?;


    if should_return_json(header_map) {
        Ok(Json(articles).into_response())
    } else {
        let data = json!({
        "articles":articles
        });

        render(s, "article/articles.html", data)
    }
}

async fn get_article(s: S,Path(id): Path<u32>,header_map: HeaderMap ) -> R<Response>  {
    let articles = Article::query_by_id(id, &s.db).await?;

    if should_return_json(header_map){
        Ok(Json(&articles[0]).into_response())
    }else{
        let data = json!({
        "article":articles[0]
        });
        render(s, "article/article_detail.html", data)
    }
}
async fn get_article_full(s: S,Path(id): Path<u32>,header_map: HeaderMap ) -> R<Response>  {
    let articles = Article::query_by_id(id, &s.db).await?;

    if should_return_json(header_map){
        Ok(Json(&articles[0]).into_response())
    }else{
        let data = json!({
        "article":articles[0]
        });
        render(s, "article/article_detail_full.html", data)
    }
}

