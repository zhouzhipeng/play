use anyhow::anyhow;
use axum::extract::{ConnectInfo, State};
use axum::middleware::Next;
use axum::response::{Html, IntoResponse};
use axum::routing::get_service;
use axum::{
    body::Body,
    http::Request,
    response::Response,
};
use std::sync::Arc;
use cookie::Cookie;
use futures_util::future::BoxFuture;
use std::convert::Infallible;
use std::future::Future;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::str::FromStr;
use std::task::{Context, Poll};
use tower::{Layer, Service};
use axum::ServiceExt;

use crate::config::{AuthConfig, ProxyTarget};
use crate::controller::cache_controller::get_cache_content;

use crate::controller::static_controller::STATIC_DIR;
use crate::{files_dir, AppState, S};
use futures::TryStreamExt;
use http::{header, HeaderName, HeaderValue, Method, StatusCode, Uri};
use mime_guess::mime;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::set_status::SetStatus;
use tracing::{info, warn};
use crate::controller::files_controller;


pub async fn http_middleware(
    state: State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    // println!("Connection from: {}", addr);

    let is_local_request = addr.ip().to_string() == "127.0.0.1";
    // info!("is_local_request >> {}", is_local_request);

    if is_local_request{
        return next.run(request).await
    }

    let auth_config = &state.config.auth_config;
    let domain_proxy = &state.config.domain_proxy;

    let uri = request.uri().to_string();
    let prefix_log = format!("served request >> method: {} , url :{}",
                             request.method(), uri);

    let fingerprint = request.headers().get("X-Browser-Fingerprint");
    // info!("fingerprint is : {:?}", fingerprint);


    //serve other domains (support both static files and upstream proxy)
    if !domain_proxy.is_empty(){
        if let Some(header) = request.headers().get(axum::http::header::HOST) {
            if let Ok(host) = header.to_str() {
                let host = host.to_string();
                if let Some(domain) = domain_proxy.iter().find(|p|p.proxy_domain.eq(&host)){
                    info!("Domain proxy matched for host: {} -> {:?}", host, domain.proxy_target);
                    return match &domain.proxy_target {
                        ProxyTarget::Folder { folder_path } => {
                            serve_domain_folder(state.clone(), host, request, folder_path).await.unwrap_or_else(|e| 
                                axum::response::Response::builder()
                                    .status(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
                                    .body(format!("Unhandled internal error: {}", e).into())
                                    .unwrap()
                            )
                        }
                        ProxyTarget::Upstream { ip, port } => {
                            serve_upstream_proxy(state.clone(), host, request, ip, *port).await.unwrap_or_else(|e| 
                                axum::response::Response::builder()
                                    .status(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
                                    .body(format!("Proxy error: {}", e).into())
                                    .unwrap()
                            )
                        }
                    }
                }
            }
        }
    }

    //check fingerprint only for main domain.
    if auth_config.enabled{
        let is_whitelist = uri == "/" || {

            auth_config.whitelist.iter().any(|x| uri.starts_with(x))
        };

        if !is_whitelist{
            //begin to match fingerprint
            let f = match fingerprint {
                None => {
                    //read from cookie
                    let mut fingerprint_from_cookie="".to_string();
                    if let Some(cookie_header) = request.headers().get(axum::http::header::COOKIE) {
                        if let Ok(cookie_string) = cookie_header.to_str() {
                            for cookie_str in cookie_string.split(';').map(str::trim) {
                                if let Ok(cookie) = Cookie::parse(cookie_str) {
                                    if cookie.name() == "browserFingerprint" {
                                        fingerprint_from_cookie = cookie.value().to_string();
                                        //  info!("The value of browserFingerprint is: {}", fingerprint_from_cookie);
                                        break
                                    }
                                }
                            }
                        }
                    }

                    fingerprint_from_cookie
                }
                Some(v) => {v.to_str().unwrap_or("").to_string()}
            };

            if f.is_empty(){
                //refuse
                warn!("no fingerprint found, refuse to visit uri : {}", uri);
                return refuse_response()
            }else{
                //match fingerprint
                if !auth_config.fingerprints.contains(&f){
                    warn!("fingerprint not match for : {}, refuse to visit  uri : {}", f,  uri);
                    //refuse
                    return refuse_response();
                }
            }

        }
    }

    // normal requests handle
    next.run(request).await

}






fn refuse_response() -> Response {
    let html = STATIC_DIR.get_file("no_permission.html").unwrap().contents_utf8().unwrap();

    let response: Response = axum::response::Response::builder()
        .status(axum::http::StatusCode::FORBIDDEN)
        .header("content-type", "text/html")
        .body(html.into()).unwrap();
    response
}

async fn handle_404(uri: axum::http::Uri) -> (axum::http::StatusCode, &'static str) {
    println!("404 Not Found: {}", uri);
    let path = uri.path();
    let extension = Path::new(path)
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or("");

    if extension.is_empty() {
        (
            axum::http::StatusCode::NOT_FOUND,
            "Page not found."
        )
    } else {
        (
            axum::http::StatusCode::NOT_FOUND,
            "Page not found."
        )
    }
}

#[derive(Clone)]
struct NotFoundService;

impl<B: Send + 'static> Service<Request<B>> for NotFoundService {
    type Response = Response;
    type Error = std::convert::Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        Box::pin(async move {
            Ok(handle_404(req.uri().clone()).await.into_response())
        })
    }
}


