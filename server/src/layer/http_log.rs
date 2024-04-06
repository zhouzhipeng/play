use std::convert::Infallible;
use axum::{
    response::Response,
    body::Body,
    http::Request,
};
use futures_util::future::BoxFuture;
use tower::{Service, Layer};
use std::task::{Context, Poll};
use axum::body::{BoxBody};

use http_body::Full;
use tracing::info;

#[derive(Clone)]
pub struct HttpLogLayer;

impl<S> Layer<S> for HttpLogLayer {
    type Service = HttpLogMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        HttpLogMiddleware { inner }
    }
}

#[derive(Clone)]
pub struct HttpLogMiddleware<S> {
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
        let prefix_log = format!("served request >> method: {} , url :{} , headers : {:?}",
                                 request.method(), request.uri(), request.headers());

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

            info!("{}", prefix_log);

            Ok(response)
        })
    }
}


use futures::TryStreamExt; // 提供 `try_concat` 方法来转换 body

