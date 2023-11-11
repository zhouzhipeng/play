use anyhow::anyhow;
use axum::body;
use axum::body::{Empty, Full};
use axum::extract::Path;
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Router;
use axum::routing::get;
use include_dir::{Dir, include_dir};
use rustpython_vm;
use crate::controller::AppError;

static STATIC_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/static");

pub fn init() -> Router {
    Router::new().route("/static/*path", get(static_path))
}


async fn static_path(Path(path): Path<String>) -> impl IntoResponse {
    let path = path.trim_start_matches('/');
    let mime_type = mime_guess::from_path(path).first_or_text_plain();

    match STATIC_DIR.get_file(path) {
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(body::boxed(Empty::new()))
            .unwrap_or(AppError(anyhow!("unknown error.")).into_response()),
        Some(file) => Response::builder()
            .status(StatusCode::OK)
            .header(
                header::CONTENT_TYPE,
                HeaderValue::from_str(mime_type.as_ref()).unwrap_or_default(),
            )
            .body(body::boxed(Full::from(file.contents())))
            .unwrap_or(AppError(anyhow!("unknown error.")).into_response()),
    }
}
