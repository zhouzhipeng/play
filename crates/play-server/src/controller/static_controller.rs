use anyhow::anyhow;
use axum::body::Body;
use axum::extract::Path;
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use include_dir::{include_dir, Dir};
use std::sync::Arc;
use tower_http::services::ServeDir;

use crate::{AppError, AppState, R, S};

pub static STATIC_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/static");

pub fn init() -> Router<Arc<AppState>> {
    #[cfg(not(feature = "debug"))]
    {
        Router::new().route("/static/{*path}", get(static_path))
    }

    #[cfg(feature = "debug")]
    Router::new().nest_service(
        "/static",
        ServeDir::new(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("static")),
    )
}

async fn static_path(s: S, Path(path): Path<String>) -> R<impl IntoResponse> {
    let path = path.trim_start_matches('/');
    let mime_type = mime_guess::from_path(path).first_or_text_plain();

    match STATIC_DIR.get_file(path) {
        None => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::empty())?),
        Some(file) => Ok(Response::builder()
            .status(StatusCode::OK)
            .header(
                header::CONTENT_TYPE,
                HeaderValue::from_str(mime_type.as_ref())?,
            )
            .header("Cross-Origin-Opener-Policy", "same-origin")
            .header("Cross-Origin-Embedder-Policy", "require-corp")
            .header("x-compress", "1")
            .body(Body::from(file.contents()))?),
    }
}