pub async fn serve_domain_folder(state: S, host: String, request: Request<axum::body::Body>, folder_path: &str) -> anyhow::Result<Response> {
    //check if has plugin can handle this.
    #[cfg(feature = "play-dylib-loader")]
    {
        use crate::controller::plugin_controller::inner_run_plugin;
        let plugin = state.config.plugin_config.iter().find(|plugin|!plugin.disable && plugin.proxy_domain.eq(&host));
        if let Some(plugin) = plugin{
            return Ok(inner_run_plugin(plugin, request).await.map_err(|e|anyhow!("{:?}", e))?)
        }
    }


    // let full_url = Uri::from_str(&format!("https://{}{}", host, request.uri().path()))?;
    // //use cache
    // if let Ok(cache) = get_cache_content(&full_url).await{
    //     info!("use cache for host : {}", host);
    //
    //     return Ok((
    //         [
    //             (
    //             header::CONTENT_TYPE,
    //             HeaderValue::from_static(mime::TEXT_HTML_UTF_8.as_ref()),
    //             ),
    //             (
    //             HeaderName::from_static("x-play-cache"),
    //             HeaderValue::from_str(&format!("{}:{}", cache.cache_key, cache.cache_time))?,
    //             ),
    //         ],
    //         cache.cache_content.to_string(),
    //     )
    //         .into_response())
    // }

    let dir = PathBuf::from(folder_path);

    // let resp  = files_controller::download_file(axum::extract::path::Path(format!("{}{}", host,request.uri().path()))).await;
    let mut svc = get_service(ServeDir::new(dir).fallback(NotFoundService));

    // 转发请求到 ServeDir
    let uri = request.uri().path().to_string();
    let mut resp = svc.call(request).await.unwrap();

    if uri.ends_with(".wasm"){
        //let cloudflare dont compress wasm file (because ios safari has issue with it)
        let headers = resp.headers_mut();
        headers.insert(axum::http::header::CONTENT_ENCODING, axum::http::HeaderValue::from_static("identity"));
        headers.insert(axum::http::header::CACHE_CONTROL, axum::http::HeaderValue::from_static("no-transform"));

    }
    Ok(resp)

}


fn extract_prefix(url: &str) -> String {
    let path = Path::new(url);
    // 获取路径的各个组成部分（即路径中的目录和文件）
    let components = path.components().collect::<Vec<_>>();

    // 检查是否有足够的组件来提取前缀
    if components.len() > 2 {
        let  p = components[1].as_os_str().to_str().unwrap_or("");

        format!("/{}/", p)
    }else if components.len()==2{
        let mut p = components[1].as_os_str().to_str().unwrap_or("");
        if p.contains("?"){
            p = p.split("?").collect::<Vec<&str>>()[0];
        }
        format!("/{}", p)
    } else {
        url.to_string()
    }
}

