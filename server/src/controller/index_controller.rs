use std::sync::Arc;

use axum::response::Html;
use axum::Router;
use axum::routing::get;
use serde_json::json;

use crate::{AppState, init_template, r_template};
use crate::controller::{HTML, R, render_fragment, S};

pub fn init() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(root))
        .route("/test-redis", get(redis_test))
        .route("/test", get(test))
}

async fn root() -> R<Html<&'static str>> {
    Ok(Html("ok."))
}


// #[debug_handler]
async fn redis_test(s: S) -> R<String> {
    s.redis_service.set("testkey", "testval").await?;
    let val = s.redis_service.get("testkey").await?;

    s.redis_service.publish("a", "test123").await?;

    Ok(val)
    // Ok("sdf".to_string())
}

async fn test(s: S) -> HTML {
    r_template!(s, "test.html", {"name":"zzp"})
}

