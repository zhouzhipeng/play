use crate::{method_router, return_error, template};
use crate::{HTML, S};
use axum::body::Body;
use axum::extract::Query;
use axum::response::Html;
use http::Request;
use serde::Deserialize;
use serde_json::json;

method_router!(
    get : "/test2"-> test,
);

#[derive(Deserialize)]
struct Param {
    path: String,
}

async fn test(s: S, Query(param): Query<Param>) -> HTML {
    // #[cfg(feature = "play-dylib-loader")]
    // {
    //     use play_dylib_loader::*;
    //
    //     let request = HttpRequest {
    //         method: HttpMethod::GET,
    //         headers: Default::default(),
    //         query: Default::default(),
    //         body: "sdfd".to_string(),
    //         url: "sdf".to_string(),
    //         context: HostContext { host_url: "".to_string(), plugin_prefix_url: "".to_string() },
    //     };
    //     let resp = load_and_run(&param.path, request).await?;
    //     return Ok(Html(resp.body.to_string()))
    // }

    return_error!("fuck33 ");
    // template!(s, "test.html", json!({"name":"zzp"}))
}
