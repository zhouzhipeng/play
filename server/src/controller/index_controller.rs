use chrono::{TimeZone, Utc};
use serde_json::json;
use shared::timestamp_to_date_str;

use crate::{ method_router, template};
use crate::{HTML, R, S};

method_router!(
    get : "/"-> root,
    get : "/test-redis"-> redis_test,
    get : "/test"-> test,
);

// #[axum::debug_handler]
async fn root(s: S) -> HTML {
    let built_time = timestamp_to_date_str!(env!("BUILT_TIME").parse::<i64>()?);
    // return_error!("test");
    template!(s, "index.html", json!({
        "built_time": built_time
    }))

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


