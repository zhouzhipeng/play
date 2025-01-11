use std::{env, fs};
use std::env::temp_dir;
use std::fs::File;
use std::io::{BufRead, BufReader, Cursor};
use std::path::Path;
use std::time::Duration;
use anyhow::anyhow;
use axum::body::StreamBody;

use axum::extract::Query;
use axum::Form;
use axum::response::{Html, IntoResponse, Response};
use chrono::Local;
use fs_extra::dir::CopyOptions;
use futures_util::TryStreamExt;
use reqwest::{ClientBuilder, Url};
use serde::Deserialize;
use serde_json::json;
use tokio_util::codec::{BytesCodec, FramedRead};
use tracing::info;
use zip::ZipArchive;

use play_shared::constants::DATA_DIR;
use play_shared::timestamp_to_date_str;
use crate::{data_dir, ensure, files_dir, HTML, method_router, R, return_error, S, template};
use crate::config::{Config, get_config_path, read_config_file, save_config_file};

method_router!(
    get : "/admin" -> enter_admin_page,
    get : "/admin/upgrade" -> upgrade,
    post : "/admin/save-config" -> save_config,
    get : "/admin/reboot" -> reboot,
    get : "/admin/backup" -> backup,
    get : "/admin/restore" -> restore,
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

    Ok("save ok.".to_string())
}
async fn reboot() -> R<String> {
    tokio::spawn(async {
        shutdown();
    });
    Ok("will reboot in a sec.".to_string())
}


async fn backup(s: S) -> R<impl IntoResponse> {
    let files_path = files_dir!();

    //make a temp dir
    let folder_path = data_dir!().join("backup");
    if folder_path.exists(){
        fs::remove_dir_all(&folder_path)?;
    }
    fs::create_dir(&folder_path)?;

    //db file path
    let raw = s.config.database.url.to_string();
    let db_path = Path::new(&raw["sqlite://".len()..raw.len()]).to_path_buf();

    //config file path
    let config_file_path = get_config_path()?;

    fs_extra::copy_items(&vec![files_path, db_path, config_file_path.into()], &folder_path, &CopyOptions{copy_inside:true, ..Default::default()})?;

    let target_file = data_dir!().join("play.zip");
    if target_file.exists(){
        tokio::fs::remove_file(&target_file).await?;
    }

    crate::controller::files_controller::zip_dir(&folder_path, &target_file)?;
    match tokio::fs::File::open(&target_file).await {
        Ok(file) => {
            // 使用 FramedRead 和 BytesCodec 将文件转换为 Stream
            let stream = FramedRead::new(file, BytesCodec::new())
                .map_ok(|bytes| bytes.freeze())
                .map_err(|e| {
                    info!("File streaming error: {}", e);
                    // 在流中发生错误时，将错误转换为 HTTP 500 状态码
                    anyhow!("file stream error")
                });

            let stream_body = StreamBody::new(stream);
            Ok(Response::new(stream_body))
        }
        Err(_) => {
            // 文件无法打开时，返回 HTTP 404 状态码
            return_error!("file not found!")
        }
    }
}

static ADMIN_HTML : &str = include_str!("templates/admin_new.html");

async fn enter_admin_page(s: S) -> HTML {
    // let config = &CONFIG;
    let config_content = read_config_file()?;
    let config_path = get_config_path()?;

    let built_time = timestamp_to_date_str!(env!("BUILT_TIME").parse::<i64>()?);
    let html = ADMIN_HTML.replace("{{title}}", "admin panel")
        .replace("{{config_content}}", &config_content)
        .replace("{{config_path}}", &config_path)
        .replace("{{built_time}}", &built_time)
        .replace("{{title}}", "admin panel");

    Ok(Html(html))
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


// 处理文件上传和解压的路由处理函数
async fn restore(mut multipart: Multipart) ->  R<impl IntoResponse>  {
    // 获取上传的文件
    let field = multipart
        .next_field()
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?
        .ok_or((StatusCode::BAD_REQUEST, "No file uploaded".to_string()))?;

    // 检查文件名和类型
    let file_name = field
        .file_name()
        .ok_or((StatusCode::BAD_REQUEST, "No filename provided".to_string()))?
        .to_string();

    if !file_name.ends_with(".zip") {
        return Err((StatusCode::BAD_REQUEST, "Only .zip files are allowed".to_string()));
    }

    // 创建临时文件保存上传的内容
    let mut temp_file = NamedTempFile::new()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // 将上传的内容写入临时文件
    copy(&mut field.bytes().await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?.as_ref(),
         &mut temp_file)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // 解压文件
    let target_dir = Path::new("./restore_target"); // 设置解压目标目录
    extract_zip(temp_file.path(), target_dir)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(json!({
        "success": true,
        "message": "File uploaded and extracted successfully"
    })))
}

// 解压zip文件的辅助函数
fn extract_zip(zip_path: &Path, target_dir: &Path) -> Result<()> {
    // 确保目标目录存在
    std::fs::create_dir_all(target_dir)?;

    // 打开zip文件
    let file = File::open(zip_path)?;
    let mut archive = ZipArchive::new(file)?;

    // 遍历并解压所有文件
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = target_dir.join(file.mangled_name());

        if file.name().ends_with('/') {
            std::fs::create_dir_all(&outpath)?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    std::fs::create_dir_all(p)?;
                }
            }
            let mut outfile = File::create(&outpath)?;
            copy(&mut file, &mut outfile)?;
        }
    }

    Ok(())
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