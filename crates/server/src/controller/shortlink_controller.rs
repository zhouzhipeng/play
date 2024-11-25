use std::sync::Arc;
use axum::body::{Body, BoxBody};

use axum::extract::OriginalUri;
use axum::response::{Html, IntoResponse, Redirect, Response};
use http::StatusCode;
use serde::Deserialize;

use crate::{app_error, AppError, AppState, S};
use crate::R;

pub fn init(state: Arc<AppState>) -> axum::Router<Arc<AppState>> {
    let mut router = axum::Router::new();
    for link in &state.config.shortlinks {
        router = router.route(&link.from, axum::routing::get(crate::controller::shortlink_controller::link));
    }

    router
}

// 定义一个用于提取路径参数的结构体
#[derive(Deserialize)]
struct ShortlinkPath {
    path: String,
}

enum MyResponse {
    Text(String),
    Redirect(Redirect),
}

impl IntoResponse for MyResponse {
    fn into_response(self) -> Response {
        match self {
            MyResponse::Text(text) => Html(text).into_response(),
            MyResponse::Redirect(redirect) => redirect.into_response(),
        }
    }
}

// #[axum::debug_handler]
async fn link( s: S,OriginalUri(uri): OriginalUri) -> R<Response> {
    let path = uri.path();
    let shortlink = s.config.shortlinks.iter().find(|c| c.from == path).ok_or::<AppError>(app_error!("404 , link not found."))?;
    // s.config.finance
    if shortlink.download {
        let resp = reqwest::get(&shortlink.to).await?;

        // 5. 构建响应
        let mut response_builder = Response::builder()
            .status(resp.status());

        // 6. 复制所有响应头
        let headers = response_builder.headers_mut().unwrap();
        for (key, value) in resp.headers() {
            headers.insert(key, value.clone());
        }

        // 7. 返回响应体
        Ok(response_builder
            .body(resp.text().await?)?.into_response())
    } else {
        Ok(MyResponse::Redirect(Redirect::temporary(&shortlink.to)).into_response())
    }
}


