use std::sync::Arc;

use axum::extract::{Query, State};
use axum::response::Html;
use axum::Router;
use axum::routing::get;
use serde::Deserialize;
use serde_json::json;
use tracing::info;

use crate::{AppState, render, render_template, TEMPLATES_DIR};

#[derive(Deserialize)]
pub struct Param {
    name: String,
}


pub async fn root(name: Query<Param>, State(state): State<Arc<AppState>>) -> Html<String> {
    // py_tool::test();


    let args = json!({
        "name": name.0.name,
        "age": 43,
        "male": true,
    });
    Html::from(render_template(state, "test.html", args))
}


pub async fn htmx_test(name: Query<Param>, State(state): State<Arc<AppState>>) -> Html<String> {
    // py_tool::test();


    let args = json!({
        "server": "rust play server",

    });

    render_template(state.clone(), "top.html", args.clone());
    render_template(state.clone(), "bottom.html", args.clone());
    let s2= render_template(state, "htmx-test.html", args);
    info!("s2 = {}", s2);
    Html::from(s2)
}


pub async fn hello(name: Query<Param>, State(state): State<Arc<AppState>>) -> String {
    format!("hello , {}", name.0.name).to_string()
}
