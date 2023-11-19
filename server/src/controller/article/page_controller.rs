// use std::sync::Arc;
//
// use axum::extract::Path;
// use axum::headers::HeaderMap;
// use axum::response::{Html, Response};
// use axum::{Form, Router};
// use axum::routing::{get, MethodFilter, on, post};
// use hyper::Method;
// use serde_json::json;
// use shared::models::article::{ADD_ARTICLE, AddArticle};
//
//
// use crate::AppState;
// use crate::controller::{R, render, S};
// use crate::tables::article::Article;
//
//
// pub fn init() -> Router<Arc<AppState>> {
//     Router::new()
//         .route("/page/article/add", get(add_article_page))
// }
//
// const ADD_ARTICLE_PAGE_FRAGMENT: &str = "article/fragments/add_article_page_fragment.html";
// const ADD_ARTICLE_PAGE: &str = "article/index.html";
//
//
// async fn add_article_page(s: S) -> R<Html<String>> {
//     render(&s,ADD_ARTICLE_PAGE, )
// }
//
