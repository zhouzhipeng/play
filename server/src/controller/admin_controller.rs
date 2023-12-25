use std::env::temp_dir;
use std::fs::File;
use std::io::Cursor;
use std::time::Duration;

use axum::extract::Query;
use axum::Form;
use axum::response::Html;
use reqwest::{ClientBuilder, Url};
use serde::Deserialize;
use serde_json::json;
use tracing::info;

use crate::{check_if, HTML, method_router, S, template};
use crate::config::{get_config_path, read_config_file, save_config_file};

method_router!(
    get : "/admin/upgrade" -> upgrade,
    get : "/admin/shutdown" -> shutdown,
    get : "/admin/index" -> enter_admin_page,
    post : "/admin/save-config" -> save_config,
);

#[derive(Deserialize)]
struct UpgradeRequest {
    url: String,
}

#[derive(Deserialize)]
struct SaveConfigReq{
    new_content: String,
}
async fn save_config(s: S, Form(req): Form<SaveConfigReq>) -> HTML {
    save_config_file(&req.new_content)?;
    Ok(Html("save ok.".to_string()))
}


async fn enter_admin_page(s: S) -> HTML {
    // let config = &CONFIG;
    let config_content = read_config_file()?;
    let config_path = get_config_path()?;

    template!(s, "frame.html"+"fragments/admin.html", json!({
        "upgrade_url" : "",
        "config_content" : config_content,
        "config_path" : config_path,
    }))
}

async fn upgrade_in_background(url: Url) ->anyhow::Result<()>{
    info!("begin to download from url in background  : {}", url);

    // download file
    let new_binary = temp_dir().join("new_play_bin");
    let mut file = File::create(&new_binary)?;
    let client = ClientBuilder::new().timeout(Duration::from_secs(30)).build()?;
    let response = client.get(url).send().await?;
    let mut content = Cursor::new(response.bytes().await?);
    std::io::copy(&mut content, &mut file)?;

    info!("downloaded and saved at : {:?}", new_binary);

    self_replace::self_replace(&new_binary)?;
    std::fs::remove_file(&new_binary)?;

    info!("replaced ok.");


    Ok(())
}

async fn upgrade(s: S, Query(upgrade): Query<UpgradeRequest>) -> HTML {
    let url = Url::parse(&upgrade.url)?;


    tokio::spawn(async move{
        let r = upgrade_in_background(url).await;
        info!("upgrade_in_background result >> {:?}", r);
    });


    Ok(Html("upgrading in background, pls wait and restart manually later.".to_string()))
}

async fn shutdown(s: S) -> HTML {


    info!("ready to reboot...");

    s.shutdown_handle.shutdown();

    Ok(Html("shutdown ok.".to_string()))
}
