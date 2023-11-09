use std::sync::Arc;

use axum::extract::{Query, State};
use axum::Router;
use axum::routing::get;
use serde::Deserialize;
use serde_json::json;

use crate::{AppState, render, TEST_HTML};

#[derive(Deserialize)]
pub struct Param {
    name: String,
}


pub async fn root(name: Query<Param>, State(state): State<Arc<AppState>>) -> String {
    // py_tool::test();


    let args = json!({
        "name": name.0.name,
        "age": 43,
        "male": true,
    });
    render(state, TEST_HTML.to_string(), args)
}
