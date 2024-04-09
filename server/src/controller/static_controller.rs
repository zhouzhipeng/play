use std::sync::Arc;
use anyhow::anyhow;
use axum::body;
use axum::body::{Empty, Full};
use axum::extract::Path;
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Router;
use axum::routing::get;
use include_dir::{Dir, include_dir};
use tower_http::services::ServeDir;

use crate::{AppError, AppState, R, S};

pub static STATIC_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/static");

pub fn init() -> Router<Arc<AppState>> {
    #[cfg(not(feature = "debug"))]
    {Router::new().route("/static/*path", get(static_path))}

    #[cfg(feature = "debug")]
    Router::new().nest_service("/static", ServeDir::new(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("static")))
}


async fn static_path(s: S, Path(path): Path<String>) -> R<impl IntoResponse> {
    let path = path.trim_start_matches('/');
    let mime_type = mime_guess::from_path(path).first_or_text_plain();


    match STATIC_DIR.get_file(path) {
        None => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(body::boxed(Empty::new()))?),
        Some(file) => Ok(Response::builder()
            .status(StatusCode::OK)
            .header(
                header::CONTENT_TYPE,
                HeaderValue::from_str(mime_type.as_ref())?
            ).body(body::boxed(Full::from(file.contents())))?),
    }
}
