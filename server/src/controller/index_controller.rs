use std::sync::Arc;


use axum::response::Html;
use axum::Router;
use axum::routing::get;


use crate::AppState;
use crate::controller::{R};

//fixme: register 'init' method in mod.rs
pub fn init() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(root))
        // .route("/test", get(htmx_test))
}

async fn root() -> R<Html<&'static str>> {
   Ok(Html("ok."))
}

//
// async fn htmx_test( State(state): S) -> R<Html<String>> {
//     // py_tool::test();
//     let top = state.template_service.render_template("top.html", json!({}))?;
//     let bottom = state.template_service.render_template("bottom.html", json!({}))?;
//
//     let args = json!({
//         "server": "rust play server99",
//         "top_html": top,
//         "bottom_html": bottom
//
//     });
//
//
//     let s2 = state.template_service.render_template("htmx-test.html", args)?;
//     // info!("s2 = {}", s2);
//     Ok(Html::from(s2))
// }


