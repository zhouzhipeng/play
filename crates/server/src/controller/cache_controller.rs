use std::time::Duration;
use anyhow::{bail, ensure};
use crate::{files_dir, get_file_modify_time, method_router, HTML, S};
use axum::body::HttpBody;
use axum::extract::Query;
use axum::Form;
use axum::response::Html;
use http::Uri;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::fs;
use tokio::task::JoinHandle;
use tracing::{error, info};

method_router!(
    get : "/cache/html" -> cache_html,
    post : "/cache/save" -> save_cache,
    post : "/cache/delete" -> delete_cache,
);

#[derive(Deserialize)]
struct CacheRequestParam {
    url: String,
    save_cache_url: String,
    delete_cache_url: String,
    header: String,
}
#[derive(Serialize, Deserialize)]
struct SaveCacheParam {
    url: String,
    cache_content: String,
}
#[derive(Serialize, Deserialize)]
struct DeleteCacheParam {
    url: String,
}

pub fn generate_cache_key(uri: &Uri) -> String {
    let mut s = format!("{}://{}{}", uri.scheme_str().unwrap_or_default(), uri.host().unwrap_or_default(), uri.path());
    if s.ends_with("/") {
        s = s[0..s.len() - 1].to_string();
    }

    s = rust_utils::md5(&s);
    s
}

const CACHE_FOLDER: &'static str = "__cache__";

pub struct CacheContent{
    pub cache_key : String,
    pub cache_content : String,
    pub cache_time: i64,
}

pub async fn get_cache_content(uri: &Uri) -> anyhow::Result<CacheContent> {
    let cache_file_name = generate_cache_key(uri);
    let cache_path = files_dir!().join(CACHE_FOLDER).join(&cache_file_name);
    if cache_path.exists() {
        let content = fs::read_to_string(&cache_path).await?;

        let cache_time = get_file_modify_time(&cache_path).await;
        Ok(CacheContent{
            cache_key: cache_file_name,
            cache_content: content,
            cache_time,
        })
    } else {
        bail!("cache not found for uri : {}", uri)
    }
}

async fn save_cache(Form(param): Form<SaveCacheParam>) -> HTML {
    //save to file
    let cache_file_name = generate_cache_key(&param.url.parse()?);
    let cache_dir = files_dir!().join(CACHE_FOLDER);

    if !cache_dir.exists() {
        tokio::fs::create_dir(&cache_dir).await?;
    }

    let save_path = cache_dir.join(&cache_file_name);
    tokio::fs::write(save_path, &param.cache_content).await?;
    Ok(Html("Ok.".to_string()))
}
async fn delete_cache(Form(param): Form<DeleteCacheParam>) -> HTML {
    //save to file
    let cache_file_name = generate_cache_key(&param.url.parse()?);
    let file_path = files_dir!().join(CACHE_FOLDER).join(&cache_file_name);

    if file_path.exists() {
        tokio::fs::remove_file(&file_path).await?;
    }

    Ok(Html("Ok.".to_string()))
}
#[cfg(feature = "play_cache")]
async fn update_cache_in_remote(s: S,param: &CacheRequestParam) -> anyhow::Result<()> {
    //upload to remote server
    let headers: Vec<&str> = param.header.split("=").collect();
    ensure!(headers.len()==2, "header is configured wrong!");

    let client = reqwest::Client::builder().timeout(Duration::from_secs(10)).build()?;

    //delete server cache
    let resp = client.post(&param.delete_cache_url)
        .header(headers[0], headers[1])
        .form(&DeleteCacheParam { url: param.url.to_string()})
        .send().await?;

    info!("delete cache : {} , resp: {}", param.delete_cache_url, resp.status());

    //delete cf cache
    let resp = client.post(&s.config.cache_config.cf_purge_cache_url)
        .header("Authorization", &s.config.cache_config.cf_token)
        // .json(&json!({"files": [&param.url]}))
        .json(&json!({"purge_everything": true}))
        .send().await?;
    info!("delete cf cache : resp: {}",  resp.status());

    tokio::time::sleep(Duration::from_secs(5)).await;

    //render html
    let html = play_cache::render_html_in_browser(&param.url).await?;

    //save cache
    let resp = client.post(&param.save_cache_url)
        .header(headers[0], headers[1])
        .form(&SaveCacheParam { url: param.url.to_string(), cache_content: html.to_string() })
        .send().await?;
    info!("cache result upload to : {} , resp: {}", param.save_cache_url, resp.status());


    //delete cf cache (again to make sure cache is latest)
    let resp = client.post(&s.config.cache_config.cf_purge_cache_url)
        .header("Authorization", &s.config.cache_config.cf_token)
        // .json(&json!({"files": [&param.url]}))
        .json(&json!({"purge_everything": true}))
        .send().await?;
    info!("delete cf cache again : resp: {}",  resp.status());

    tokio::time::sleep(Duration::from_secs(5)).await;

    //visit again to make new cache
    client.get(&param.url).send().await?;

    Ok(())
}

