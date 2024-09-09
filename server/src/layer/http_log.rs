use std::convert::Infallible;
use std::net::SocketAddr;
use std::path::Path;
use axum::{
    response::Response,
    body::Body,
    http::Request,
};
use futures_util::future::BoxFuture;
use tower::{Service, Layer, ServiceExt};
use std::task::{Context, Poll};
use axum::body::{BoxBody};
use axum::response::{Html, IntoResponse};
use axum::routing::get_service;
use cookie::Cookie;

use http_body::Full;
use tracing::{info, warn};

#[derive(Clone)]
pub struct HttpLogLayer{
    pub  auth_config : AuthConfig
}

impl<S> Layer<S> for HttpLogLayer {
    type Service = HttpLogMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        HttpLogMiddleware { inner, auth_config: self.auth_config.clone() }
    }
}

#[derive(Clone)]
pub struct HttpLogMiddleware<S> {
    auth_config : AuthConfig,
    inner: S,
}

impl<S> Service<Request<Body>> for HttpLogMiddleware<S>
    where
        S: Service<Request<Body>, Response = Response> + Send + 'static,
        S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    // `BoxFuture` is a type alias for `Pin<Box<dyn Future + Send + 'a>>`
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }



    fn call(&mut self, request: Request<Body>) -> Self::Future {
        let uri = request.uri().to_string();
        let prefix_log = format!("served request >> method: {} , url :{}",
                                 request.method(), uri);

        let fingerprint = request.headers().get("X-Browser-Fingerprint");
        // info!("fingerprint is : {:?}", fingerprint);


        //serve other domains (support only static files now)
        if !self.auth_config.serve_domains.is_empty(){
            if let Some(header) = request.headers().get(axum::http::header::HOST) {
                if let Ok(host) = header.to_str() {
                    let host = host.to_string();
                    if self.auth_config.serve_domains.contains(&host){
                        return Box::pin(async move {
                            let response = serve_domain_folder(host, request).await;
                            Ok(response)
                        })
                    }
                }
            }
        }

        //check fingerprint only for main domain.
        if self.auth_config.enabled{
            let is_whitelist = uri == "/" || {

                request.method() == &Method::GET && self.auth_config.whitelist.iter().any(|x| uri.starts_with(x))
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
                                            info!("The value of browserFingerprint is: {}", fingerprint_from_cookie);
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
                    return Box::pin(async move {
                        let response = refuse_response();
                        Ok(response)

                    })
                }else{
                    //match fingerprint
                    if !self.auth_config.fingerprints.contains(&f){
                        warn!("fingerprint not match for : {}, refuse to visit  uri : {}", f,  uri);
                        //refuse
                        return Box::pin(async move {
                            let response = refuse_response();
                            Ok(response)

                        })
                    }
                }

            }
        }




        // normal requests handle
        let future = self.inner.call(request);
        Box::pin(async move {
            let response: Response = future.await?;

            let prefix_log = format!("{}, response_status_code: {} , response_headers :{:?}",
                                     prefix_log, response.status(), response.headers());
            //
            // if !response.status().is_success(){
            //     info!("error response : {} , resp_content : {}", prefix_log, response.);
            // }else{
            //     info!("success response : {}", prefix_log);
            // }
            if !(uri.to_string().starts_with("/static")
                || uri.to_string().starts_with("/files")
                || uri.to_string().starts_with("/admin")
            ){
                info!("{}", prefix_log);
            }



            Ok(response)
        })
    }
}


fn refuse_response() -> Response {
    let html = STATIC_DIR.get_file("no_permission.html").unwrap().contents_utf8().unwrap();

    let response: Response = Response::builder()
        .status(StatusCode::FORBIDDEN)
        .header("content-type", "text/html")
        .body(Body::from(html)).unwrap().into_response();
    response
}
async fn serve_domain_folder(host: String, request: Request<Body>) -> Response {
    let dir = files_dir!().join(host);
    let svc = get_service(ServeDir::new(dir))
        .handle_error(|error| async move {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Unhandled internal error: {}", error),
            )
        });

    // 转发请求到 ServeDir
    svc.oneshot(request).await.into_response()
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

use futures::TryStreamExt;
use http::{HeaderValue, Method, StatusCode};
use tower_http::services::ServeDir;
use crate::config::AuthConfig;
use crate::controller::static_controller::STATIC_DIR;
use crate::files_dir;
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