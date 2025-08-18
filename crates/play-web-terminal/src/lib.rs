pub mod error;
pub mod local_terminal;
pub mod server;
pub mod websocket;

pub use error::{Error, Result};

use axum::Router;

pub fn create_router<S>() -> Router<S> 
where
    S: Clone + Send + Sync + 'static,
{
    server::create_router()
}