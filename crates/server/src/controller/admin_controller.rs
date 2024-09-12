use std::{env, fs};
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
use zip::ZipArchive;

use shared::constants::DATA_DIR;

use crate::{ensure, HTML, method_router, R, S, template};
use crate::config::{Config, get_config_path, read_config_file, save_config_file};

method_router!(
    get : "/admin" -> enter_admin_page,
    get : "/admin/upgrade" -> upgrade,
    post : "/admin/save-config" -> save_config,
    get : "/admin/logs" -> display_logs,
);

#[derive(Deserialize)]
struct UpgradeRequest {
    url: Option<String>,
}

#[derive(Deserialize)]
struct SaveConfigReq {
    new_content: String,
}

async fn display_logs(s: S) -> HTML {
    let count = 100;
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

    let tail_lines: Vec<String> = lines.iter()
        .rev()
        .take(count)
        .rev()
        .cloned()
        .collect();

    let coverted_str = tail_lines.join("\n");
    let converted = ansi_to_html::convert(&coverted_str).unwrap();

    Ok(Html(converted))
}

async fn save_config(s: S, Form(req): Form<SaveConfigReq>) -> R<String> {
    toml::from_str::<Config>(&req.new_content)?;
    save_config_file(&req.new_content)?;
    tokio::spawn(async {
        shutdown();
    });
    Ok("save ok,will reboot in a sec.".to_string())
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

fn copy_me()->anyhow::Result<()>{
    // 获取当前执行文件的路径
    let current_exe = env::current_exe()?;

    // 创建目标文件名（在当前目录下，添加 "_copy" 后缀）
    let file_name = current_exe.file_name().unwrap().to_str().unwrap();
    let copy_name = format!("{}_bak", file_name);
    let destination =  current_exe.parent().unwrap().join(copy_name);

    // 复制文件
    fs::copy(&current_exe, &destination)?;

    info!("copy_me >> destination : {:?}",destination);
    Ok(())
}

async fn upgrade_in_background(url: Url) -> anyhow::Result<()> {
    info!("begin to download from url in background  : {}", url);

    // download file
    let new_binary = temp_dir().join("new_play_bin");
    let mut file = File::create(&new_binary)?;
    let client = ClientBuilder::new().timeout(Duration::from_secs(30)).build()?;
    let response = client.get(url).send().await?;
    let content = Cursor::new(response.bytes().await?);

    let mut archive = ZipArchive::new(BufReader::new(content))?;
    ensure!( archive.len()==1, "upgrade_url for zip file is not valid, should have only one file inside!");
    let mut inside_file = archive.by_index(0)?;
    std::io::copy(&mut inside_file, &mut file)?;

    //make a backup for old binary
    copy_me()?;

    info!("downloaded and saved at : {:?}", new_binary);

    self_replace::self_replace(&new_binary)?;
    std::fs::remove_file(&new_binary)?;

    info!("replaced ok. and ready to shutdown self");

    Ok(())
}

async fn upgrade(s: S, Query(upgrade): Query<UpgradeRequest>) -> HTML {
    info!("begin upgrade...");

    let url = Url::parse(&upgrade.url.as_ref().unwrap_or(&s.config.upgrade_url))?;


    tokio::spawn(async move {
        let r = upgrade_in_background(url).await;
        info!("upgrade_in_background result >> {:?}", r);


        let sender= urlencoding::encode("upgrade done").into_owned();
        let title= urlencoding::encode(&format!("result : {:?}", r)).into_owned();
        reqwest::get(format!("{}/{}/{}",&s.config.misc_config.mail_notify_url,  sender, title)).await;


        shutdown();
    });


    Ok(Html("upgrading in background, pls wait a minute and system will restart automatically later.".to_string()))
}

pub  fn shutdown() {
    info!("ready to shutdown...");
    std::process::exit(0);
}



#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.

    use super::*;

    #[tokio::test]
    pub async fn test_copy_me(){
        let r = copy_me();
        println!("{:?}", r);
    }
}