use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;

use axum::{Router};
use axum::extract::State;
use axum::http::{Method, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use hyper::HeaderMap;

use serde_json::{json, Value};
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::{DefaultMakeSpan, TraceLayer};

use crate::{AppState, include_html};
use tower_http::cors::{Any, CorsLayer};
mod index_controller;
mod static_controller;
mod user_controller;
mod ws_controller;
pub mod article;


type R<T> = Result<T, AppError>;
type S = State<Arc<AppState>>;

pub fn routers(app_state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        // allow `GET` and `POST` when accessing the resource
        .allow_methods([Method::GET, Method::POST])
        // allow requests from any origin
        .allow_origin(Any);

    Router::new()
        .merge(index_controller::init())
        .merge(user_controller::init())
        .merge(article::api_controller::init())
        .merge(article::fragment_controller::init())
        .merge(article::page_controller::init())
        .merge(ws_controller::init())
        //register your new controller here
        .with_state(app_state)
        .merge(static_controller::init())
        // logging so we can see whats going on
        .layer(TraceLayer::new_for_http().make_span_with(DefaultMakeSpan::default().include_headers(true)))
        .layer(TimeoutLayer::new(Duration::from_secs(3)))
        .layer(cors)
}

// Make our own error that wraps `anyhow::Error`.
pub struct AppError(anyhow::Error);

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Server Error: {}", self.0),
        )
            .into_response()
    }
}


impl Deref for AppError {
    type Target = anyhow::Error;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// This enables using `?` on functions that return `Result<_, anyhow::Error>` to turn them into
// `Result<_, AppError>`. That way you don't need to do that manually.
impl<E> From<E> for AppError
    where
        E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

#[allow(dead_code)]
fn should_return_json(header_map: &HeaderMap) -> bool {
    let mut return_json = false;
    if let Some(v) = header_map.get("accept") {
        if v.to_str().unwrap().contains("json") {
            return_json = true;
        }
    }
    return_json
}


#[allow(dead_code)]
fn should_return_fragment(header_map: &HeaderMap) -> bool {
    let mut return_fragment = false;
    if let Some(_v) = header_map.get("HX-Request") {
        return_fragment = true;
    }
    return_fragment
}


pub struct Template{
    pub name: &'static str,
    pub content: &'static str,
}

include_html!(HEAD,HEAD_CONTENT, "head.html");
include_html!(TOP,TOP_CONTENT, "top.html");
include_html!(BOTTOM,BOTTOM_CONTENT, "bottom.html");

fn render_page(s: &S, page: Template, fragment: Template, data: Value) -> R<Html<String>> {
    let head = s.template_service.render_template(Template{ name:HEAD, content:HEAD_CONTENT}, json!({}))?;
    let top = s.template_service.render_template(Template{ name:TOP,  content:TOP_CONTENT}, json!({}))?;
    let bottom = s.template_service.render_template(Template{ name:BOTTOM, content:BOTTOM_CONTENT}, json!({}))?;

    let content = s.template_service.render_template(Template{ name:fragment.name, content:fragment.content}, data)?;

    let final_data = json!({
        "head_html": head,
        "top_html": top,
        "bottom_html": bottom,
        "content": content
    });

    let final_html = s.template_service.render_template(Template{ name:page.name,content:page.content}, final_data)?;

    Ok(Html(final_html))
}

fn render_fragment(s: &S, fragment: Template, data: Value) -> R<Html<String>> {
    let content = s.template_service.render_template(Template{ name:fragment.name, content:fragment.content}, data)?;
    Ok(Html(content))
}

#[allow(dead_code)]
fn merge_json(mut json1: Value, json2: Value) -> Value {
    if let Value::Object(ref mut obj1) = json1 {
        if let Value::Object(obj2) = json2 {
            for (key, value) in obj2 {
                obj1.insert(key.clone(), value.clone());
            }
        }
    }
    json1
}

// fn auto_response(s: S, header_map: &HeaderMap, page: &str, fragment: &str, data: Value ) -> R<Response> {
//     if should_return_json(&header_map) {
//         Ok(Json(data).into_response())
//     } else {
//         if should_return_fragment(&header_map) {
//             render_fragment(s, fragment, data)
//         } else {
//             render(s, page, fragment, data)
//         }
//     }
// }
//
//
//
