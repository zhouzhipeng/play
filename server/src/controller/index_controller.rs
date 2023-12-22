use std::sync::Arc;

use axum::response::Html;
use axum::Router;
use axum::routing::get;
use serde_json::json;

use shared::constants::API_EXECUTE_SQL;

use crate::{AppState, template};
use crate::{HTML, R, S};
use crate::tables::article::Article;

pub fn init() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(root))
        .route("/test-redis", get(redis_test))
        .route("/test", get(test))
}
// #[axum::debug_handler]
async fn root(s: S) -> HTML {
    template!(s, "index.html", json!({}))
}


// #[debug_handler]
async fn redis_test(s: S) -> R<String> {
    s.redis_service.set("testkey", "testval").await?;
    let val = s.redis_service.get("testkey").await?;

    // s.redis_service.unwrap().publish("a", "test123").await?;

    Ok(val)
    // Ok("sdf".to_string())
}

async fn test(s: S) -> HTML {
    // template!(s, "test.html", json!({"name":"zzp"}))
    template!(s, "test.html", json!({"name":"zzp"}))
}