#[cfg(feature = "play_cache")]
async fn cache_html(s: S, Query(param): Query<CacheRequestParam>) -> HTML {
    let s_copy = s.clone();
    tokio::spawn(async move {
        if let Err(e) = update_cache_in_remote(s_copy,&param).await {
            error!("cache_html error: {:?}", e);
        }
    });

    Ok(Html("ok".to_string()))
}


#[cfg(not(feature = "play_cache"))]
async fn cache_html(Query(param): Query<CacheRequestParam>) -> HTML {
    Ok(Html("play_cache feature is disabled.".to_owned()))
}



#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.

    use std::env;
    use std::path::Path;
    use anyhow::Context;
    use shared::constants::DATA_DIR;
    use crate::{init_log, mock_state};
    use super::*;

    #[tokio::test]
    async fn test_get_cache() -> anyhow::Result<()> {
        let r = get_cache_content(&"https://crab.rs".parse().unwrap()).await;
        println!("{:?}", r);
        Ok(())
    }
    #[tokio::test]
    async fn test_generate_cache_key() -> anyhow::Result<()> {
        let r = generate_cache_key(&"https://crab.rs".parse().unwrap());
        println!("{r}");
        let r = generate_cache_key(&"https://crab.rs/".parse().unwrap());
        println!("{r}");
        let r = generate_cache_key(&"https://crab.rs/abc/a".parse().unwrap());
        println!("{r}");
        let r = generate_cache_key(&"https://crab.rs/abc/a/".parse().unwrap());
        println!("{r}");
        let r = generate_cache_key(&"https://crab.rs/abc".parse().unwrap());
        println!("{r}");
        let r = generate_cache_key(&"https://crab.rs/abc/".parse().unwrap());
        println!("{r}");
        let r = generate_cache_key(&"https://crab.rs/abc?a=1&b=2".parse().unwrap());
        println!("{r}");
        let r = generate_cache_key(&"https://crab.rs/abc/?a=1&b=2".parse().unwrap());
        println!("{r}");
        Ok(())
    }
    #[tokio::test]
    async fn test_get_cache_content() -> anyhow::Result<()> {
        env::set_var(DATA_DIR, Path::new(env!("CARGO_MANIFEST_DIR")).join("output_dir"));
        let r = get_cache_content(&"https://crab.rs".parse().unwrap()).await?;
        println!("{r}");
        Ok(())
    }
    #[tokio::test]
    async fn test_save_cache() -> anyhow::Result<()> {
        env::set_var(DATA_DIR, Path::new(env!("CARGO_MANIFEST_DIR")).join("output_dir"));

        let resp = save_cache(Form(SaveCacheParam {
            url: "https://crab.rs".to_string(), cache_content: "9999".to_string() })).await.unwrap();

        assert_eq!(resp.0, "Ok.");
        Ok(())
    }
    #[tokio::test]
    async fn test_delete_cache() -> anyhow::Result<()> {
        env::set_var(DATA_DIR, Path::new(env!("CARGO_MANIFEST_DIR")).join("output_dir"));

        let resp = delete_cache(Form(DeleteCacheParam {
            url: "https://crab.rs".to_string()
        })).await.unwrap();

        assert_eq!(resp.0, "Ok.");
        Ok(())
    }
    #[tokio::test]
    async fn test_update_cache_in_remote() -> anyhow::Result<()> {
        env::set_var(DATA_DIR, Path::new(env!("CARGO_MANIFEST_DIR")).join("output_dir"));

        init_log!();
        let s = mock_state!();
        let resp = update_cache_in_remote(s, &CacheRequestParam {
            url: "https://crab.rs".to_string(),
            save_cache_url: "http://127.0.0.1:3000/cache/save".to_string(),
            delete_cache_url: "http://127.0.0.1:3000/cache/delete".to_string(),
            header: "a=1".to_string(),
        }).await?;

        Ok(())
    }
}