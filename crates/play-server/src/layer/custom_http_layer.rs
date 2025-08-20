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
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{Duration, timeout};
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

// TCP连接池用于复用连接
#[derive(Clone)]
pub struct TcpConnectionPool {
    connections: Arc<RwLock<HashMap<String, Arc<Mutex<TcpStream>>>>>,
}

impl TcpConnectionPool {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn get_or_create_connection(&self, target: &str) -> anyhow::Result<Arc<Mutex<TcpStream>>> {
        // 先尝试获取现有连接
        {
            let connections = self.connections.read().await;
            if let Some(conn) = connections.get(target) {
                // 检查连接是否仍然有效
                if let Ok(stream) = conn.try_lock() {
                    // 简单检查连接状态
                    match stream.peer_addr() {
                        Ok(addr) => {
                            info!("Reusing existing connection to {} (peer: {})", target, addr);
                            return Ok(conn.clone());
                        },
                        Err(e) => {
                            warn!("Existing connection to {} is invalid: {}", target, e);
                        }
                    }
                } else {
                    warn!("Existing connection to {} is locked, creating new connection", target);
                }
            }
        }

        // 创建新连接
        info!("Creating new TCP connection to {}", target);
        let stream = TcpStream::connect(target).await
            .map_err(|e| anyhow!("Failed to establish TCP connection to {}: {}", target, e))?;
        
        let peer_addr = stream.peer_addr()
            .map_err(|e| anyhow!("Failed to get peer address for {}: {}", target, e))?;
        
        info!("Successfully established TCP connection to {} (peer: {})", target, peer_addr);
        let conn = Arc::new(Mutex::new(stream));
        
        // 存储连接
        {
            let mut connections = self.connections.write().await;
            connections.insert(target.to_string(), conn.clone());
            info!("Stored connection to {} in pool (total connections: {})", target, connections.len());
        }

        Ok(conn)
    }

    async fn remove_connection(&self, target: &str) {
        let mut connections = self.connections.write().await;
        if connections.remove(target).is_some() {
            warn!("Removed failed connection to {} from pool (remaining connections: {})", target, connections.len());
        }
    }
}

// 全局连接池
use std::sync::OnceLock;
static TCP_POOL: OnceLock<TcpConnectionPool> = OnceLock::new();

fn get_tcp_pool() -> &'static TcpConnectionPool {
    TCP_POOL.get_or_init(|| TcpConnectionPool::new())
}

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


