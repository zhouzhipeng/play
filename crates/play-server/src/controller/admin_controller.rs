use std::{env, fs, io};
use std::env::temp_dir;
use std::fs::File;
use std::io::{BufRead, BufReader, copy, Cursor, Write};
use std::path::Path;
use std::time::Duration;

use anyhow::{anyhow, bail};
use axum::{Form, Json};
use axum::body::Bytes;
use axum::extract::{Multipart, Query};
use axum::response::{Html, IntoResponse, Response};
use chrono::Local;
use std::process::Command;
use fs_extra::dir::CopyOptions;
use futures_util::TryStreamExt;
use http::StatusCode;
use reqwest::{ClientBuilder, Url};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio_util::codec::{BytesCodec, FramedRead};
use tracing::{info, error};
use zip::{ZipArchive, ZipWriter, CompressionMethod, write::{FileOptions, ExtendedFileOptions}};

use play_shared::{current_timestamp, timestamp_to_date_str};
use play_shared::constants::DATA_DIR;

use crate::{data_dir, promise, files_dir, HTML, method_router, R, return_error, S, template};
use crate::config::{Config, get_config_path, read_config_file, save_config_file};
use crate::tables::change_log::ChangeLog;

// Create the init function manually to handle conditional compilation
pub fn init() -> axum::Router<std::sync::Arc<crate::AppState>> {
    let mut router = axum::Router::new();
    router = router.route("/admin", axum::routing::get(enter_admin_page));
    router = router.route("/admin/upgrade", axum::routing::get(upgrade));
    router = router.route("/admin/save-config", axum::routing::post(save_config));
    router = router.route("/admin/reboot", axum::routing::get(reboot));
    router = router.route("/admin/backup", axum::routing::get(backup));
    router = router.route("/admin/backup-encrypted", axum::routing::get(backup_encrypted));
    router = router.route("/admin/backup-encrypted-to-cloud", axum::routing::get(backup_encrypted_to_cloud));
    router = router.route("/admin/restore", axum::routing::post(restore));
    router = router.route("/admin/logs", axum::routing::get(display_logs));
    router = router.route("/admin/clean-change-logs", axum::routing::get(clean_change_logs));
    router = router.route("/admin/translator", axum::routing::get(translator_page));
    router = router.route("/admin/translate", axum::routing::post(translate_text));
    
    #[cfg(feature = "play-dylib-loader")]
    {
        router = router.route("/admin/get-request-info", axum::routing::get(get_request_info));
        router = router.route("/admin/push-response-info", axum::routing::post(push_response_info));
        router = router.route("/admin/store-request-info", axum::routing::post(store_request_info));
    }
    
    router
}

// Use stores from play-dylib-loader
#[cfg(feature = "play-dylib-loader")]
use play_dylib_loader::{get_request, store_response};

#[derive(Deserialize)]
struct RequestIdQuery {
    request_id: i64,
}

#[derive(Deserialize)]
struct UpgradeRequest {
    url: Option<String>,
}

#[derive(Deserialize)]
struct SaveConfigReq {
    new_content: String,
}
#[derive(Deserialize)]
struct DeleteChangelogReq {
    #[serde(default="default_days")]
    days: u32,
}

#[derive(Deserialize)]
struct TranslateRequest {
    text: String,
    #[serde(default)]
    complex_mode: bool,
}

fn default_days()->u32{
    3
}

#[cfg(feature = "play-dylib-loader")]
async fn get_request_info(Query(RequestIdQuery { request_id }): Query<RequestIdQuery>) -> Response {
    match get_request(request_id) {
        Some(request) => {
            Json(request).into_response()
        },
        None => {
            (StatusCode::NOT_FOUND, format!("Request with id {} not found", request_id)).into_response()
        }
    }
}

#[cfg(feature = "play-dylib-loader")]
async fn push_response_info(
    Query(RequestIdQuery { request_id }): Query<RequestIdQuery>,
    Json(response): Json<play_dylib_loader::HttpResponse>
) -> Response {
    store_response(request_id, response);
    (StatusCode::OK, "Response stored successfully").into_response()
}

