use std::ops::Deref;
use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Router;

use crate::AppState;

mod index_controller;
mod static_controller;
mod user_controller;


type R<T> = Result<T, AppError>;
type S = State<Arc<AppState>>;

pub fn routers(app_state: Arc<AppState>) -> Router {
    Router::new()
        .merge(index_controller::init())
        .merge(user_controller::init())
        .with_state(app_state)
        .merge(static_controller::init())
}

// Make our own error that wraps `anyhow::Error`.
struct AppError(anyhow::Error);

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Server Error: {}", self.0),
        )
            .into_response()
    }
}


impl Deref for AppError {
    type Target = anyhow::Error;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// This enables using `?` on functions that return `Result<_, anyhow::Error>` to turn them into
// `Result<_, AppError>`. That way you don't need to do that manually.
impl<E> From<E> for AppError
    where
        E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}
