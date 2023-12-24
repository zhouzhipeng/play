use serde_json::json;

use crate::{method_router, template};
use crate::{HTML, R, S};

method_router!(
    get : "/"-> root,
    get : "/test-redis"-> redis_test,
    get : "/test"-> test,
);

// #[axum::debug_handler]
async fn root(s: S) -> HTML {
    // return_error!("test");
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

