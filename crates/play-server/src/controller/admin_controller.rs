use std::env::temp_dir;
use std::fs::File;
use std::future::Future;
use std::io::{copy, BufRead, BufReader, Cursor, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use std::{env, fs, io};

use anyhow::{anyhow, bail, Context};
use axum::body::Bytes;
use axum::extract::{Multipart, Query};
use axum::response::{Html, IntoResponse, Response};
use axum::{Form, Json};
use chrono::{DateTime, Local, Utc};
use fs_extra::dir::CopyOptions;
use futures_util::TryStreamExt;
use hmac::{Hmac, Mac};
use http::StatusCode;
use reqwest::{Client, ClientBuilder, Url};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::process::Command;
use std::sync::Arc;
use tokio_util::codec::{BytesCodec, FramedRead};
use tracing::{error, info};
use zip::{
    write::{ExtendedFileOptions, FileOptions},
    CompressionMethod, ZipArchive, ZipWriter,
};

use play_shared::constants::DATA_DIR;
use play_shared::{current_timestamp, timestamp_to_date_str};

use crate::config::{
    get_config_path, read_config_file, save_config_file, CloudflareDnsRecordConfig, Config,
    OneKeyChangeIpConfig,
};
use crate::tables::change_log::ChangeLog;
use crate::{data_dir, files_dir, method_router, promise, return_error, template, HTML, R, S};

// Create the init function manually to handle conditional compilation
pub fn init() -> axum::Router<std::sync::Arc<crate::AppState>> {
    let mut router = axum::Router::new();
    router = router.route("/admin", axum::routing::get(enter_admin_page));
    router = router.route("/admin/upgrade", axum::routing::get(upgrade));
    router = router.route("/admin/save-config", axum::routing::post(save_config));
    router = router.route("/admin/reboot", axum::routing::get(reboot));
    router = router.route("/admin/backup", axum::routing::get(backup));
    router = router.route(
        "/admin/backup-encrypted",
        axum::routing::get(backup_encrypted),
    );
    router = router.route(
        "/admin/backup-encrypted-to-cloud",
        axum::routing::get(backup_encrypted_to_cloud),
    );
    router = router.route(
        "/admin/one-key-change-ip",
        axum::routing::get(one_key_change_ip),
    );
    router = router.route("/admin/restore", axum::routing::post(restore));
    router = router.route("/admin/logs", axum::routing::get(display_logs));
    router = router.route(
        "/admin/clean-change-logs",
        axum::routing::get(clean_change_logs),
    );
    router = router.route("/admin/translator", axum::routing::get(translator_page));
    router = router.route("/admin/translate", axum::routing::post(translate_text));

    #[cfg(feature = "play-dylib-loader")]
    {
        router = router.route(
            "/admin/get-request-info",
            axum::routing::get(get_request_info),
        );
        router = router.route(
            "/admin/push-response-info",
            axum::routing::post(push_response_info),
        );
        router = router.route(
            "/admin/store-request-info",
            axum::routing::post(store_request_info),
        );
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
    #[serde(default = "default_days")]
    days: u32,
}

#[derive(Deserialize)]
struct TranslateRequest {
    text: String,
    #[serde(default)]
    complex_mode: bool,
}

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone)]
struct OneKeyChangeIpResult {
    old_static_ip_name: Option<String>,
    old_ip: Option<String>,
    new_static_ip_name: String,
    new_ip: String,
}

#[derive(Debug, Deserialize, Clone)]
struct StaticIpInfo {
    name: String,
    #[serde(default, rename = "ipAddress")]
    ip_address: String,
    #[serde(default, rename = "attachedTo")]
    attached_to: Option<String>,
}

impl StaticIpInfo {
    fn attached_to_instance(&self, instance_name: &str) -> bool {
        self.attached_to.as_deref() == Some(instance_name)
    }
}

#[derive(Debug, Deserialize)]
struct StaticIpsResponse {
    #[serde(default, rename = "staticIps")]
    static_ips: Vec<StaticIpInfo>,
}

#[derive(Debug, Deserialize)]
struct StaticIpResponse {
    #[serde(rename = "staticIp")]
    static_ip: StaticIpInfo,
}

struct LightsailClient {
    http_client: Client,
    config: OneKeyChangeIpConfig,
}

fn default_days() -> u32 {
    3
}

#[cfg(feature = "play-dylib-loader")]
async fn get_request_info(Query(RequestIdQuery { request_id }): Query<RequestIdQuery>) -> Response {
    match get_request(request_id) {
        Some(request) => Json(request).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            format!("Request with id {} not found", request_id),
        )
            .into_response(),
    }
}

#[cfg(feature = "play-dylib-loader")]
async fn push_response_info(
    Query(RequestIdQuery { request_id }): Query<RequestIdQuery>,
    Json(response): Json<play_dylib_loader::HttpResponse>,
) -> Response {
    store_response(request_id, response);
    (StatusCode::OK, "Response stored successfully").into_response()
}

