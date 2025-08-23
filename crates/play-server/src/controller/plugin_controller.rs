use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::Duration;
use crate::{return_error, AppState, S};
use crate::AppError;
use crate::controller::admin_controller::{PLUGIN_REQUEST_STORE, PLUGIN_RESPONSE_STORE};
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
            router = router.route(&format!("{}/{{*url}}", plugin.url_prefix),
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
#[axum::debug_handler]
async fn run_plugin(s: axum::extract::State<Arc<AppState>>, request: Request<Body>) -> Response {
    (
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        "play-dylib-loader feature not enabled!"
    ).into_response()
}

fn remove_trailing_slash(uri: &str) -> String {
    if uri.ends_with('/') && uri.len() >= 1 {
        uri[..uri.len() - 1].to_string()
    } else {
        uri.to_string()
    }
}
#[cfg(feature = "play-dylib-loader")]
#[axum::debug_handler]
async fn run_plugin(s: axum::extract::State<Arc<AppState>>, request: Request<Body>) -> Response {
    let url = request.uri().path();
    let url = remove_trailing_slash(url);
    
    match s.config.plugin_config.iter().find(|plugin|{
        !plugin.disable &&
        !plugin.url_prefix.is_empty() &&
        (url.eq(&plugin.url_prefix) ||  url.starts_with(&format!("{}/", plugin.url_prefix)))
    }) {
        Some(plugin) => match inner_run_plugin(plugin, request).await {
            Ok(response) => response,
            Err(e) => (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Error: {}", e)
            ).into_response()
        },
        None => (
            axum::http::StatusCode::NOT_FOUND,
            "Plugin not found for URL"
        ).into_response()
    }
}

// New function to handle the coordinated request/response flow
#[cfg(feature = "play-dylib-loader")]
async fn load_and_run_coordinated(dylib_path: &str, request_id: i64) -> Result<play_dylib_loader::HttpResponse, AppError> {
    use play_dylib_loader::HttpResponse;
    
    let dylib_path = dylib_path.to_string();
    
    // Call the plugin
    tokio::spawn(async move {
        // Load and call the plugin with request_id
        if let Err(e) = unsafe { call_plugin_with_id(&dylib_path, request_id) } {
            eprintln!("Error calling plugin: {:?}", e);
        }
    });
    
    // Wait for response with timeout
    let timeout = Duration::from_secs(30);
    let start = std::time::Instant::now();
    
    loop {
        // Check if response is available
        if let Some(response) = PLUGIN_RESPONSE_STORE.remove(&request_id) {
            // Clean up request from store
            PLUGIN_REQUEST_STORE.remove(&request_id);
            return Ok(response.1);
        }
        
        if start.elapsed() > timeout {
            // Clean up request from store
            PLUGIN_REQUEST_STORE.remove(&request_id);
            return Err(AppError::from(anyhow::anyhow!("Plugin response timeout")));
        }
        
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

#[cfg(feature = "play-dylib-loader")]
unsafe fn call_plugin_with_id(dylib_path: &str, request_id: i64) -> anyhow::Result<()> {
    use libloading::{Library, Symbol};
    use play_dylib_loader::{HandleRequestFn, HANDLE_REQUEST_FN_NAME};
    use anyhow::Context;
    
    let lib = Library::new(dylib_path).context("Failed to load plugin library")?;
    let handle_request: Symbol<HandleRequestFn> = lib.get(HANDLE_REQUEST_FN_NAME.as_ref())
        .context("`handle_request` method not found")?;
    handle_request(request_id);
    Ok(())
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
            config_text: if plugin.need_config_file{Some(read_config_file(true).await?)}else{None},
        },

    };

    // Generate unique request ID
    static REQUEST_ID_COUNTER: AtomicI64 = AtomicI64::new(0);
    let request_id = REQUEST_ID_COUNTER.fetch_add(1, Ordering::SeqCst);
    
    // Store the request in the shared store
    PLUGIN_REQUEST_STORE.insert(request_id, plugin_request);
    
    // Call the plugin with just the request_id
    let plugin_resp = load_and_run_coordinated(&plugin.file_path, request_id).await?;

    let mut resp_builder = Response::builder()
        .status(StatusCode::from_u16(plugin_resp.status_code)?);
    for (k, v) in plugin_resp.headers {
        resp_builder = resp_builder.header(k,v);
    }

    let response: Response = resp_builder.body(Body::from(plugin_resp.body))?.into_response();
    Ok(response)
}



async fn body_to_bytes(body: Body) -> anyhow::Result<String> {
    use http_body_util::BodyExt;
    
    let bytes = body.collect().await?.to_bytes().to_vec();
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