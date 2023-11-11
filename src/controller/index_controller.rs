use std::sync::Arc;

use axum::extract::{Query, State};
use axum::response::Html;
use axum::Router;
use axum::routing::get;
use serde::Deserialize;
use serde_json::{json, json_internal};
use tracing::info;

use crate::AppState;
use crate::controller::AppError;
use crate::tables::user::{AddUser, User};

#[derive(Deserialize)]
struct Param {
    name: String,
}

pub fn init() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(root))
        .route("/test", get(htmx_test))
        .route("/hello", get(hello))
}

async fn root(name: Query<Param>, State(state): State<Arc<AppState>>) -> Result<Html<String>, AppError> {
    // py_tool::test();

    let name = name.0.name;

    let args = json!({
        "name": name,
        "age": 43,
        "male": true,
    });
    Ok(Html::from(state.template_service.render_template("test.html", args)?))
}


async fn htmx_test(name: Query<Param>, State(state): State<Arc<AppState>>) -> Result<Html<String>, AppError> {
    // py_tool::test();
    let top = state.template_service.render_template("top.html", json!({}))?;
    let bottom = state.template_service.render_template("bottom.html", json!({}))?;

    let args = json!({
        "server": "rust play server",
        "top_html": top,
        "bottom_html": bottom

    });


    let s2 = state.template_service.render_template("htmx-test.html", args)?;
    // info!("s2 = {}", s2);
    Ok(Html::from(s2))
}


async fn hello(name: Query<Param>) -> String {
    format!("hello , {}", name.0.name).to_string()
}
