use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;

use axum::{Json, Router};
use axum::extract::State;
use axum::headers::Header;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use hyper::HeaderMap;
use serde::Serialize;
use serde_json::{json, Value};
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::{DefaultMakeSpan, TraceLayer};

use crate::AppState;

mod index_controller;
mod static_controller;
mod user_controller;
mod article_controller;
mod ws_controller;


type R<T> = Result<T, AppError>;
type S = State<Arc<AppState>>;

pub fn routers(app_state: Arc<AppState>) -> Router {
    Router::new()
        .merge(index_controller::init())
        .merge(user_controller::init())
        .merge(article_controller::init())
        .merge(ws_controller::init())
        //register your new controller here
        .with_state(app_state)
        .merge(static_controller::init())
        // logging so we can see whats going on
        .layer(TraceLayer::new_for_http().make_span_with(DefaultMakeSpan::default().include_headers(true)))
        .layer(TimeoutLayer::new(Duration::from_secs(30)))
}

// Make our own error that wraps `anyhow::Error`.
struct AppError(anyhow::Error);

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


fn should_return_json(header_map: &HeaderMap) -> bool {
    let mut return_json = false;
    if let Some(v) = header_map.get("accept") {
        if v.to_str().unwrap().contains("json") {
            return_json = true;
        }
    }
    return_json
}


fn should_return_fragment(header_map: &HeaderMap) -> bool {
    let mut return_fragment = false;
    if let Some(v) = header_map.get("HX-Request") {
        return_fragment = true;
    }
    return_fragment
}

fn render(s: S, page: &str, fragment: &str, data: Value) -> R<Response> {
    let head = s.template_service.render_template("head.html", json!({}))?;
    let top = s.template_service.render_template("top.html", json!({}))?;
    let bottom = s.template_service.render_template("bottom.html", json!({}))?;

    let content = s.template_service.render_template(fragment, data)?;

    let final_data = json!({
        "head_html": head,
        "top_html": top,
        "bottom_html": bottom,
        "content": content
    });

    let final_html = s.template_service.render_template(page, final_data)?;

    Ok(Html(final_html).into_response())
}

fn render_fragment(s: S, fragment: &str, data: Value) -> R<Response> {
    let content = s.template_service.render_template(fragment, data)?;
    Ok(Html(content).into_response())
}

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

fn auto_response<T: Serialize>(s: S, header_map: &HeaderMap, page: &str, fragment: &str, key: &str,data: &T, ) -> R<Response> {
    if should_return_json(&header_map) {
        Ok(Json(data).into_response())
    } else {
        let data = json!({
                key:data
                });
        if should_return_fragment(&header_map) {
            render_fragment(s, fragment, data)
        } else {
            render(s, page, fragment, data)
        }
    }
}