#[cfg(feature = "play-dylib-loader")]
async fn store_request_info(
    Query(RequestIdQuery { request_id }): Query<RequestIdQuery>,
    Json(request): Json<play_dylib_loader::HttpRequest>
) -> Response {
    play_dylib_loader::store_request(request_id, request);
    (StatusCode::OK, "Request stored successfully").into_response()
}
async fn clean_change_logs(s: S, Query(DeleteChangelogReq{days}): Query<DeleteChangelogReq>) -> R<String> {
    let days_ago = days;
    let timestamp = current_timestamp!() - (days_ago * 24 * 60 * 60 * 1000) as i64;
    let date_str = timestamp_to_date_str!(timestamp);

    let result = ChangeLog::delete_days_ago(&date_str, &s.db).await?;

    let msg = format!("Cleaned {} change log entries older than {} days", result.rows_affected(), days_ago);
    info!("{msg}");

    Ok(msg)
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
        tokio::time::sleep(Duration::from_millis(10)).await;
        shutdown();
    });
    Ok("will reboot in a sec.".to_string())
}


async fn backup(s: S) -> R<impl IntoResponse> {
    let files_path = files_dir!();

    //make a temp dir
    let folder_path = data_dir!().join("backup");
    if folder_path.exists() {
        fs::remove_dir_all(&folder_path)?;
    }
    fs::create_dir(&folder_path)?;

    //db file path
    let raw = s.config.database.url.to_string();
    let db_path = Path::new(&raw["sqlite://".len()..raw.len()]).to_path_buf();

    //config file path
    let config_file_path = get_config_path()?;

    fs_extra::copy_items(&vec![files_path, db_path, config_file_path.into()], &folder_path, &CopyOptions { copy_inside: true, ..Default::default() })?;

    let target_file = data_dir!().join("play.zip");
    if target_file.exists() {
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

            // In axum 0.8 we use Body::from_stream instead of StreamBody
            let body = axum::body::Body::from_stream(stream);
            Ok(Response::new(body))
        }
        Err(_) => {
            // 文件无法打开时，返回 HTTP 404 状态码
            return_error!("file not found!")
        }
    }
}

