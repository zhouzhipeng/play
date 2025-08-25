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

use crate::config::{AuthConfig, ProxyTarget, WebSocketConfig, DomainProxy, OriginStrategy};
use crate::controller::cache_controller::get_cache_content;

use crate::controller::static_controller::STATIC_DIR;

use crate::{files_dir, AppState, S};

// 自定义证书验证器，用于忽略所有证书验证（仅开发环境使用）
#[derive(Debug)]
struct NoVerifier;

impl rustls::client::danger::ServerCertVerifier for NoVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls_pki_types::CertificateDer<'_>,
        _intermediates: &[rustls_pki_types::CertificateDer<'_>],
        _server_name: &rustls_pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls_pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }
    
    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls_pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }
    
    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls_pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }
    
    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA1,
            rustls::SignatureScheme::ECDSA_SHA1_Legacy,
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
            rustls::SignatureScheme::ECDSA_NISTP521_SHA512,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
            rustls::SignatureScheme::ED25519,
            rustls::SignatureScheme::ED448,
        ]
    }
}
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

    let remote_ip =  addr.ip().to_string();
    let is_local_request = remote_ip == "::ffff:127.0.0.1";
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
                            serve_upstream_proxy_with_config(state.clone(), host, request, ip, *port, domain).await.unwrap_or_else(|e| 
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

// 带配置的代理函数
pub async fn serve_upstream_proxy_with_config(
    state: S, 
    host: String, 
    mut request: Request<axum::body::Body>, 
    ip: &str, 
    port: u16,
    domain_config: &DomainProxy
) -> anyhow::Result<Response> {
    use axum_reverse_proxy::ReverseProxy;
    use tower::Service;
    
    // 获取原始路径和查询参数
    let original_uri = request.uri().clone();
    let path_and_query = original_uri.path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or("/");
    
    // 根据DomainProxy配置确定schema
    let scheme = match domain_config.use_https {
        Some(true) => "https",
        Some(false) => "http", 
        None => if port == 443 { "https" } else { "http" } // 默认逻辑
    };
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
    
    // 对于WebSocket请求，根据配置处理Origin头部
    if is_websocket {
        handle_websocket_origin(&mut request, &host, &scheme, ip, port, &domain_config.websocket_config)?;
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
    
    // 创建反向代理
    let mut proxy = if domain_config.ignore_cert && scheme == "https" {
        warn!("Creating proxy with certificate verification DISABLED for {}", target_base);
        match create_ignore_cert_client() {
            Ok(custom_client) => {
                ReverseProxy::new_with_client("", &target_base, custom_client).with_ignore_cert(true)
            }
            Err(e) => {
                warn!("Failed to create ignore cert client: {}, using standard proxy", e);
                ReverseProxy::new("", &target_base)
            }
        }
    } else {
        ReverseProxy::new("", &target_base)
    };
    
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

// 根据配置处理WebSocket Origin头部
fn handle_websocket_origin(
    request: &mut Request<axum::body::Body>,
    host: &str,
    scheme: &str,
    ip: &str,
    port: u16,
    websocket_config: &WebSocketConfig
) -> anyhow::Result<()> {
    info!("Handling WebSocket Origin with strategy: {:?}", websocket_config.origin_strategy);
    
    match &websocket_config.origin_strategy {
        OriginStrategy::Keep => {
            // 保持原始Origin不变
            if let Some(origin) = request.headers().get(axum::http::header::ORIGIN) {
                info!("Keeping original Origin: {:?}", origin);
            } else {
                info!("No original Origin header to keep");
            }
        }
        OriginStrategy::Remove => {
            // 移除Origin头部
            request.headers_mut().remove(axum::http::header::ORIGIN);
            info!("Removed Origin header for WebSocket request");
        }
        OriginStrategy::Host => {
            // 设置为代理域名
            let host_origin = format!("{}://{}", scheme, host);
            request.headers_mut().insert(
                axum::http::header::ORIGIN,
                axum::http::HeaderValue::from_str(&host_origin)?
            );
            info!("Set WebSocket Origin to host: {}", host_origin);
        }
        OriginStrategy::Backend => {
            // 设置为后端服务器地址
            let backend_origin = format!("{}://{}:{}", scheme, ip, port);
            request.headers_mut().insert(
                axum::http::header::ORIGIN,
                axum::http::HeaderValue::from_str(&backend_origin)?
            );
            info!("Set WebSocket Origin to backend: {}", backend_origin);
        }
        OriginStrategy::Custom => {
            // 使用自定义Origin
            if let Some(custom_origin) = &websocket_config.custom_origin {
                request.headers_mut().insert(
                    axum::http::header::ORIGIN,
                    axum::http::HeaderValue::from_str(custom_origin)?
                );
                info!("Set WebSocket Origin to custom: {}", custom_origin);
            } else {
                warn!("Custom origin strategy selected but no custom_origin provided, falling back to backend");
                let backend_origin = format!("{}://{}:{}", scheme, ip, port);
                request.headers_mut().insert(
                    axum::http::header::ORIGIN,
                    axum::http::HeaderValue::from_str(&backend_origin)?
                );
                info!("Set WebSocket Origin to backend (fallback): {}", backend_origin);
            }
        }
    }
    
    Ok(())
}

// 自定义代理处理函数，用于忽略HTTPS证书验证
async fn handle_custom_proxy_with_ignore_cert(
    mut request: Request<axum::body::Body>,
    target_uri: &str,
    domain_config: &DomainProxy,
    host: &str,
    scheme: &str,
    ip: &str,
    port: u16
) -> anyhow::Result<Response> {
    use axum::body::{to_bytes, Body};
    use http::HeaderMap;
    
    // 检查是否是WebSocket升级请求
    let is_websocket = request.headers().get(axum::http::header::UPGRADE)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.to_lowercase() == "websocket")
        .unwrap_or(false);
    
    if is_websocket {
        info!("WebSocket upgrade request with ignored certificates: {}", target_uri);
        
        // 使用本地 fork 的 axum-reverse-proxy，应该已经修复了 TLS 编译问题
        // 重新设置URI
        match target_uri.parse::<hyper::Uri>() {
            Ok(new_uri) => {
                *request.uri_mut() = new_uri;
            }
            Err(e) => {
                warn!("Failed to parse target URI for WebSocket: {}, error: {}", target_uri, e);
                return Ok(axum::response::Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body("Invalid target URI".into())?);
            }
        }
        
        let target_base = format!("{}://{}:{}", scheme, ip, port);
        
        // 尝试使用自定义客户端（忽略证书验证）
        match create_ignore_cert_client() {
            Ok(custom_client) => {
                info!("WebSocket: Using custom client with certificate verification DISABLED for {}", target_base);
                let mut proxy = axum_reverse_proxy::ReverseProxy::new_with_client("", &target_base, custom_client)
                    .with_ignore_cert(true);
                use tower::Service;
                
                return match proxy.call(request).await {
                    Ok(response) => {
                        let status = response.status();
                        if status == StatusCode::SWITCHING_PROTOCOLS {
                            info!("WebSocket upgrade successful (certificate verification DISABLED): {}", target_base);
                        } else {
                            warn!("WebSocket upgrade failed: {} -> {}", status, target_base);
                        }
                        Ok(response)
                    }
                    Err(e) => {
                        warn!("WebSocket proxy error with custom client: {:?}", e);
                        Ok(axum::response::Response::builder()
                            .status(StatusCode::BAD_GATEWAY)
                            .body(format!("WebSocket proxy error: {:?}", e).into())?)
                    }
                };
            }
            Err(e) => {
                warn!("Failed to create custom client for WebSocket, falling back to standard proxy: {}", e);
                // 回退到标准代理
                let mut proxy = axum_reverse_proxy::ReverseProxy::new("", &target_base);
                use tower::Service;
                
                return match proxy.call(request).await {
                    Ok(response) => {
                        let status = response.status();
                        if status == StatusCode::SWITCHING_PROTOCOLS {
                            warn!("WebSocket upgrade successful (with STANDARD certificate verification): {}", target_base);
                        } else {
                            warn!("WebSocket upgrade failed: {} -> {}", status, target_base);
                        }
                        Ok(response)
                    }
                    Err(e) => {
                        warn!("WebSocket proxy error with standard client: {:?}", e);
                        Ok(axum::response::Response::builder()
                            .status(StatusCode::BAD_GATEWAY)
                            .body(format!("WebSocket proxy error: {:?}", e).into())?)
                    }
                };
            }
        }
    }
    
    info!("HTTP request with ignored certificates: {}", target_uri);
    
    // 创建忽略证书验证的reqwest客户端
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .danger_accept_invalid_hostnames(true)
        .build()
        .map_err(|e| anyhow!("Failed to create client: {}", e))?;
    
    // 转换请求方法
    let method = match request.method() {
        &http::Method::GET => reqwest::Method::GET,
        &http::Method::POST => reqwest::Method::POST,
        &http::Method::PUT => reqwest::Method::PUT,
        &http::Method::DELETE => reqwest::Method::DELETE,
        &http::Method::HEAD => reqwest::Method::HEAD,
        &http::Method::OPTIONS => reqwest::Method::OPTIONS,
        &http::Method::PATCH => reqwest::Method::PATCH,
        other => {
            warn!("Unsupported HTTP method: {}", other);
            return Ok(axum::response::Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .body("Unsupported HTTP method".into())?);
        }
    };
    
    // 获取请求体
    let body = std::mem::replace(request.body_mut(), Body::empty());
    let body_bytes = to_bytes(body, usize::MAX).await
        .map_err(|e| anyhow!("Failed to read request body: {}", e))?;
    
    // 构建reqwest请求
    let mut req_builder = client
        .request(method, target_uri)
        .body(body_bytes.to_vec());
    
    // 复制请求头，跳过某些不应该转发的头
    let skip_headers = ["host", "content-length", "connection"];
    for (name, value) in request.headers().iter() {
        let name_str = name.as_str().to_lowercase();
        if !skip_headers.contains(&name_str.as_str()) {
            if let Ok(value_str) = value.to_str() {
                req_builder = req_builder.header(name.as_str(), value_str);
            }
        }
    }
    
    // 发送请求
    let response = req_builder
        .send()
        .await
        .map_err(|e| anyhow!("Proxy request failed: {}", e))?;
        
    // 构建响应
    let mut response_builder = axum::response::Response::builder()
        .status(response.status().as_u16());
    
    // 复制响应头，跳过某些不应该转发的头
    let skip_response_headers = ["content-length", "connection", "transfer-encoding"];
    for (name, value) in response.headers().iter() {
        let name_str = name.as_str().to_lowercase();
        if !skip_response_headers.contains(&name_str.as_str()) {
            response_builder = response_builder.header(name.as_str(), value.as_bytes());
        }
    }
    
    // 获取响应体
    let response_bytes = response.bytes().await
        .map_err(|e| anyhow!("Failed to read response body: {}", e))?;
    
    Ok(response_builder
        .body(Body::from(response_bytes.to_vec()))?)
}

// 创建忽略证书验证的HTTP客户端
fn create_ignore_cert_client() -> Result<hyper_util::client::legacy::Client<hyper_rustls::HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>, axum::body::Body>, Box<dyn std::error::Error + Send + Sync>> {
    use std::sync::Arc;
    use rustls::ClientConfig;
    use hyper_rustls::HttpsConnectorBuilder;
    use hyper_util::client::legacy::Client;
    use hyper_util::rt::TokioExecutor;
    
    // 创建忽略证书验证的 rustls 配置
    let config = ClientConfig::builder()
        .dangerous() // 启用危险配置
        .with_custom_certificate_verifier(Arc::new(NoVerifier))
        .with_no_client_auth();
    
    // 创建 HTTPS 连接器
    let https_connector = HttpsConnectorBuilder::new()
        .with_tls_config(config)
        .https_or_http()
        .enable_http1()
        .build();
    
    // 创建客户端
    let client = Client::builder(TokioExecutor::new())
        .build(https_connector);
        
    Ok(client)
}


// 兼容性函数：不带配置的代理函数（向后兼容）
pub async fn serve_upstream_proxy(
    state: S, 
    host: String, 
    request: Request<axum::body::Body>, 
    ip: &str, 
    port: u16
) -> anyhow::Result<Response> {
    // 使用默认配置
    let default_domain_config = DomainProxy {
        proxy_domain: host.clone(),
        proxy_target: ProxyTarget::Upstream { 
            ip: ip.to_string(), 
            port 
        },
        use_https: None,
        ignore_cert: false,
        websocket_config: WebSocketConfig::default(),
    };
    serve_upstream_proxy_with_config(state, host, request, ip, port, &default_domain_config).await
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