#[cfg(feature = "play-dylib-loader")]
async fn store_request_info(
    Query(RequestIdQuery { request_id }): Query<RequestIdQuery>,
    Json(request): Json<play_dylib_loader::HttpRequest>,
) -> Response {
    play_dylib_loader::store_request(request_id, request);
    (StatusCode::OK, "Request stored successfully").into_response()
}
async fn clean_change_logs(
    s: S,
    Query(DeleteChangelogReq { days }): Query<DeleteChangelogReq>,
) -> R<String> {
    let days_ago = days;
    let timestamp = current_timestamp!() - (days_ago * 24 * 60 * 60 * 1000) as i64;
    let date_str = timestamp_to_date_str!(timestamp);

    let result = ChangeLog::delete_days_ago(&date_str, &s.db).await?;

    let msg = format!(
        "Cleaned {} change log entries older than {} days",
        result.rows_affected(),
        days_ago
    );
    info!("{msg}");

    Ok(msg)
}
async fn display_logs(s: S) -> HTML {
    let count = 100;
    // Get the current local date
    let now = Local::now();

    // Format the date as a string
    let date_string = now.format("%Y-%m-%d").to_string();
    let file_path =
        Path::new(env::var(DATA_DIR)?.as_str()).join(format!("play.{}.log", date_string));
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);

    let lines: Vec<String> = reader.lines().filter_map(Result::ok).collect();

    let tail_lines: Vec<String> = lines.iter().rev().take(count).rev().cloned().collect();

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

    fs_extra::copy_items(
        &vec![files_path, db_path, config_file_path.into()],
        &folder_path,
        &CopyOptions {
            copy_inside: true,
            ..Default::default()
        },
    )?;

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
            let db_path =
                Path::new(&database_url["sqlite://".len()..database_url.len()]).to_path_buf();

            //config file path
            let config_file_path = get_config_path()?;

            fs_extra::copy_items(
                &vec![files_path, db_path, config_file_path.into()],
                &folder_path,
                &CopyOptions {
                    copy_inside: true,
                    ..Default::default()
                },
            )?;

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
            let actual_upload_url =
                upload_url_template.replace("{?name,label}", "") + "?name=" + &backup_filename;

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
            let assets_url =
                format!("https://api.github.com/repos/zhouzhipeng/play/releases/tags/backup");
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
                let assets = release_data["assets"]
                    .as_array()
                    .unwrap_or(&Vec::new())
                    .clone();

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
                        let delete_url = format!(
                            "https://api.github.com/repos/zhouzhipeng/play/releases/assets/{}",
                            id
                        );
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

            Ok::<String, anyhow::Error>(format!(
                "Backup {} successfully uploaded to GitHub releases",
                backup_filename
            ))
        }
        .await;

        // Log the result and send notification
        match result {
            Ok(msg) => {
                info!("Cloud backup success: {}", msg);
                // // Send success notification
                // let sender = urlencoding::encode("cloud backup success").into_owned();
                // let title = urlencoding::encode(&msg).into_owned();
                // let _ = reqwest::get(format!("{}/{}/{}", mail_notify_url, sender, title)).await;
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

    Ok(
        "Backup to cloud started in background. You will receive a notification when complete."
            .to_string(),
    )
}

async fn one_key_change_ip(s: S) -> R<String> {
    let config = s.config.one_key_change_ip.clone();
    validate_one_key_change_ip_config(&config)?;

    let mail_notify_url = s.config.misc_config.mail_notify_url.clone();
    let data_dir = PathBuf::from(env::var(DATA_DIR)?);

    tokio::spawn(async move {
        match run_one_key_change_ip_task(config, mail_notify_url.clone(), data_dir).await {
            Ok(result) => {
                info!(
                    "one-key-change-ip completed: old_static_ip_name={:?}, old_ip={:?}, new_static_ip_name={}, new_ip={}",
                    result.old_static_ip_name, result.old_ip, result.new_static_ip_name, result.new_ip
                );
            }
            Err(error) => {
                error!("one-key-change-ip failed: {:?}", error);
                app_push(
                    &mail_notify_url,
                    "one-key-change-ip error",
                    &format!("failed: {}", error),
                )
                .await;
            }
        }
    });

    Ok("one-key-change-ip started in background.".to_string())
}

fn validate_one_key_change_ip_config(config: &OneKeyChangeIpConfig) -> anyhow::Result<()> {
    require_config_value(&config.aws_region, "one_key_change_ip.aws_region")?;
    require_config_value(
        &config.aws_access_key_id,
        "one_key_change_ip.aws_access_key_id",
    )?;
    require_config_value(
        &config.aws_secret_access_key,
        "one_key_change_ip.aws_secret_access_key",
    )?;
    require_config_value(&config.instance_name, "one_key_change_ip.instance_name")?;
    require_config_value(
        &config.cloudflare_api_token,
        "one_key_change_ip.cloudflare_api_token",
    )?;
    require_config_value(
        &config.cloudflare_zone_id,
        "one_key_change_ip.cloudflare_zone_id",
    )?;

    promise!(
        !config.cloudflare_dns_records.is_empty(),
        "one_key_change_ip.cloudflare_dns_records must contain at least one DNS record"
    );

    for (index, record) in config.cloudflare_dns_records.iter().enumerate() {
        require_config_value(
            &record.name,
            &format!("one_key_change_ip.cloudflare_dns_records[{index}].name"),
        )?;
        require_config_value(
            &record.record_type,
            &format!("one_key_change_ip.cloudflare_dns_records[{index}].record_type"),
        )?;
        promise!(
            record.ttl > 0,
            "one_key_change_ip.cloudflare_dns_records[{index}].ttl must be greater than 0"
        );
    }

    Ok(())
}

fn require_config_value(value: &str, name: &str) -> anyhow::Result<()> {
    promise!(!value.trim().is_empty(), "{} is not configured", name);
    Ok(())
}

async fn run_one_key_change_ip_task(
    config: OneKeyChangeIpConfig,
    mail_notify_url: String,
    data_dir: PathBuf,
) -> anyhow::Result<OneKeyChangeIpResult> {
    validate_one_key_change_ip_config(&config)?;

    app_push(
        &mail_notify_url,
        "one-key-change-ip",
        &format!("started for instance {}", config.instance_name),
    )
    .await;

    let http_client = ClientBuilder::new()
        .timeout(Duration::from_secs(config.request_timeout_secs.max(1)))
        .http1_only()
        .pool_max_idle_per_host(0)
        .user_agent("play-server-one-key-change-ip")
        .build()?;
    let lightsail_client = LightsailClient {
        http_client: http_client.clone(),
        config: config.clone(),
    };

    let old_static_ip = lightsail_client
        .find_attached_static_ip(&config.instance_name)
        .await?;

    if let Some(old) = &old_static_ip {
        app_push(
            &mail_notify_url,
            "one-key-change-ip",
            &format!("old static IP: {} {}", old.name, old.ip_address),
        )
        .await;

        lightsail_client.detach_static_ip(&old.name).await?;
        app_push(
            &mail_notify_url,
            "one-key-change-ip",
            &format!("detached old static IP {}", old.name),
        )
        .await;

        lightsail_client
            .wait_until_no_static_ip_attached(&config.instance_name)
            .await?;
        app_push(
            &mail_notify_url,
            "one-key-change-ip",
            &format!("{} has no static IP attached", config.instance_name),
        )
        .await;
    } else {
        app_push(
            &mail_notify_url,
            "one-key-change-ip",
            &format!("no existing static IP attached to {}", config.instance_name),
        )
        .await;
    }

    let new_static_ip_name = format!(
        "{}-ip-{}",
        config.instance_name,
        Local::now().format("%Y%m%d%H%M%S")
    );

    lightsail_client
        .allocate_static_ip(&new_static_ip_name)
        .await?;
    app_push(
        &mail_notify_url,
        "one-key-change-ip",
        &format!("allocated static IP name {}", new_static_ip_name),
    )
    .await;

    let new_static_ip = lightsail_client
        .wait_until_static_ip_exists(&new_static_ip_name)
        .await?;
    promise!(
        !new_static_ip.ip_address.trim().is_empty(),
        "allocated static IP `{}` has no IP address",
        new_static_ip_name
    );
    app_push(
        &mail_notify_url,
        "one-key-change-ip",
        &format!("allocated static IP address {}", new_static_ip.ip_address),
    )
    .await;

    lightsail_client
        .attach_static_ip(&config.instance_name, &new_static_ip_name)
        .await?;
    app_push(
        &mail_notify_url,
        "one-key-change-ip",
        &format!(
            "attaching {} to {}",
            new_static_ip_name, config.instance_name
        ),
    )
    .await;

    lightsail_client
        .wait_until_static_ip_attached(&config.instance_name, &new_static_ip_name)
        .await?;
    app_push(
        &mail_notify_url,
        "one-key-change-ip",
        &format!(
            "attached {} to {}",
            new_static_ip_name, config.instance_name
        ),
    )
    .await;

    if let Some(old) = &old_static_ip {
        if old.name != new_static_ip_name {
            lightsail_client.release_static_ip(&old.name).await?;
            app_push(
                &mail_notify_url,
                "one-key-change-ip",
                &format!("released old static IP {}", old.name),
            )
            .await;
        }
    }

    for record in &config.cloudflare_dns_records {
        let query_response = query_cloudflare_dns_record(&http_client, &config, record).await?;
        let record_id = resolve_cloudflare_record_id(record, &query_response)?;
        app_push(
            &mail_notify_url,
            "one-key-change-ip",
            &format!(
                "queried Cloudflare DNS record {} ({})",
                record.name, record_id
            ),
        )
        .await;

        update_cloudflare_dns_record(
            &http_client,
            &config,
            record,
            &record_id,
            &new_static_ip.ip_address,
        )
        .await?;
        app_push(
            &mail_notify_url,
            "one-key-change-ip",
            &format!(
                "updated Cloudflare DNS record {} -> {}",
                record.name, new_static_ip.ip_address
            ),
        )
        .await;
    }

    let old_ip = old_static_ip
        .as_ref()
        .map(|static_ip| static_ip.ip_address.clone())
        .filter(|ip| !ip.trim().is_empty())
        .ok_or_else(|| {
            anyhow!(
                "cannot update vpn.yaml because no existing static IP address was attached to {}",
                config.instance_name
            )
        })?;
    let vpn_path = data_dir.join("files").join("vpn.yaml");
    replace_ip_in_vpn_yaml(&vpn_path, &old_ip, &new_static_ip.ip_address).await?;
    app_push(
        &mail_notify_url,
        "one-key-change-ip",
        &format!(
            "updated {} from {} to {}",
            vpn_path.display(),
            old_ip,
            new_static_ip.ip_address
        ),
    )
    .await;

    app_push(
        &mail_notify_url,
        "one-key-change-ip final",
        &format!("done: {} -> {}", old_ip, new_static_ip.ip_address),
    )
    .await;

    Ok(OneKeyChangeIpResult {
        old_static_ip_name: old_static_ip
            .as_ref()
            .map(|static_ip| static_ip.name.clone()),
        old_ip: Some(old_ip),
        new_static_ip_name,
        new_ip: new_static_ip.ip_address,
    })
}

impl LightsailClient {
    async fn lightsail_api(&self, action: &str, payload: Value) -> anyhow::Result<Value> {
        let host = format!("lightsail.{}.amazonaws.com", self.config.aws_region);
        let url = format!("https://{host}/");
        let target = format!("Lightsail_20161128.{action}");
        let body = payload.to_string();
        let headers =
            build_lightsail_sigv4_headers(&self.config, &host, &target, &body, Utc::now())?;

        let mut request = self.http_client.post(url).body(body);
        for (name, value) in headers {
            request = request.header(name.as_str(), value);
        }

        let clone_error = format!("failed to clone Lightsail {action} request");
        let response = retry_external_request(
            &format!("Lightsail {action} request"),
            3,
            Duration::from_secs(2),
            || {
                let request = request.try_clone();
                let clone_error = clone_error.clone();
                async move {
                    let request = request.ok_or_else(|| anyhow!(clone_error))?;
                    Ok(request.send().await?)
                }
            },
        )
        .await?;
        parse_json_response(response, &format!("Lightsail {action}")).await
    }

    async fn get_static_ips(&self) -> anyhow::Result<Vec<StaticIpInfo>> {
        let value = self.lightsail_api("GetStaticIps", json!({})).await?;
        let response = serde_json::from_value::<StaticIpsResponse>(value)
            .context("failed to parse Lightsail GetStaticIps response")?;
        Ok(response.static_ips)
    }

    async fn get_static_ip(&self, static_ip_name: &str) -> anyhow::Result<StaticIpInfo> {
        let value = self
            .lightsail_api("GetStaticIp", json!({ "staticIpName": static_ip_name }))
            .await?;
        let response = serde_json::from_value::<StaticIpResponse>(value)
            .context("failed to parse Lightsail GetStaticIp response")?;
        Ok(response.static_ip)
    }

    async fn find_attached_static_ip(
        &self,
        instance_name: &str,
    ) -> anyhow::Result<Option<StaticIpInfo>> {
        Ok(self
            .get_static_ips()
            .await?
            .into_iter()
            .find(|static_ip| static_ip.attached_to_instance(instance_name)))
    }

    async fn detach_static_ip(&self, static_ip_name: &str) -> anyhow::Result<()> {
        self.lightsail_api("DetachStaticIp", json!({ "staticIpName": static_ip_name }))
            .await?;
        Ok(())
    }

    async fn allocate_static_ip(&self, static_ip_name: &str) -> anyhow::Result<()> {
        self.lightsail_api(
            "AllocateStaticIp",
            json!({ "staticIpName": static_ip_name }),
        )
        .await?;
        Ok(())
    }

    async fn attach_static_ip(
        &self,
        instance_name: &str,
        static_ip_name: &str,
    ) -> anyhow::Result<()> {
        self.lightsail_api(
            "AttachStaticIp",
            json!({
                "instanceName": instance_name,
                "staticIpName": static_ip_name
            }),
        )
        .await?;
        Ok(())
    }

    async fn release_static_ip(&self, static_ip_name: &str) -> anyhow::Result<()> {
        self.lightsail_api("ReleaseStaticIp", json!({ "staticIpName": static_ip_name }))
            .await?;
        Ok(())
    }

    async fn wait_until_no_static_ip_attached(&self, instance_name: &str) -> anyhow::Result<()> {
        let deadline = Instant::now() + self.operation_timeout();
        loop {
            if self.find_attached_static_ip(instance_name).await?.is_none() {
                return Ok(());
            }
            promise!(
                Instant::now() < deadline,
                "timed out waiting for {} to detach its static IP",
                instance_name
            );
            tokio::time::sleep(self.poll_interval()).await;
        }
    }

    async fn wait_until_static_ip_exists(
        &self,
        static_ip_name: &str,
    ) -> anyhow::Result<StaticIpInfo> {
        let deadline = Instant::now() + self.operation_timeout();
        let mut last_error = None;

        loop {
            match self.get_static_ip(static_ip_name).await {
                Ok(static_ip) => return Ok(static_ip),
                Err(error) => last_error = Some(error.to_string()),
            }

            let last_error_text = last_error.as_deref().unwrap_or("none");
            promise!(
                Instant::now() < deadline,
                "timed out waiting for static IP `{}` to exist; last error: {}",
                static_ip_name,
                last_error_text
            );
            tokio::time::sleep(self.poll_interval()).await;
        }
    }

    async fn wait_until_static_ip_attached(
        &self,
        instance_name: &str,
        static_ip_name: &str,
    ) -> anyhow::Result<()> {
        let deadline = Instant::now() + self.operation_timeout();
        loop {
            let attached = self.get_static_ips().await?.into_iter().any(|static_ip| {
                static_ip.name == static_ip_name && static_ip.attached_to_instance(instance_name)
            });
            if attached {
                return Ok(());
            }

            promise!(
                Instant::now() < deadline,
                "timed out waiting for static IP `{}` to attach to {}",
                static_ip_name,
                instance_name
            );
            tokio::time::sleep(self.poll_interval()).await;
        }
    }

    fn poll_interval(&self) -> Duration {
        Duration::from_secs(self.config.poll_interval_secs.max(1))
    }

    fn operation_timeout(&self) -> Duration {
        Duration::from_secs(self.config.operation_timeout_secs.max(1))
    }
}

fn build_lightsail_sigv4_headers(
    config: &OneKeyChangeIpConfig,
    host: &str,
    target: &str,
    body: &str,
    now: DateTime<Utc>,
) -> anyhow::Result<Vec<(String, String)>> {
    let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
    let date = now.format("%Y%m%d").to_string();
    let credential_scope = format!("{}/{}/lightsail/aws4_request", date, config.aws_region);
    let payload_hash = sha256_hex(body.as_bytes());

    let mut canonical_headers = vec![
        (
            "content-type".to_string(),
            "application/x-amz-json-1.1".to_string(),
        ),
        ("host".to_string(), host.to_string()),
        ("x-amz-date".to_string(), amz_date.clone()),
        ("x-amz-target".to_string(), target.to_string()),
    ];

    let session_token = config.aws_session_token.trim();
    if !session_token.is_empty() {
        canonical_headers.push((
            "x-amz-security-token".to_string(),
            session_token.to_string(),
        ));
    }
    canonical_headers.sort_by(|left, right| left.0.cmp(&right.0));

    let canonical_headers_text = canonical_headers
        .iter()
        .map(|(name, value)| format!("{name}:{value}\n"))
        .collect::<String>();
    let signed_headers = canonical_headers
        .iter()
        .map(|(name, _)| name.as_str())
        .collect::<Vec<_>>()
        .join(";");

    let canonical_request = format!(
        "POST\n/\n\n{}\n{}\n{}",
        canonical_headers_text, signed_headers, payload_hash
    );
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{}\n{}\n{}",
        amz_date,
        credential_scope,
        sha256_hex(canonical_request.as_bytes())
    );
    let signing_key = aws_signing_key(
        &config.aws_secret_access_key,
        &date,
        &config.aws_region,
        "lightsail",
    )?;
    let signature = hex::encode(hmac_sha256(&signing_key, string_to_sign.as_bytes())?);
    let authorization = format!(
        "AWS4-HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={}",
        config.aws_access_key_id, credential_scope, signed_headers, signature
    );

    let mut headers = vec![
        (
            "Content-Type".to_string(),
            "application/x-amz-json-1.1".to_string(),
        ),
        ("Host".to_string(), host.to_string()),
        ("X-Amz-Date".to_string(), amz_date),
        ("X-Amz-Target".to_string(), target.to_string()),
        ("Authorization".to_string(), authorization),
    ];
    if !session_token.is_empty() {
        headers.push((
            "X-Amz-Security-Token".to_string(),
            session_token.to_string(),
        ));
    }

    Ok(headers)
}