async fn backup_encrypted_to_cloud(s: S) -> R<String> {
    // Check GitHub token early
    let github_token = s.config.misc_config.github_token.clone();
    if github_token.is_empty() {
        return_error!("GitHub token not configured in config.toml");
    }

    // Clone necessary config values for the spawned task
    let database_url = s.config.database.url.clone();
    let passcode = s.config.auth_config.passcode.clone();
    let mail_notify_url = s.config.misc_config.mail_notify_url.clone();

    // Spawn background task
    tokio::spawn(async move {
        let result = async {
            let files_path = files_dir!();

            //make a temp dir
            let folder_path = data_dir!().join("backup");
            if folder_path.exists() {
                fs::remove_dir_all(&folder_path)?;
            }
            fs::create_dir(&folder_path)?;

            //db file path
            let db_path = Path::new(&database_url["sqlite://".len()..database_url.len()]).to_path_buf();

            //config file path
            let config_file_path = get_config_path()?;

            fs_extra::copy_items(&vec![files_path, db_path, config_file_path.into()], &folder_path, &CopyOptions { copy_inside: true, ..Default::default() })?;

            // Create unencrypted zip first
            let temp_file = data_dir!().join("play_temp.zip");
            if temp_file.exists() {
                tokio::fs::remove_file(&temp_file).await?;
            }
            crate::controller::files_controller::zip_dir(&folder_path, &temp_file)?;

            // Create encrypted zip with passcode
            let timestamp = current_timestamp!();
            let date_str = timestamp_to_date_str!(timestamp);
            let backup_filename = format!("play_backup_{}.zip", date_str);
            let target_file = data_dir!().join(&backup_filename);
            if target_file.exists() {
                tokio::fs::remove_file(&target_file).await?;
            }

            // Read the temp zip and create encrypted version
            let temp_data = fs::read(&temp_file)?;
            let encrypted_file = File::create(&target_file)?;
            let mut zip = ZipWriter::new(encrypted_file);
            
            let options = FileOptions::<ExtendedFileOptions>::default()
                .compression_method(CompressionMethod::Deflated)
                .with_aes_encryption(zip::AesMode::Aes256, passcode.as_str());
            
            zip.start_file("play_backup.zip", options)?;
            std::io::Write::write_all(&mut zip, &temp_data)?;
            zip.finish()?;

            // Clean up temp file
            fs::remove_file(&temp_file)?;

            // Read the encrypted file
            let file_data = fs::read(&target_file)?;
            
            let client = ClientBuilder::new()
                .timeout(Duration::from_secs(300))
                .build()?;

            // First, get the release ID for the 'backup' tag
            let release_url = "https://api.github.com/repos/zhouzhipeng/play/releases/tags/backup";

            // Get the release information again to get the upload URL
            let release_response = client
                .get(release_url)
                .header("Authorization", format!("Bearer {}", github_token))
                .header("Accept", "application/vnd.github+json")
                .header("X-GitHub-Api-Version", "2022-11-28")
                .header("User-Agent", "play-server-backup")
                .send()
                .await?;

            let release_info: Value = release_response.json().await?;
            let upload_url_template = release_info["upload_url"]
                .as_str()
                .ok_or_else(|| anyhow!("Upload URL not found in release info"))?;
            
            // Remove the {?name,label} template part and add our filename
            let actual_upload_url = upload_url_template
                .replace("{?name,label}", "")
                + "?name=" + &backup_filename;

            // Upload the file
            let upload_response = client
                .post(&actual_upload_url)
                .header("Authorization", format!("Bearer {}", github_token))
                .header("Accept", "application/vnd.github+json")
                .header("X-GitHub-Api-Version", "2022-11-28")
                .header("User-Agent", "play-server-backup")
                .header("Content-Type", "application/zip")
                .body(file_data)
                .send()
                .await?;

            if !upload_response.status().is_success() {
                let error_text = upload_response.text().await?;
                bail!("Failed to upload to GitHub: {}", error_text);
            }

            // Clean up old backup files - keep only latest 10
            // Get the assets from the release info we already have
            let assets_url = format!("https://api.github.com/repos/zhouzhipeng/play/releases/tags/backup");
            let assets_response = client
                .get(&assets_url)
                .header("Authorization", format!("Bearer {}", github_token))
                .header("Accept", "application/vnd.github+json")
                .header("X-GitHub-Api-Version", "2022-11-28")
                .header("User-Agent", "play-server-backup")
                .send()
                .await?;

            if assets_response.status().is_success() {
                let release_data: Value = assets_response.json().await?;
                let assets = release_data["assets"].as_array().unwrap_or(&Vec::new()).clone();
                
                // Filter only backup files (matching pattern play_backup_*.zip)
                let mut backup_files: Vec<(String, String, String)> = assets
                    .iter()
                    .filter_map(|asset| {
                        let name = asset["name"].as_str()?;
                        let id = asset["id"].as_u64()?.to_string();
                        let created_at = asset["created_at"].as_str()?.to_string();
                        if name.starts_with("play_backup_") && name.ends_with(".zip") {
                            Some((name.to_string(), id, created_at))
                        } else {
                            None
                        }
                    })
                    .collect();

                // Sort by created_at timestamp (newest first)
                backup_files.sort_by(|a, b| b.2.cmp(&a.2));

                // Delete files beyond the 10th position
                if backup_files.len() > 10 {
                    for (name, id, _) in backup_files.iter().skip(10) {
                        let delete_url = format!("https://api.github.com/repos/zhouzhipeng/play/releases/assets/{}", id);
                        let delete_response = client
                            .delete(&delete_url)
                            .header("Authorization", format!("Bearer {}", github_token))
                            .header("Accept", "application/vnd.github+json")
                            .header("X-GitHub-Api-Version", "2022-11-28")
                            .header("User-Agent", "play-server-backup")
                            .send()
                            .await?;
                        
                        if delete_response.status().is_success() {
                            info!("Deleted old backup file: {}", name);
                        } else {
                            error!("Failed to delete old backup file: {}", name);
                        }
                    }
                }
            }

            // Clean up local file after successful upload
            fs::remove_file(&target_file)?;
            fs::remove_dir_all(&folder_path)?;

            Ok::<String, anyhow::Error>(format!("Backup {} successfully uploaded to GitHub releases", backup_filename))
        }.await;

        // Log the result and send notification
        match result {
            Ok(msg) => {
                info!("Cloud backup success: {}", msg);
                // Send success notification
                let sender = urlencoding::encode("cloud backup success").into_owned();
                let title = urlencoding::encode(&msg).into_owned();
                let _ = reqwest::get(format!("{}/{}/{}", mail_notify_url, sender, title)).await;
            }
            Err(e) => {
                error!("Cloud backup failed: {}", e);
                // Send error notification
                let sender = urlencoding::encode("cloud backup error").into_owned();
                let title = urlencoding::encode(&format!("Backup failed: {}", e)).into_owned();
                let _ = reqwest::get(format!("{}/{}/{}", mail_notify_url, sender, title)).await;
            }
        }
    });

    Ok("Backup to cloud started in background. You will receive a notification when complete.".to_string())
}

