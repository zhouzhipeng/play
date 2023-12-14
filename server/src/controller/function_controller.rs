use std::sync::Arc;

use axum::response::Html;
use axum::{Form, Router};
use axum::routing::{get, post};
use serde::Deserialize;
use serde_json::json;
use shared::models::article::AddArticle;

use crate::{AppState, template};
use crate::controller::{HTML, render_fragment, S, Template};
use crate::controller::article::api_controller;
use crate::tables::article::Article;

pub fn init() -> Router<Arc<AppState>> {
    Router::new()
        //pages
        .route("/functions/str-joiner", post(str_joiner))
}

#[derive(Deserialize)]
struct Data{
    s : String,
}

async fn str_joiner(s: S, Form(data): Form<Data> ) -> HTML {

   render_fragment(&s, Template::DynamicTemplate{
       name: "<string>".to_string(),
       content: data.s,
   }, json!({})).await
}
