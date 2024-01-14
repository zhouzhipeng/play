use std::env;
use std::env::temp_dir;
use std::fs::File;
use std::io::{BufRead, BufReader, Cursor};
use std::path::Path;
use std::time::Duration;

use axum::extract::Query;
use axum::Form;
use axum::response::Html;
use chrono::Local;
use reqwest::{ClientBuilder, Url};
use serde::Deserialize;
use serde_json::json;
use tracing::info;
use shared::constants::DATA_DIR;

use crate::{check_if, HTML, method_router, S, template};
use crate::config::{get_config_path, read_config_file, save_config_file};

method_router!(
    get : "/admin" -> enter_admin_page,
    get : "/admin/upgrade" -> upgrade,
    get : "/admin/shutdown" -> shutdown,
    post : "/admin/save-config" -> save_config,
    get : "/admin/logs" -> display_logs,
);

#[derive(Deserialize)]
struct UpgradeRequest {
    url: Option<String>,
}

#[derive(Deserialize)]
struct SaveConfigReq{
    new_content: String,
}
async fn display_logs(s: S) -> HTML {
    let count= 50;
    // Get the current local date
    let now = Local::now();

    // Format the date as a string
    let date_string = now.format("%Y-%m-%d").to_string();
    let file_path = Path::new(env::var(DATA_DIR)?.as_str()).join(format!("play.{}.log", date_string));
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);

    let lines: Vec<String> = reader.lines()
        .filter_map(Result::ok)
        .collect();

    let tail_lines:Vec<String> = lines.iter()
        .rev()
        .take(count)
        .rev()
        .cloned()
        .collect();

    let coverted_str = tail_lines.join("\n");
    let converted = ansi_to_html::convert(&coverted_str).unwrap();

    Ok(Html(converted))
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
        "title": "admin panel",
        "upgrade_url" : &s.config.upgrade_url,
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

    info!("replaced ok. and ready to shutdown self");

    shutdown().await;


    Ok(())
}

async fn upgrade(s: S, Query(upgrade): Query<UpgradeRequest>) -> HTML {
    info!("begin upgrade...");

    let url = Url::parse(&upgrade.url.as_ref().unwrap_or(&s.config.upgrade_url))?;


    tokio::spawn(async move{
        let r = upgrade_in_background(url).await;
        info!("upgrade_in_background result >> {:?}", r);
    });


    Ok(Html("upgrading in background, pls wait and restart manually later.".to_string()))
}

async fn shutdown() -> HTML {

    info!("ready to reboot...");

    std::process::exit(0);

}
