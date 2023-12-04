use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;

use axum::{Json, Router};
use axum::extract::State;
use axum::http::{Method, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use hyper::HeaderMap;
use serde::Serialize;

use serde_json::{json, Value};
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::{DefaultMakeSpan, TraceLayer};

use crate::{AppState, init_template};
use tower_http::cors::{Any, CorsLayer};
mod index_controller;
mod static_controller;
mod user_controller;
mod ws_controller;
pub mod article;


type R<T> = Result<T, AppError>;
type S = State<Arc<AppState>>;

type HTML = Result<Html<String>, AppError>;
type JSON<T> = Result<Json<T>, AppError>;



#[derive(Serialize)]
pub struct Success{

}


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
        E:Into<anyhow::Error>,
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

use crate::file_path;
#[macro_export]
macro_rules! init_template {
    ($fragment: expr) => {
        crate::controller::Template { name: $fragment, content: include_str!(crate::file_path!(concat!("/templates/",  $fragment))) }
    };
}

#[macro_export]
macro_rules! template {
    ($s: ident, $fragment: expr, $json: expr) => {
        {
            let t = crate::init_template!($fragment);
            let content: Html<String> = crate::controller::render_fragment(&$s,t,  $json).await?;
            Ok(content)
        }

    };
    ($s: ident, $page: expr, $fragment: expr, $json:expr) => {
        {
            let page = crate::init_template!($page);
            let frag = crate::init_template!($fragment);
            let content: Html<String> = crate::controller::render_page_v2(&$s,page,frag, $json).await?;
            Ok(content)
        }

    };
}


const HEAD: Template = init_template!("head.html");
const TOP : Template = init_template!("top.html");
const BOTTOM: Template = init_template!("bottom.html");


async fn render_page_v2(s: &S, page: Template, fragment: Template, data: Value) -> R<Html<String>> {
    let head = s.template_service.render_template(HEAD, json!({})).await?;
    let top = s.template_service.render_template(TOP, json!({})).await?;
    let bottom = s.template_service.render_template(BOTTOM, json!({})).await?;

    let content = s.template_service.render_template(Template{ name:fragment.name, content:fragment.content}, data).await?;

    let final_data = json!({
        "head_html": head,
        "top_html": top,
        "bottom_html": bottom,
        "content": content
    });

    let final_html = s.template_service.render_template(Template{ name:page.name,content:page.content}, final_data).await?;

    Ok(Html(final_html))
}

async fn render_fragment(s: &S, fragment: Template, data: Value) -> R<Html<String>> {
    let content = s.template_service.render_template(Template{ name:fragment.name, content:fragment.content}, data).await?;
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
