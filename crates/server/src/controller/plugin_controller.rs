use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use crate::{return_error, AppState, S};
use crate::AppError;
use anyhow::Context;
use axum::body::Body;
use axum::response::{IntoResponse, Response};
use futures_util::{StreamExt, TryStreamExt};
use http::{Request, StatusCode};
use serde::Deserialize;
use play_shared::constants::DATA_DIR;
use crate::config::{read_config_file, PluginConfig};

// method_router!(
//     get : "/plugin/*url"-> run_plugin,
//     post : "/plugin/*url"-> run_plugin,
//     put : "/plugin/*url"-> run_plugin,
//     delete : "/plugin/*url"-> run_plugin,
// );

pub fn init(state: Arc<AppState>) -> axum::Router<Arc<AppState>> {
    let mut router = axum::Router::new();
    for plugin in &state.config.plugin_config {
        if !plugin.url_prefix.is_empty(){
            router = router.route(&format!("{}/*url", plugin.url_prefix),
                  axum::routing::get(run_plugin)
                      .post(run_plugin)
                      .put(run_plugin)
                      .delete(run_plugin)
            );
            router = router.route(&format!("{}", plugin.url_prefix),
                  axum::routing::get(run_plugin)
                      .post(run_plugin)
                      .put(run_plugin)
                      .delete(run_plugin)
            );
        }
    }

    router
}

#[cfg(not(feature = "play-dylib-loader"))]
async fn run_plugin(s: S, request: Request<Body>) -> Result<Response, AppError> {
    crate::return_error!("play-dylib-loader feature not enabled!")
}

fn remove_trailing_slash(uri: &str) -> String {
    if uri.ends_with('/') && uri.len() >= 1 {
        uri[..uri.len() - 1].to_string()
    } else {
        uri.to_string()
    }
}
#[cfg(feature = "play-dylib-loader")]
async fn run_plugin(s: S, request: Request<Body>) -> Result<Response, AppError> {
    let url = request.uri().path();
    let url = remove_trailing_slash(url);
    let plugin = s.config.plugin_config.iter()
        .find(|plugin|{
            !plugin.disable &&
            !plugin.url_prefix.is_empty() &&
            (url.eq(&plugin.url_prefix) ||  url.starts_with(&format!("{}/", plugin.url_prefix)))
        }).context("plugin for found for url!")?;

    inner_run_plugin(plugin, request).await

}

#[cfg(feature = "play-dylib-loader")]
pub async fn inner_run_plugin( plugin: &PluginConfig, request: Request<Body>)->Result<Response, AppError>{
    use play_dylib_loader::HostContext;
    use play_dylib_loader::*;

    let url = request.uri().path();
    let url = remove_trailing_slash(url);

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
        context: HostContext {
            host_url: env::var("HOST")?,
            plugin_prefix_url: plugin.url_prefix.to_string(),
            data_dir: env::var(DATA_DIR)?,
            config_text: if plugin.need_config_file{Some(read_config_file()?)}else{None},
        },

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

#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn test_convert(){

        println!("Back to Rust String: {:?}", remove_trailing_slash("/sdfsdf/sf/"));
    }

}