async fn backup_encrypted(s: S) -> R<impl IntoResponse> {
    let files_path = files_dir!();

    //make a temp dir
    let folder_path = data_dir!().join("backup");
    if folder_path.exists() {
        fs::remove_dir_all(&folder_path)?;
    }
    fs::create_dir(&folder_path)?;

    //db file path
    let raw = s.config.database.url.to_string();
    let db_path = Path::new(&raw["sqlite://".len()..raw.len()]).to_path_buf();

    //config file path
    let config_file_path = get_config_path()?;

    fs_extra::copy_items(&vec![files_path, db_path, config_file_path.into()], &folder_path, &CopyOptions { copy_inside: true, ..Default::default() })?;

    // Create unencrypted zip first
    let temp_file = data_dir!().join("play_temp.zip");
    if temp_file.exists() {
        tokio::fs::remove_file(&temp_file).await?;
    }
    crate::controller::files_controller::zip_dir(&folder_path, &temp_file)?;

    // Create encrypted zip with passcode
    let target_file = data_dir!().join("play_encrypted.zip");
    if target_file.exists() {
        tokio::fs::remove_file(&target_file).await?;
    }

    // Read the temp zip and create encrypted version
    let temp_data = fs::read(&temp_file)?;
    let encrypted_file = File::create(&target_file)?;
    let mut zip = ZipWriter::new(encrypted_file);
    
    let options = FileOptions::<ExtendedFileOptions>::default()
        .compression_method(CompressionMethod::Deflated)
        .with_aes_encryption(zip::AesMode::Aes256, s.config.auth_config.passcode.as_str());
    
    zip.start_file("play_backup.zip", options)?;
    std::io::Write::write_all(&mut zip, &temp_data)?;
    zip.finish()?;

    // Clean up temp file
    fs::remove_file(&temp_file)?;

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

            // In axum 0.8 we use Body::from_stream instead of StreamBody
            let body = axum::body::Body::from_stream(stream);
            Ok(Response::new(body))
        }
        Err(_) => {
            // 文件无法打开时，返回 HTTP 404 状态码
            return_error!("file not found!")
        }
    }
}

static ADMIN_HTML: &str = include_str!("templates/admin_new.html");

async fn enter_admin_page(s: S) -> HTML {
    // let config = &CONFIG;
    let config_content = read_config_file(false).await?;
    let config_path = get_config_path()?;

    let built_time = timestamp_to_date_str!(env!("BUILT_TIME").parse::<i64>()?);
    let html = ADMIN_HTML.replace("{{title}}", "admin panel")
        .replace("{{config_content}}", &config_content)
        .replace("{{config_path}}", &config_path)
        .replace("{{built_time}}", &built_time)
        .replace("{{title}}", "admin panel");

    Ok(Html(html))
}

