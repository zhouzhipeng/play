use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::Html;
use axum::Router;
use axum::routing::get;
use serde_json::json;

use crate::AppState;
use crate::controller::{R, S};
use crate::tables::article::{Article, QueryArticle};

pub fn init() -> Router<Arc<AppState>> {
    Router::new()
        .route("/article/index", get(index))
        .route("/article/:id", get(get_article))
}

async fn index(State(s): S) -> R<Html<String>> {
    let articles = Article::query_all( &s.db).await?;
    let data = json!({
        "articles":articles
    });
    let contnet = s.template_service.render_template("article/articles.html", data)?;
    Ok(Html(contnet))
}
async fn get_article(Path(id) : Path<u32>,State(s): S) -> R<Html<String>> {
    let articles = Article::query_by_id( id,&s.db).await?;
    let data = json!({
        "article":articles[0]
    });
    let contnet = s.template_service.render_template("article/article_detail.html", data)?;
    Ok(Html(contnet))
}

