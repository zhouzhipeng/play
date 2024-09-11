use crate::{files_dir, method_router, HTML, S};
use axum::body::HttpBody;
use axum::extract::Query;
use axum::response::Html;
use serde::Deserialize;
use tokio::task::JoinHandle;
use tracing::{error, info};

method_router!(
    get : "/cache/html" -> cache_html,
);

#[derive(Deserialize)]
struct Param {
    url: String,
}

#[cfg(feature = "play_cache")]
async fn cache_html(s: S, Query(param): Query<Param>) -> HTML {
    tokio::spawn(async move {
        async fn inner(url: &str)->anyhow::Result<()> {
            let html = play_cache::render_html_in_browser(url).await?;

            //save to file
            let cache_file_name = rust_utils::md5(url);
            let save_path  = files_dir!().join("__cache__").join(&cache_file_name);



            info!("html >> {:?}", html);
            Ok(())
        }

        if let Err(e) = inner(&param.url).await{
            error!("cache_html error: {:?}", e);
        }

    });

    Ok(Html("ok".to_string()))
}


#[cfg(not(feature = "play_cache"))]
async fn cache_html(s: S, Query(param): Query<Param>) -> HTML {
    Ok(Html("play_cache feature is disabled.".to_owned()))
}