fn copy_me() -> anyhow::Result<()> {
    // 获取当前执行文件的路径
    let current_exe = env::current_exe()?;

    // 创建目标文件名（在当前目录下，添加 "_copy" 后缀）
    let file_name = current_exe.file_name().unwrap().to_str().unwrap();
    let copy_name = format!("{}_bak", file_name);
    let destination = current_exe.parent().unwrap().join(copy_name);

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
    promise!( archive.len()==1, "upgrade_url for zip file is not valid, should have only one file inside!");
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


        if r.is_ok(){
            let sender = urlencoding::encode("upgrade done").into_owned();
            let title = urlencoding::encode(&format!("result : {:?}", r)).into_owned();
            reqwest::get(format!("{}/{}/{}", &s.config.misc_config.mail_notify_url, sender, title)).await;
            shutdown();
        }else{
            let sender = urlencoding::encode("upgrade error").into_owned();
            let title = urlencoding::encode(&format!("result : {:?}", r)).into_owned();
            reqwest::get(format!("{}/{}/{}", &s.config.misc_config.mail_notify_url, sender, title)).await;

        }

    });


    Ok(Html("upgrading in background, pls wait a minute and system will restart automatically later.".to_string()))
}

pub fn shutdown() {
    info!("ready to shutdown...");
    std::process::exit(0);
}


// 处理文件上传和解压的路由处理函数
async fn restore(mut multipart: Multipart) -> R<String> {
    if let Ok(Some(field)) = multipart.next_field().await {
        let temp_dir = tempfile::tempdir()?;

        let archive = Cursor::new(field.bytes().await?);
        extract_and_copy(archive, temp_dir.path(),data_dir!() )?;
    }
    Ok("ok".to_string())
}


fn extract_and_copy(cursor: Cursor<Bytes>, extract_dir: &Path, target_dir: &Path) -> anyhow::Result<()> {
    // 创建临时解压目录
    fs::create_dir_all(extract_dir)?;

    // 解压ZIP文件到临时目录
    zip_extract::extract(cursor, extract_dir, true)?;

    // 找到第一个子目录
    if let Some(first_dir) = fs::read_dir(extract_dir)?
        .filter_map(|entry| entry.ok())
        .find(|entry| entry.path().is_dir())
    {
        // 复制文件到目标目录
        copy_dir_contents(&first_dir.path(), target_dir)?;

        // 清理临时解压目录
        fs::remove_dir_all(extract_dir)?;
    } else {
        bail!("在目录A中没有找到子目录");
    }



    Ok(())
}