fn aws_signing_key(
    secret_access_key: &str,
    date: &str,
    region: &str,
    service: &str,
) -> anyhow::Result<Vec<u8>> {
    let date_key = hmac_sha256(
        format!("AWS4{}", secret_access_key).as_bytes(),
        date.as_bytes(),
    )?;
    let region_key = hmac_sha256(&date_key, region.as_bytes())?;
    let service_key = hmac_sha256(&region_key, service.as_bytes())?;
    hmac_sha256(&service_key, b"aws4_request")
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> anyhow::Result<Vec<u8>> {
    let mut mac = HmacSha256::new_from_slice(key)
        .map_err(|error| anyhow!("failed to create HMAC-SHA256 key: {}", error))?;
    mac.update(data);
    Ok(mac.finalize().into_bytes().to_vec())
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

async fn retry_external_request<F, Fut, T>(
    description: &str,
    attempts: u32,
    delay: Duration,
    mut operation: F,
) -> anyhow::Result<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = anyhow::Result<T>>,
{
    let attempts = attempts.max(1);
    for attempt in 1..=attempts {
        match operation().await {
            Ok(value) => return Ok(value),
            Err(error) if attempt < attempts => {
                error!(
                    "{} failed on attempt {}/{}: {}",
                    description, attempt, attempts, error
                );
                tokio::time::sleep(delay).await;
            }
            Err(error) => {
                return Err(error).with_context(|| {
                    format!("{} failed after {} attempt(s)", description, attempts)
                });
            }
        }
    }

    unreachable!("attempts is clamped to at least one");
}

async fn query_cloudflare_dns_record(
    http_client: &Client,
    config: &OneKeyChangeIpConfig,
    record: &CloudflareDnsRecordConfig,
) -> anyhow::Result<Value> {
    let url = format!(
        "https://api.cloudflare.com/client/v4/zones/{}/dns_records?type={}&name={}",
        config.cloudflare_zone_id,
        urlencoding::encode(&record.record_type),
        urlencoding::encode(&record.name)
    );

    let cloudflare_api_token = config.cloudflare_api_token.clone();
    let response = retry_external_request(
        &format!("Cloudflare query {} request", record.name),
        3,
        Duration::from_secs(2),
        || {
            let http_client = http_client.clone();
            let url = url.clone();
            let cloudflare_api_token = cloudflare_api_token.clone();
            async move {
                Ok(http_client
                    .get(url)
                    .bearer_auth(cloudflare_api_token)
                    .header("Content-Type", "application/json")
                    .send()
                    .await?)
            }
        },
    )
    .await?;

    parse_json_response(response, &format!("Cloudflare query {}", record.name)).await
}

async fn update_cloudflare_dns_record(
    http_client: &Client,
    config: &OneKeyChangeIpConfig,
    record: &CloudflareDnsRecordConfig,
    record_id: &str,
    new_ip: &str,
) -> anyhow::Result<Value> {
    let url = format!(
        "https://api.cloudflare.com/client/v4/zones/{}/dns_records/{}",
        config.cloudflare_zone_id, record_id
    );

    let payload = json!({
        "type": &record.record_type,
        "name": &record.name,
        "content": new_ip,
        "ttl": record.ttl,
        "proxied": record.proxied
    });

    let cloudflare_api_token = config.cloudflare_api_token.clone();
    let response = retry_external_request(
        &format!("Cloudflare update {} request", record.name),
        3,
        Duration::from_secs(2),
        || {
            let http_client = http_client.clone();
            let url = url.clone();
            let cloudflare_api_token = cloudflare_api_token.clone();
            let payload = payload.clone();
            async move {
                Ok(http_client
                    .patch(url)
                    .bearer_auth(cloudflare_api_token)
                    .header("Content-Type", "application/json")
                    .json(&payload)
                    .send()
                    .await?)
            }
        },
    )
    .await?;

    parse_json_response(response, &format!("Cloudflare update {}", record.name)).await
}

fn resolve_cloudflare_record_id(
    record: &CloudflareDnsRecordConfig,
    query_response: &Value,
) -> anyhow::Result<String> {
    if let Some(record_id) = record
        .record_id
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        return Ok(record_id.to_string());
    }

    let result = query_response
        .get("result")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            anyhow!(
                "Cloudflare query response has no result array for {}",
                record.name
            )
        })?;

    result
        .iter()
        .find(|item| {
            item.get("name").and_then(Value::as_str) == Some(record.name.as_str())
                && item.get("type").and_then(Value::as_str) == Some(record.record_type.as_str())
        })
        .and_then(|item| item.get("id").and_then(Value::as_str))
        .map(|id| id.to_string())
        .ok_or_else(|| {
            anyhow!(
                "Cloudflare DNS record id not found for {} {}",
                record.record_type,
                record.name
            )
        })
}

