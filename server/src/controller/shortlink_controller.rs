use std::sync::Arc;
use axum::body::{Body, BoxBody};

use axum::extract::OriginalUri;
use axum::response::{Html, IntoResponse, Redirect, Response};
use http::StatusCode;
use serde::Deserialize;

use crate::{app_error, AppError, AppState, S};
use crate::R;

pub fn init(state: Arc<AppState>) -> axum::Router<std::sync::Arc<crate::AppState>> {
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
async fn link( s: S,OriginalUri(uri): OriginalUri) -> R<MyResponse> {
    let path = uri.path();
    let shortlink = s.config.shortlinks.iter().find(|c| c.from == path).ok_or::<AppError>(app_error!("404 , link not found."))?;
    // s.config.finance
    if shortlink.download {
        let res = reqwest::get(&shortlink.to).await?.text().await?;
        Ok(MyResponse::Text(res))
    } else {
        Ok(MyResponse::Redirect(Redirect::temporary(&shortlink.to)))
    }
}


