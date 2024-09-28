use std::collections::HashMap;
use std::env;
use crate::{return_error, S};
use crate::{method_router, AppError};
use anyhow::Context;
use axum::body::Body;
use axum::extract::Query;
use axum::Form;
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use futures_util::{StreamExt, TryStreamExt};
use http::{Request, StatusCode};
use serde::Deserialize;
use serde_json::Value;



method_router!(
    get : "/plugin/*url"-> run_plugin,
    post : "/plugin/*url"-> run_plugin,
    put : "/plugin/*url"-> run_plugin,
    delete : "/plugin/*url"-> run_plugin,
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

    use play_dylib_loader::*;

    let mut  headers = HashMap::new();
    for (name, value) in request.headers() {
        headers.insert(name.to_string(), value.to_str()?.to_string());
    }

    let method = match request.method().as_str(){
        "GET"=>HttpMethod::GET,
        "POST"=>HttpMethod::POST,
        "PUT"=>HttpMethod::PUT,
        "DELETE"=>HttpMethod::DELETE,
        _ => return_error!("unsupported http method")
    };


    let plugin_request = HttpRequest {
        method,
        headers,
        query: request.uri().query().unwrap_or_default().to_string(),
        url: url.to_string(),
        body: body_to_bytes(request.into_body()).await?,
        host_env: HostEnv { host_url: env::var("HOST")? },
    };

    let plugin_resp = load_and_run(&plugin.file_path, plugin_request).await?;

    let mut resp_builder = Response::builder()
        .status(StatusCode::from_u16(plugin_resp.status_code)?);
    for (k, v) in plugin_resp.headers {
        resp_builder = resp_builder.header(k,v);
    }

    let response: Response = resp_builder.body(Body::from(plugin_resp.body))?.into_response();
    Ok(response)
}



async fn body_to_bytes(body: Body) -> anyhow::Result<String> {
    let bytes = hyper::body::to_bytes(body).await?.to_vec();

    let body_string = String::from_utf8(bytes)?;

    Ok(body_string)
}