use axum::extract::Path;
use axum::response::{IntoResponse, Redirect, Response};
use http::StatusCode;
use serde::Deserialize;

use crate::{app_error, AppError, method_router, S};
use crate::R;

method_router!(
    get : "/s/:path"-> link,
);

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
            MyResponse::Text(text) => (StatusCode::OK, text).into_response(),
            MyResponse::Redirect(redirect) => redirect.into_response(),
        }
    }
}

// #[axum::debug_handler]
async fn link(Path(path): Path<ShortlinkPath>, s: S) -> R<MyResponse> {
    let shortlink = s.config.shortlinks.iter().find(|c| c.from == path.path).ok_or::<AppError>(app_error!("404 , link not found."))?;
    // s.config.finance
    if !shortlink.jump {
        let res = reqwest::get(&shortlink.to).await?.text().await?;
        Ok(MyResponse::Text(res))
    } else {
        Ok(MyResponse::Redirect(Redirect::temporary(&shortlink.to)))
    }
}

