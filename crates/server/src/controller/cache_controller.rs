use std::time::Duration;
use anyhow::ensure;
use crate::{files_dir, method_router, HTML, S};
use axum::body::HttpBody;
use axum::extract::Query;
use axum::Form;
use axum::response::Html;
use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;
use tracing::{error, info};

method_router!(
    get : "/cache/html" -> cache_html,
    post : "/cache/save" -> save_cache,
);

#[derive(Deserialize)]
struct CacheRequestParam {
    url: String,
    upload_to_url: String,
    header: String,
}
#[derive(Serialize, Deserialize)]
struct SaveCacheParam {
    cache_key: String,
    cache_content: String,
}

async fn save_cache(s: S, Form(param): Form<SaveCacheParam>) -> HTML {
    //save to file
    let cache_file_name = rust_utils::md5(&param.cache_key);
    let cache_dir = files_dir!().join("__cache__");

    if !cache_dir.exists(){
        tokio::fs::create_dir(&cache_dir).await?;
    }

    let save_path  = cache_dir.join(&cache_file_name);
    tokio::fs::write(save_path, &param.cache_content).await?;
    Ok(Html("Ok.".to_string()))
}

#[cfg(feature = "play_cache")]
async fn cache_html(s: S, Query(param): Query<CacheRequestParam>) -> HTML {
    tokio::spawn(async move {
        async fn inner(url: &str, upload_to_url: &str, header: &str)->anyhow::Result<()> {
            let html = play_cache::render_html_in_browser(url).await?;

            //upload to remote server
            let vals: Vec<&str> = header.split("=").collect();
            ensure!(vals.len()==2, "header is configured wrong!");

            let resp = reqwest::Client::builder().timeout(Duration::from_secs(10)).build()?.post(upload_to_url)
                .header(vals[0], vals[1])
                .form(&SaveCacheParam { cache_key: url.to_string(), cache_content: html.to_string() })
                .send().await?;
            info!("cache result upload to : {} , resp: {:?}", upload_to_url,resp);


            info!("html >> {:?}", html);
            Ok(())
        }

        if let Err(e) = inner(&param.url, &param.upload_to_url, &param.header).await{
            error!("cache_html error: {:?}", e);
        }

    });

    Ok(Html("ok".to_string()))
}


#[cfg(not(feature = "play_cache"))]
async fn cache_html(s: S, Query(param): Query<CacheRequestParam>) -> HTML {
    Ok(Html("play_cache feature is disabled.".to_owned()))
}