async fn parse_json_response(
    response: reqwest::Response,
    description: &str,
) -> anyhow::Result<Value> {
    let status = response.status();
    let text = response.text().await?;

    if !status.is_success() {
        bail!("{} failed ({}): {}", description, status, text);
    }

    if text.trim().is_empty() {
        return Ok(json!({}));
    }

    serde_json::from_str(&text)
        .with_context(|| format!("failed to parse {} JSON response: {}", description, text))
}

async fn app_push(mail_notify_url: &str, sender: &str, title: &str) {
    let notify_url = mail_notify_url.trim();
    if notify_url.is_empty() {
        info!("app push skipped because misc_config.mail_notify_url is empty: {sender} {title}");
        return;
    }

    let sender = urlencoding::encode(sender).into_owned();
    let title = urlencoding::encode(title).into_owned();
    let url = format!("{}/{}/{}", notify_url.trim_end_matches('/'), sender, title);

    match reqwest::get(url).await {
        Ok(response) => info!("app push response: {}", response.status()),
        Err(error) => error!("app push failed: {}", error),
    }
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

    fs_extra::copy_items(
        &vec![files_path, db_path, config_file_path.into()],
        &folder_path,
        &CopyOptions {
            copy_inside: true,
            ..Default::default()
        },
    )?;

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

async fn replace_ip_in_vpn_yaml(path: &Path, old_ip: &str, new_ip: &str) -> anyhow::Result<()> {
    promise!(!old_ip.trim().is_empty(), "old IP is empty");
    promise!(!new_ip.trim().is_empty(), "new IP is empty");

    let content = tokio::fs::read_to_string(path).await?;
    promise!(
        content.contains(old_ip),
        "old IP `{}` not found in {}",
        old_ip,
        path.display()
    );

    tokio::fs::write(path, content.replace(old_ip, new_ip)).await?;
    Ok(())
}

static ADMIN_HTML: &str = include_str!("templates/admin_new.html");

async fn enter_admin_page(s: S) -> HTML {
    // let config = &CONFIG;
    let config_content = read_config_file(false).await?;
    let config_path = get_config_path()?;

    let built_time = timestamp_to_date_str!(env!("BUILT_TIME").parse::<i64>()?);
    let html = ADMIN_HTML
        .replace("{{title}}", "admin panel")
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

    info!("copy_me >> destination : {:?}", destination);
    Ok(())
}

async fn upgrade_in_background(url: Url) -> anyhow::Result<()> {
    info!("begin to download from url in background  : {}", url);

    // download file
    let new_binary = temp_dir().join("new_play_bin");
    let mut file = File::create(&new_binary)?;
    let client = ClientBuilder::new()
        .timeout(Duration::from_secs(30))
        .build()?;
    let response = client.get(url).send().await?;
    let content = Cursor::new(response.bytes().await?);

    let mut archive = ZipArchive::new(BufReader::new(content))?;
    promise!(
        archive.len() == 1,
        "upgrade_url for zip file is not valid, should have only one file inside!"
    );
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

        if r.is_ok() {
            let sender = urlencoding::encode("upgrade done").into_owned();
            let title = urlencoding::encode(&format!("result : {:?}", r)).into_owned();
            reqwest::get(format!(
                "{}/{}/{}",
                &s.config.misc_config.mail_notify_url, sender, title
            ))
            .await;
            shutdown();
        } else {
            let sender = urlencoding::encode("upgrade error").into_owned();
            let title = urlencoding::encode(&format!("result : {:?}", r)).into_owned();
            reqwest::get(format!(
                "{}/{}/{}",
                &s.config.misc_config.mail_notify_url, sender, title
            ))
            .await;
        }
    });

    Ok(Html(
        "upgrading in background, pls wait a minute and system will restart automatically later."
            .to_string(),
    ))
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
        extract_and_copy(archive, temp_dir.path(), data_dir!())?;
    }
    Ok("ok".to_string())
}

