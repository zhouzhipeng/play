use std::collections::HashMap;
use crate::S;
use crate::{method_router, AppError};
use anyhow::Context;
use axum::body::Body;
use axum::extract::Query;
use axum::response::{IntoResponse, Response};
use http::{Request, StatusCode};
use serde::Deserialize;


#[cfg(feature = "play-dylib-loader")]
method_router!(
    get : "/plugin/*url"-> run_plugin,
);


#[cfg(feature = "play-dylib-loader")]
async fn run_plugin(s: S, request: Request<Body>) -> Result<Response, AppError> {
    let url = request.uri().path();
    let plugin = s.config.plugin_config.iter()
        .find(|plugin|url.starts_with(&plugin.url_prefix)).context("plugin for found for url!")?;

    //todo : pass headers to plugin system
    // request.headers();
    use play_dylib_loader::*;
    let params: Query<HashMap<String, String>> = Query::try_from_uri(request.uri())?;

    let plugin_request = HttpRequest {
        headers: Default::default(),
        query: params.0,
        url: url.to_string(),
        body: "".to_string(),
    };

    let plugin_resp = load_and_run(&plugin.file_path, plugin_request).await?;

    let resp_builder = Response::builder()
        .status(StatusCode::from_u16(plugin_resp.status_code)?);
    //todo: pass through headers
    let response: Response =
        resp_builder.body(Body::from(plugin_resp.body))?.into_response();
    Ok(response)
}