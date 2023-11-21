use std::sync::Arc;


use axum::response::Html;
use axum::Router;
use axum::routing::get;
use crate::service::redis::RedisOperation;

use crate::AppState;
use crate::controller::{R, S};


pub fn init() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(root))
        .route("/test-redis", get(redis_test))
}

async fn root() -> R<Html<&'static str>> {
   Ok(Html("ok."))
}


async fn redis_test(s: S) -> R<String> {
    s.redis_service.set("testkey", "testval").await?;
    let val = s.redis_service.get( "testkey").await?;

    Ok(val)
}

