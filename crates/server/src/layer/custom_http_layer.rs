use anyhow::anyhow;
use axum::body::BoxBody;
use axum::extract::{ConnectInfo, State};
use axum::middleware::Next;
use axum::response::{Html, IntoResponse};
use axum::routing::get_service;
use axum::{
    body::Body,
    http::Request,
    response::Response,
};
use cookie::Cookie;
use futures_util::future::BoxFuture;
use std::convert::Infallible;
use std::future::Future;
use std::net::SocketAddr;
use std::path::Path;
use std::pin::Pin;
use std::str::FromStr;
use std::sync::Arc;
use std::task::{Context, Poll};
use tower::{Layer, Service, ServiceExt};

use crate::config::AuthConfig;
use crate::controller::cache_controller::get_cache_content;

use crate::controller::static_controller::STATIC_DIR;
use crate::{files_dir, AppState, S};
use futures::TryStreamExt;
use http::{header, HeaderName, HeaderValue, Method, StatusCode, Uri};
use http_body::Full;
use mime_guess::mime;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::set_status::SetStatus;
use tracing::{info, warn};

pub async fn http_middleware(
    state: State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request: Request<Body>,
    next: Next<Body>,
) -> Response {
    // println!("Connection from: {}", addr);

    let is_local_request = addr.ip().to_string() == "127.0.0.1";
    // info!("is_local_request >> {}", is_local_request);

    if is_local_request{
        return next.run(request).await
    }

    let auth_config = &state.config.auth_config;

    let uri = request.uri().to_string();
    let prefix_log = format!("served request >> method: {} , url :{}",
                             request.method(), uri);

    let fingerprint = request.headers().get("X-Browser-Fingerprint");
    // info!("fingerprint is : {:?}", fingerprint);


    //serve other domains (support only static files now)
    if !auth_config.serve_domains.is_empty(){
        if let Some(header) = request.headers().get(header::HOST) {
            if let Ok(host) = header.to_str() {
                let host = host.to_string();
                if auth_config.serve_domains.contains(&host){
                    return serve_domain_folder(state.clone(), host, request).await.unwrap_or_else(|e| (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Unhandled internal error: {}", e),
                    ).into_response())
                }
            }
        }
    }

    //check fingerprint only for main domain.
    if auth_config.enabled{
        let is_whitelist = uri == "/" || {

            request.method() == &Method::GET && auth_config.whitelist.iter().any(|x| uri.starts_with(x))
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

    let response: Response = Response::builder()
        .status(StatusCode::FORBIDDEN)
        .header("content-type", "text/html")
        .body(Body::from(html)).unwrap().into_response();
    response
}

async fn handle_404(req: Request<Body>) -> impl IntoResponse {
    println!("404 Not Found: {}", req.uri());
    let uri = req.uri().path();
    let extension = Path::new(uri)
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or("");


    if extension.is_empty(){
        //return index page (dioxus will handle it)
        let index_file = files_dir!().join(req.headers().get(http::header::HOST).unwrap().to_str().unwrap()).join("index.html");
        if !index_file.exists(){
            // 这里您可以自定义404响应
            return Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("Page not found."))
                .unwrap()
        }

        Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "text/html")
            .body(Body::from(tokio::fs::read_to_string(&index_file).await.unwrap()))
            .unwrap()
    }else{
        // 这里您可以自定义404响应
        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("Page not found."))
            .unwrap()
    }

}

#[derive(Clone)]
struct NotFoundService;

impl Service<Request<Body>> for NotFoundService {
    type Response = Response;
    type Error = std::convert::Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        Box::pin(async move {
            Ok(handle_404(req).await.into_response().into())
        })
    }
}


async fn serve_domain_folder(state: S, host: String, request: Request<Body>) -> anyhow::Result<Response> {
    //check if has plugin can handle this.
    #[cfg(feature = "play-dylib-loader")]
    {
        use crate::controller::plugin_controller::inner_run_plugin;
        let plugin = state.config.plugin_config.iter().find(|plugin|!plugin.disable && plugin.proxy_domain.eq(&host));
        if let Some(plugin) = plugin{
            return Ok(inner_run_plugin(plugin, request).await.map_err(|e|anyhow!("{:?}", e))?)
        }
    }


    let full_url = Uri::from_str(&format!("https://{}{}", host, request.uri().path()))?;
    //use cache
    if let Ok(cache) = get_cache_content(&full_url).await{
        info!("use cache for host : {}", host);

        return Ok((
            [
                (
                header::CONTENT_TYPE,
                HeaderValue::from_static(mime::TEXT_HTML_UTF_8.as_ref()),
                ),
                (
                HeaderName::from_static("x-play-cache"),
                HeaderValue::from_str(&format!("{}:{}", cache.cache_key, cache.cache_time))?,
                ),
            ],
            cache.cache_content.to_string(),
        )
            .into_response())
    }

    let dir = files_dir!().join(host);
    let svc = get_service(ServeDir::new(dir).fallback(NotFoundService))
        .handle_error(|error| async move {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Unhandled internal error: {}", error),
            )
        });

    // 转发请求到 ServeDir
    Ok(svc.oneshot(request).await.into_response())
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