pub async fn serve_upstream_proxy(
    state: S, 
    host: String, 
    mut request: Request<axum::body::Body>, 
    ip: &str, 
    port: u16
) -> anyhow::Result<Response> {
    use axum_reverse_proxy::ReverseProxy;
    use tower::Service;
    
    // 获取原始路径和查询参数
    let original_uri = request.uri().clone();
    let path_and_query = original_uri.path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or("/");
    
    // 构建目标URL
    let scheme = if port == 443 { "https" } else { "http" };
    let target_base = format!("{}://{}:{}", scheme, ip, port);
    
    // 检查是否是WebSocket升级请求
    let is_websocket = request.headers().get(axum::http::header::UPGRADE)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.to_lowercase() == "websocket")
        .unwrap_or(false);
    
    if is_websocket {
        info!("WebSocket upgrade request: {} {} -> {}", request.method(), path_and_query, target_base);
    } else {
        info!("HTTP request: {} {} -> {}", request.method(), path_and_query, target_base);
    }
    
    // 重要：设置正确的Host头部
    // 某些服务器（如Cloudflare CDN后的服务器）需要原始的Host头部
    request.headers_mut().insert(
        axum::http::header::HOST,
        axum::http::HeaderValue::from_str(&host)
            .unwrap_or_else(|_| axum::http::HeaderValue::from_static("localhost"))
    );
    
    // 对于WebSocket请求，修复Origin头部以匹配目标服务器
    if is_websocket {
        // 确保WebSocket相关头部存在（axum-reverse-proxy会自动处理，但我们记录日志）
        if let Some(connection) = request.headers().get(axum::http::header::CONNECTION) {
            info!("WebSocket Connection header: {:?}", connection);
        }
        if let Some(sec_websocket_key) = request.headers().get("sec-websocket-key") {
            info!("WebSocket Key present: {:?}", sec_websocket_key);
        }
        
        // 方案1：修复Origin头部 - 将Origin设置为目标服务器的域名
        let target_origin = format!("{}://{}", scheme, host);
        request.headers_mut().insert(
            axum::http::header::ORIGIN,
            axum::http::HeaderValue::from_str(&target_origin)
                .unwrap_or_else(|_| axum::http::HeaderValue::from_static("http://localhost"))
        );
        info!("Set WebSocket Origin to: {}", target_origin);
        
        // 方案2：如果上面的方案不工作，可以尝试移除Origin头部
        // 这让目标服务器跳过Origin检查
        // request.headers_mut().remove(axum::http::header::ORIGIN);
        // info!("Removed Origin header for WebSocket request");
        
        // 方案3：如果目标服务器需要特定的Origin，可以设置为目标服务器的实际地址
        // let backend_origin = format!("{}://{}:{}", scheme, ip, port);
        // request.headers_mut().insert(
        //     axum::http::header::ORIGIN,
        //     axum::http::HeaderValue::from_str(&backend_origin)?
        // );
        // info!("Set WebSocket Origin to backend: {}", backend_origin);
    }
    
    // 构建完整的目标URI（包含scheme、host和path）
    let full_target_uri = format!("{}{}", target_base, path_and_query);
    match full_target_uri.parse::<hyper::Uri>() {
        Ok(new_uri) => {
            info!("Setting request URI to: {}", new_uri);
            *request.uri_mut() = new_uri;
        }
        Err(e) => {
            warn!("Failed to parse target URI: {}, error: {}", full_target_uri, e);
        }
    }
    
    // 创建反向代理 - 使用空字符串作为前缀，因为我们已经设置了完整的URI
    let mut proxy = ReverseProxy::new("", &target_base);
    
    // 使用Tower Service trait调用代理
    match proxy.call(request).await {
        Ok(response) => {
            let status = response.status();
            
            if is_websocket {
                if status == StatusCode::SWITCHING_PROTOCOLS {
                    info!("WebSocket upgrade successful: {} -> {}", status, target_base);
                } else {
                    warn!("WebSocket upgrade failed: {} -> {}", status, target_base);
                }
            } else {
                if status == StatusCode::NOT_FOUND {
                    warn!("404 response for path: {} (target: {}{})", path_and_query, target_base, path_and_query);
                } else if status.is_success() {
                    info!("Success: {} for path: {}", status, path_and_query);
                } else {
                    warn!("Non-success status: {} for path: {}", status, path_and_query);
                }
            }
            
            Ok(response)
        }
        Err(e) => {
            if is_websocket {
                warn!("WebSocket proxy error to {}: {:?}", target_base, e);
            } else {
                warn!("HTTP proxy error to {}: {:?}", target_base, e);
            }
            
            // 返回502 Bad Gateway错误
            Ok(axum::response::Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(format!("Proxy error: {:?}", e).into())?)
        }
    }
}


// 提供 `try_concat` 方法来转换 body

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.

    use super::*;

    #[test]
    fn test_parse() -> anyhow::Result<()> {
        let u = extract_prefix("/functions/aa");

        println!("{}", u);
        Ok(())
    }
}