fn extract_and_copy(
    cursor: Cursor<Bytes>,
    extract_dir: &Path,
    target_dir: &Path,
) -> anyhow::Result<()> {
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
        r#"You are a translation service that MUST translate ANY text provided, regardless of context or completeness.

CRITICAL INSTRUCTIONS:
1. ALWAYS translate the input text EXACTLY as given - do not refuse, ask questions, or request clarification
2. If text appears incomplete, translate it anyway
3. If text references files or objects, translate the literal text
4. NEVER respond with explanations, only translations

Auto-detect input language and translate:

FOR ENGLISH INPUT → CHINESE:
- Simplified Chinese translation
- Pinyin with tone marks

FOR CHINESE INPUT → ENGLISH:
- English translation
- Basic IPA pronunciation

Example: If user inputs "参考这个文件实现一个新的" you MUST translate it to "Refer to this file to implement a new one" NOT ask for more information.

Respond with this exact JSON structure:
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

Return only the JSON object without any markdown formatting or code blocks."#
    };

    // Call Claude with the translation request
    let output = Command::new("claude")
        .current_dir(temp_dir())
        .arg("-p")
        .arg(&req.text)
        .arg("--append-system-prompt")
        .arg(system_prompt)
        .arg("--output-format")
        .arg("json")
        // .arg("--max-turns")
        // .arg("1")
        .output()
        .map_err(|e| anyhow!("Failed to execute claude command: {}", e))?;

    if !output.status.success() {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        return_error!("Claude command failed: {}", error_msg);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    info!("Claude stdout: {}", stdout);

    // Parse the output to extract the result field
    let parsed: Value = serde_json::from_str(&stdout)
        .map_err(|e| anyhow!("Failed to parse claude output: {}", e))?;

    // Extract the result field
    let result = parsed
        .get("result")
        .ok_or_else(|| anyhow!("No result field in claude output"))?;

    // The result contains markdown-wrapped JSON, extract and parse it
    let result_str = result
        .as_str()
        .ok_or_else(|| anyhow!("Result field is not a string"))?;

    // Remove markdown code block formatting
    let json_str = if result_str.starts_with("```json\n") {
        result_str
            .strip_prefix("```json\n")
            .unwrap()
            .strip_suffix("\n```")
            .unwrap()
    } else if result_str.starts_with("```\n") {
        result_str
            .strip_prefix("```\n")
            .unwrap()
            .strip_suffix("\n```")
            .unwrap()
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

    #[tokio::test]
    async fn replaces_old_ip_in_vpn_yaml() -> anyhow::Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let vpn_path = temp_dir.path().join("vpn.yaml");
        tokio::fs::write(
            &vpn_path,
            "server: 13.230.224.104\nremote: 13.230.224.104:500\n",
        )
        .await?;

        replace_ip_in_vpn_yaml(&vpn_path, "13.230.224.104", "203.0.113.10").await?;

        let content = tokio::fs::read_to_string(&vpn_path).await?;
        assert_eq!(content, "server: 203.0.113.10\nremote: 203.0.113.10:500\n");
        Ok(())
    }

    #[tokio::test]
    async fn replace_ip_in_vpn_yaml_errors_when_old_ip_is_missing() -> anyhow::Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let vpn_path = temp_dir.path().join("vpn.yaml");
        tokio::fs::write(&vpn_path, "server: 198.51.100.10\n").await?;

        let error = replace_ip_in_vpn_yaml(&vpn_path, "13.230.224.104", "203.0.113.10")
            .await
            .unwrap_err();

        assert!(error.to_string().contains("13.230.224.104"));
        Ok(())
    }

    #[test]
    fn one_key_change_ip_config_validation_requires_dns_records() {
        let config = crate::config::OneKeyChangeIpConfig {
            aws_region: "ap-northeast-1".to_string(),
            aws_access_key_id: "test-access-key".to_string(),
            aws_secret_access_key: "test-secret-key".to_string(),
            instance_name: "Debian-1".to_string(),
            cloudflare_api_token: "test-cloudflare-token".to_string(),
            cloudflare_zone_id: "test-zone-id".to_string(),
            ..Default::default()
        };

        let error = validate_one_key_change_ip_config(&config).unwrap_err();

        assert!(error.to_string().contains("cloudflare_dns_records"));
    }

    #[test]
    fn one_key_change_ip_config_validation_allows_missing_record_id() {
        let config = crate::config::OneKeyChangeIpConfig {
            aws_region: "ap-northeast-1".to_string(),
            aws_access_key_id: "test-access-key".to_string(),
            aws_secret_access_key: "test-secret-key".to_string(),
            instance_name: "Debian-1".to_string(),
            cloudflare_api_token: "test-cloudflare-token".to_string(),
            cloudflare_zone_id: "test-zone-id".to_string(),
            cloudflare_dns_records: vec![CloudflareDnsRecordConfig {
                name: "ip.zhouzhipeng.com".to_string(),
                record_id: None,
                ..Default::default()
            }],
            ..Default::default()
        };

        validate_one_key_change_ip_config(&config).unwrap();
    }

    #[test]
    fn resolves_cloudflare_record_id_from_query_response_when_missing_in_config() {
        let record = CloudflareDnsRecordConfig {
            record_type: "A".to_string(),
            name: "ip.zhouzhipeng.com".to_string(),
            record_id: None,
            ..Default::default()
        };
        let response = json!({
            "success": true,
            "result": [
                {"id": "ignored-aaaa", "type": "AAAA", "name": "ip.zhouzhipeng.com"},
                {"id": "resolved-a", "type": "A", "name": "ip.zhouzhipeng.com"}
            ]
        });

        let record_id = resolve_cloudflare_record_id(&record, &response).unwrap();

        assert_eq!(record_id, "resolved-a");
    }

    #[tokio::test]
    async fn retry_external_request_retries_until_success() -> anyhow::Result<()> {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let attempts = Arc::new(AtomicUsize::new(0));
        let attempts_copy = attempts.clone();

        let result = retry_external_request("test retry", 3, Duration::ZERO, move || {
            let attempts = attempts_copy.clone();
            async move {
                let attempt = attempts.fetch_add(1, Ordering::SeqCst);
                if attempt < 2 {
                    Err(anyhow!("temporary failure"))
                } else {
                    Ok("ok")
                }
            }
        })
        .await?;

        assert_eq!(result, "ok");
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
        Ok(())
    }
}
