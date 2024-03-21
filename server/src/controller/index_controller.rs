use tokio::fs::File;
use axum::body::Body;
use axum::response::{IntoResponse, Response};
use chrono::{TimeZone, Utc};
use serde_json::json;
use tokio_util::codec::{BytesCodec, FramedRead};
use shared::timestamp_to_date_str;

use crate::{ method_router, template};
use crate::{HTML, R, S};
use crate::config::get_config_path;

method_router!(
    get : "/"-> root,
    get : "/test-redis"-> redis_test,
    get : "/test"-> test,
    get : "/download-db"-> serve_db_file,
    get : "/download-config"-> serve_config_file,
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


async fn serve_db_file(s: S) -> impl IntoResponse {
    let raw = s.config.database.url.to_string();
    let path = &raw["sqlite://".len()..raw.len()];
    let file = File::open(path).await.expect("Cannot open file");
    let stream = FramedRead::new(file, BytesCodec::new());
    let body = Body::wrap_stream(stream);
    Response::new(body)
}
async fn serve_config_file(s: S) -> impl IntoResponse {
    let path = get_config_path().unwrap();
    let file = File::open(&path).await.expect("Cannot open file");
    let stream = FramedRead::new(file, BytesCodec::new());
    let body = Body::wrap_stream(stream);
    Response::new(body)
}

