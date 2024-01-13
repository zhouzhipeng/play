// use axum::{
//     body::BoxBody,
//     response::{Response, IntoResponse},
//     http::{Request, StatusCode},
// };
// use tower::{Service, ServiceExt, BoxError, layer::Layer};
// use futures_util::future::BoxFuture;
// use std::task::{Context, Poll};
// use std::sync::Arc;
// use std::time::Duration;
// use moka::future::Cache;
//
// use tracing::info;
//
// // 自定义中间件
// struct HtmlCacheLayer {
//     cache: Cache<String, String>,
// }
//
// impl<S> Layer<S> for HtmlCacheLayer {
//     type Service = HtmlCacheService<S>;
//
//     fn layer(&self, inner: S) -> Self::Service {
//         HtmlCacheService {
//             inner,
//             cache: self.cache.clone(),
//         }
//     }
// }
//
// struct HtmlCacheService<S> {
//     inner: S,
//     cache: Cache<String, String>,
// }
//
// impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for HtmlCacheService<S>
//     where
//         S: Service<Request<ReqBody>, Response = Response<ResBody>> + Clone + Send + 'static,
//         S::Future: Send + 'static,
//         ReqBody: Send + 'static,
//         ResBody: http_body::Body + Send + 'static,
//         ResBody::Data: Send,
//         ResBody::Error: Into<BoxError>,
// {
//     type Response = S::Response;
//     type Error = S::Error;
//     type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;
//
//     fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
//         self.inner.poll_ready(cx)
//     }
//
//     fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
//         let mut inner = self.inner.clone();
//
//         Box::pin(async move {
//             let uri_key = req.uri().path();
//             if self.cache.contains_key(uri_key){
//                 info!("using cache to return html content >>  url : {}", uri_key);
//
//             }
//             let response: Response = inner.call(req).await?;
//
//             // 检查响应的 Content-Type
//             if response.headers().get(http::header::CONTENT_TYPE)
//                 .map(|value| value.to_str().unwrap_or_default().starts_with("text/html"))
//                 .unwrap_or(false)
//             {
//                 // 应用缓存逻辑
//                 // 这里需要添加具体的缓存处理代码
//                 self.cache.insert(uri_key, response.body)
//
//
//
//             }
//
//             Ok(response)
//         })
//     }
// }
//
