use std::convert::Infallible;
use std::path::Path;
use axum::{
    response::Response,
    body::Body,
    http::Request,
};
use futures_util::future::BoxFuture;
use tower::{Service, Layer};
use std::task::{Context, Poll};
use axum::body::{BoxBody};
use axum::response::IntoResponse;
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
        let prefix_log = format!("served request >> method: {} , url :{} , headers : {:?}",
                                 request.method(), uri, request.headers());

        let fingerprint = request.headers().get("X-Browser-Fingerprint");
        // info!("fingerprint is : {:?}", fingerprint);

        //check fingerprint
        if self.auth_config.enabled{
            let uri_prefix = extract_prefix(&uri);
            if self.auth_config.whitelist.contains(&uri_prefix){
                // info!("whitelist uri : {}, skip checking fingerprint.", uri);
            }else{
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
    let response: Response = Response::builder()
        .status(StatusCode::FORBIDDEN)
        .header("content-type", "text/plain")
        .body(Body::from("NO PERMISSION! (please contact admin)")).unwrap().into_response();
    response
}


fn extract_prefix(url: &str) -> String {
    let path = Path::new(url);
    // 获取路径的各个组成部分（即路径中的目录和文件）
    let components = path.components().collect::<Vec<_>>();

    // 检查是否有足够的组件来提取前缀
    if components.len() > 1 {


        format!("/{}/", components[1].as_os_str().to_str().unwrap_or(""))
    } else {
        url.to_string()
    }
}

use futures::TryStreamExt;
use http::{HeaderValue, StatusCode};
use crate::config::AuthConfig; // 提供 `try_concat` 方法来转换 body

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