async fn serve_domain_folder(state: S, host: String, request: Request<axum::body::Body>, folder_path: &str) -> anyhow::Result<Response> {
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

async fn serve_upstream_proxy(
    state: S, 
    host: String, 
    request: Request<axum::body::Body>, 
    ip: &str, 
    port: u16
) -> anyhow::Result<Response> {
    let target = format!("{}:{}", ip, port);
    
    // 从连接池获取或创建连接
    let conn = match get_tcp_pool().get_or_create_connection(&target).await {
        Ok(conn) => {
            info!("Successfully connected to upstream {}", target);
            conn
        },
        Err(e) => {
            warn!("Failed to connect to upstream {} - Error: {:?}", target, e);
            return Ok(axum::response::Response::builder()
                .status(axum::http::StatusCode::BAD_GATEWAY)
                .body(format!("Connection failed to {}: {}", target, e).into())?);
        }
    };

    // 构建原始HTTP请求（在消费request之前）
    let method = request.method().clone();
    let uri = request.uri().clone();
    let version = request.version();
    let headers = request.headers().clone();
    
    // 提取请求体
    let body_bytes = axum::body::to_bytes(request.into_body(), usize::MAX).await?;
    
    // 构建原始HTTP请求
    let raw_request = build_raw_http_request_from_parts(&method, &uri, version, &headers, &host, body_bytes.len())?;

    // 通过TCP隧道发送请求并接收响应
    info!("Sending request to {} via TCP tunnel: {} {}", target, method, uri);
    match tcp_tunnel_request(conn.clone(), &raw_request, &body_bytes).await {
        Ok(response_data) => {
            info!("Received response from {} ({} bytes)", target, response_data.len());
            // 解析响应
            match parse_http_response(&response_data) {
                Ok(response) => {
                    info!("Successfully parsed response from {}", target);
                    Ok(response)
                },
                Err(e) => {
                    warn!("Failed to parse HTTP response from {} - Error: {:?}, Raw data length: {}", target, e, response_data.len());
                    // 记录响应数据的前100字节用于调试
                    let debug_data = if response_data.len() > 100 {
                        &response_data[..100]
                    } else {
                        &response_data
                    };
                    warn!("Response data preview: {:?}", String::from_utf8_lossy(debug_data));
                    
                    // 移除无效连接
                    get_tcp_pool().remove_connection(&target).await;
                    Ok(axum::response::Response::builder()
                        .status(axum::http::StatusCode::BAD_GATEWAY)
                        .body(format!("Invalid HTTP response from {}: {}", target, e).into())?)
                }
            }
        }
        Err(e) => {
            warn!("TCP tunnel communication error with {} - Error: {:?}", target, e);
            // 移除失效连接
            get_tcp_pool().remove_connection(&target).await;
            Ok(axum::response::Response::builder()
                .status(axum::http::StatusCode::BAD_GATEWAY)
                .body(format!("TCP tunnel error to {}: {}", target, e).into())?)
        }
    }
}

fn build_raw_http_request_from_parts(
    method: &http::Method,
    uri: &http::Uri,
    version: http::Version,
    headers: &http::HeaderMap,
    host: &str,
    content_length: usize,
) -> anyhow::Result<String> {
    // 构建请求行
    let mut raw_request = format!("{} {} {:?}\r\n", method, uri, version);
    
    // 添加头部
    for (name, value) in headers.iter() {
        if let Ok(value_str) = value.to_str() {
            raw_request.push_str(&format!("{}: {}\r\n", name, value_str));
        }
    }
    
    // 确保有Host头部
    if !headers.contains_key("host") {
        raw_request.push_str(&format!("Host: {}\r\n", host));
    }
    
    // 添加Connection: keep-alive以支持连接复用
    if !headers.contains_key("connection") {
        raw_request.push_str("Connection: keep-alive\r\n");
    }
    
    // 添加Content-Length头部（如果有body且没有Content-Length头）
    if content_length > 0 && !headers.contains_key("content-length") {
        raw_request.push_str(&format!("Content-Length: {}\r\n", content_length));
    }
    
    raw_request.push_str("\r\n");
    Ok(raw_request)
}

async fn tcp_tunnel_request(
    conn: Arc<Mutex<TcpStream>>, 
    raw_request: &str, 
    body: &[u8]
) -> anyhow::Result<Vec<u8>> {
    let mut stream = conn.lock().await;
    
    // 发送请求
    info!("Sending HTTP request headers ({} bytes)", raw_request.len());
    stream.write_all(raw_request.as_bytes()).await
        .map_err(|e| anyhow!("Failed to write request headers: {}", e))?;
    
    if !body.is_empty() {
        info!("Sending HTTP request body ({} bytes)", body.len());
        stream.write_all(body).await
            .map_err(|e| anyhow!("Failed to write request body: {}", e))?;
    }
    
    stream.flush().await
        .map_err(|e| anyhow!("Failed to flush request to upstream: {}", e))?;
    
    // 读取响应 - 使用超时避免无限等待
    info!("Reading response from upstream...");
    let mut response_buffer = Vec::new();
    let read_result = timeout(Duration::from_secs(30), async {
        // 先读取响应头
        let mut header_buffer = Vec::new();
        let mut temp_buffer = [0; 1];
        let mut consecutive_crlf = 0;
        
        info!("Reading HTTP response headers...");
        loop {
            match stream.read_exact(&mut temp_buffer).await {
                Ok(_) => {
                    header_buffer.push(temp_buffer[0]);
                    
                    // 检查是否到达头部结尾 (\r\n\r\n)
                    if temp_buffer[0] == b'\r' || temp_buffer[0] == b'\n' {
                        consecutive_crlf += 1;
                        if consecutive_crlf >= 4 && header_buffer.ends_with(b"\r\n\r\n") {
                            break;
                        }
                    } else {
                        consecutive_crlf = 0;
                    }
                },
                Err(e) => {
                    return Err(anyhow!("Failed to read response headers: {}", e));
                }
            }
        }
        
        info!("Read response headers ({} bytes)", header_buffer.len());
        response_buffer.extend_from_slice(&header_buffer);
        
        // 解析Content-Length或使用分块编码
        let header_str = String::from_utf8_lossy(&header_buffer);
        if let Some(content_length) = extract_content_length(&header_str) {
            info!("Reading response body with Content-Length: {}", content_length);
            // 读取固定长度的body
            let mut body_buffer = vec![0; content_length];
            match stream.read_exact(&mut body_buffer).await {
                Ok(_) => {
                    info!("Successfully read response body ({} bytes)", content_length);
                    response_buffer.extend_from_slice(&body_buffer);
                },
                Err(e) => {
                    return Err(anyhow!("Failed to read response body (Content-Length: {}): {}", content_length, e));
                }
            }
        } else if header_str.contains("Transfer-Encoding: chunked") {
            info!("Reading chunked response body...");
            match read_chunked_body(&mut *stream, &mut response_buffer).await {
                Ok(_) => info!("Successfully read chunked response body"),
                Err(e) => return Err(anyhow!("Failed to read chunked response body: {}", e)),
            }
        } else {
            info!("No Content-Length or chunked encoding found, response complete");
        }
        
        Ok::<Vec<u8>, anyhow::Error>(response_buffer)
    }).await;
    
    match read_result {
        Ok(Ok(data)) => {
            info!("Successfully completed TCP tunnel request ({} bytes total)", data.len());
            Ok(data)
        },
        Ok(Err(e)) => {
            warn!("TCP tunnel read error: {:?}", e);
            Err(e)
        },
        Err(_) => {
            warn!("TCP tunnel request timeout (30s)");
            Err(anyhow!("Request timeout after 30 seconds"))
        },
    }
}

fn extract_content_length(headers: &str) -> Option<usize> {
    for line in headers.lines() {
        if line.to_lowercase().starts_with("content-length:") {
            if let Some(length_str) = line.split(':').nth(1) {
                return length_str.trim().parse().ok();
            }
        }
    }
    None
}

async fn read_chunked_body(stream: &mut TcpStream, buffer: &mut Vec<u8>) -> anyhow::Result<()> {
    loop {
        // 读取chunk大小行
        let mut size_line = Vec::new();
        let mut temp_buffer = [0; 1];
        
        loop {
            stream.read_exact(&mut temp_buffer).await?;
            if temp_buffer[0] == b'\r' {
                stream.read_exact(&mut temp_buffer).await?; // 读取\n
                if temp_buffer[0] == b'\n' {
                    break;
                }
            }
            size_line.push(temp_buffer[0]);
        }
        
        let size_str = String::from_utf8_lossy(&size_line);
        let chunk_size = usize::from_str_radix(size_str.trim(), 16)?;
        
        buffer.extend_from_slice(&size_line);
        buffer.extend_from_slice(b"\r\n");
        
        if chunk_size == 0 {
            // 最后一个chunk，读取可能的尾部头部
            stream.read_exact(&mut temp_buffer).await?; // \r
            stream.read_exact(&mut temp_buffer).await?; // \n
            buffer.extend_from_slice(b"\r\n");
            break;
        }
        
        // 读取chunk数据
        let mut chunk_data = vec![0; chunk_size];
        stream.read_exact(&mut chunk_data).await?;
        buffer.extend_from_slice(&chunk_data);
        
        // 读取chunk结尾的\r\n
        stream.read_exact(&mut temp_buffer).await?; // \r
        stream.read_exact(&mut temp_buffer).await?; // \n
        buffer.extend_from_slice(b"\r\n");
    }
    
    Ok(())
}

fn parse_http_response(data: &[u8]) -> anyhow::Result<Response> {
    // 查找头部结尾
    let headers_end = data
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| anyhow!("Invalid HTTP response: no header end found"))?;
    
    let headers_bytes = &data[..headers_end];
    let body_bytes = &data[headers_end + 4..];
    
    // 解析头部
    let headers_str = String::from_utf8_lossy(headers_bytes);
    let mut lines = headers_str.lines();
    
    // 解析状态行
    let status_line = lines.next().ok_or_else(|| anyhow!("No status line"))?;
    let status_code = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse::<u16>().ok())
        .ok_or_else(|| anyhow!("Invalid status code"))?;
    
    let mut response_builder = axum::response::Response::builder()
        .status(axum::http::StatusCode::from_u16(status_code)?);
    
    // 解析并添加头部
    for line in lines {
        if let Some(colon_pos) = line.find(':') {
            let name = line[..colon_pos].trim();
            let value = line[colon_pos + 1..].trim();
            
            if let (Ok(header_name), Ok(header_value)) = (
                axum::http::HeaderName::from_str(name),
                axum::http::HeaderValue::from_str(value)
            ) {
                response_builder = response_builder.header(header_name, header_value);
            }
        }
    }
    
    Ok(response_builder.body(Body::from(body_bytes.to_vec()))?)
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