fn copy_dir_contents(src: &Path, dst: &Path) -> io::Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if file_type.is_dir() {
            copy_dir_contents(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}

// Translator functions
static TRANSLATOR_HTML: &str = include_str!("templates/translator.html");

async fn translator_page() -> HTML {
    let html = TRANSLATOR_HTML.replace("{{title}}", "Translator Tool");
    Ok(Html(html))
}

async fn translate_text(Form(req): Form<TranslateRequest>) -> R<Json<Value>> {
    let system_prompt = if req.complex_mode {
        r#"You are an expert bilingual Chinese-English dictionary with pronunciation expertise.

Auto-detect input language and provide comprehensive translation:

FOR ENGLISH INPUT → CHINESE:
- Simplified & Traditional Chinese
- Pinyin with tone marks (e.g., zhōngwén)
- Tone numbers (e.g., zhong1wen2)
- HSK level classification

FOR CHINESE INPUT → ENGLISH:
- English translation(s)
- IPA pronunciation (e.g., /ˈɪŋɡlɪʃ/)
- American phonetic (e.g., ING-glish)
- British phonetic (e.g., ING-glish)
- Syllable breakdown

ALWAYS include:
- Multiple definitions if applicable
- Common collocations
- Usage frequency
- Register level (formal/informal/neutral)

Respond with this exact JSON structure:
{
  "input": "user input",
  "detected_language": "english|chinese",
  "translations": {
    "primary": "main translation",
    "alternatives": ["alt1", "alt2"],
    "chinese_simplified": "简体",
    "chinese_traditional": "繁體",
    "english": "English translation"
  },
  "pronunciation": {
    "pinyin_tones": "pīnyīn with tone marks",
    "pinyin_numbers": "pin1yin1 with numbers",
    "ipa_us": "/aɪˈpiːeɪ/",
    "ipa_uk": "/aɪˈpiːeɪ/",
    "phonetic_us": "fuh-NET-ik",
    "phonetic_uk": "fuh-NET-ik",
    "syllables": "syl-la-bles",
    "stress": "primary stress on syllable 2"
  },
  "grammar": {
    "part_of_speech": "noun|verb|adj|etc",
    "gender": "if applicable",
    "plural": "plural form if applicable",
    "verb_forms": "past/present/future if verb"
  },
  "definitions": [
    {"meaning": "definition 1", "register": "formal|informal|neutral"},
    {"meaning": "definition 2", "register": "formal|informal|neutral"}
  ],
  "examples": [
    {
      "source": "example in source language",
      "target": "example in target language",
      "context": "usage context"
    }
  ],
  "related_words": {
    "synonyms": ["syn1", "syn2"],
    "antonyms": ["ant1", "ant2"],
    "collocations": ["common phrase 1", "common phrase 2"],
    "derivatives": ["related word 1", "related word 2"]
  },
  "metadata": {
    "frequency": "very_common|common|uncommon|rare",
    "difficulty": "HSK1-6|beginner|intermediate|advanced",
    "domain": "general|technical|medical|legal|etc",
    "origin": "etymology if interesting",
    "cultural_notes": "cultural context if relevant"
  }
}

Provide accurate, comprehensive information. Return only the JSON object without any markdown formatting or code blocks."#
    } else {
        r#"You are a fast bilingual Chinese-English translator.

Auto-detect input language and provide quick translation with basic pronunciation:

FOR ENGLISH INPUT → CHINESE:
- Simplified Chinese
- Pinyin with tone marks

FOR CHINESE INPUT → ENGLISH:
- English translation
- Basic IPA pronunciation

Respond with this exact JSON structure (ONLY translations and pronunciation):
{
  "input": "user input",
  "detected_language": "english|chinese",
  "translations": {
    "primary": "main translation",
    "alternatives": ["alt1", "alt2"],
    "chinese_simplified": "简体",
    "english": "English translation"
  },
  "pronunciation": {
    "pinyin_tones": "pīnyīn with tone marks",
    "ipa_us": "/aɪˈpiːeɪ/"
  }
}

Be fast and concise. Return only the JSON object without any markdown formatting or code blocks."#
    };

    // Call Claude with the translation request
    let output = Command::new("claude")
        .arg("-p")
        .arg(&req.text)
        .arg("--append-system-prompt")
        .arg(system_prompt)
        .arg("--output-format")
        .arg("json")
        .arg("--max-turns")
        .arg("1")
        .output()
        .map_err(|e| anyhow!("Failed to execute claude command: {}", e))?;

    if !output.status.success() {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        return_error!("Claude command failed: {}", error_msg);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Parse the output to extract the result field
    let parsed: Value = serde_json::from_str(&stdout)
        .map_err(|e| anyhow!("Failed to parse claude output: {}", e))?;
    
    // Extract the result field
    let result = parsed.get("result")
        .ok_or_else(|| anyhow!("No result field in claude output"))?;
    
    // The result contains markdown-wrapped JSON, extract and parse it
    let result_str = result.as_str()
        .ok_or_else(|| anyhow!("Result field is not a string"))?;
    
    // Remove markdown code block formatting
    let json_str = if result_str.starts_with("```json\n") {
        result_str.strip_prefix("```json\n").unwrap().strip_suffix("\n```").unwrap()
    } else if result_str.starts_with("```\n") {
        result_str.strip_prefix("```\n").unwrap().strip_suffix("\n```").unwrap()
    } else {
        result_str
    };
    
    // Parse the clean JSON
    let translation_result: Value = serde_json::from_str(json_str)
        .map_err(|e| anyhow!("Failed to parse translation JSON: {}", e))?;
    
    Ok(Json(translation_result))
}


#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.

    use super::*;

    #[tokio::test]
    pub async fn test_copy_me() {
        let r = copy_me();
        println!("{:?}", r);
    }
}