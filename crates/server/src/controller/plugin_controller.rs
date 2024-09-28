use std::collections::HashMap;
use std::env;
use crate::S;
use crate::{method_router, AppError};
use anyhow::Context;
use axum::body::Body;
use axum::extract::Query;
use axum::response::{IntoResponse, Response};
use http::{Request, StatusCode};
use serde::Deserialize;
use serde_json::Value;



method_router!(
    get : "/plugin/*url"-> run_plugin,
);

#[cfg(not(feature = "play-dylib-loader"))]
async fn run_plugin(s: S, request: Request<Body>) -> Result<Response, AppError> {
    crate::return_error!("play-dylib-loader feature not enabled!")
}
#[cfg(feature = "play-dylib-loader")]
async fn run_plugin(s: S, request: Request<Body>) -> Result<Response, AppError> {
    let url = request.uri().path();
    let plugin = s.config.plugin_config.iter()
        .find(|plugin|url.starts_with(&plugin.url_prefix)).context("plugin for found for url!")?;

    //todo : pass headers to plugin system
    // request.headers();
    use play_dylib_loader::*;

    let plugin_request = HttpRequest {
        headers: Default::default(),
        query: request.uri().query().unwrap_or_default().to_string(),
        url: url.to_string(),
        body: "".to_string(),
        host_env: HostEnv { host_url: env::var("HOST")? },
    };

    let plugin_resp = load_and_run(&plugin.file_path, plugin_request).await?;

    let mut resp_builder = Response::builder()
        .status(StatusCode::from_u16(plugin_resp.status_code)?);
    for (k, v) in plugin_resp.headers {
        resp_builder = resp_builder.header(k,v);
    }

    let response: Response =
        resp_builder.body(Body::from(plugin_resp.body))?.into_response();
    Ok(response)
}