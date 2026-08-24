use std::collections::{HashMap, HashSet};
use std::env::temp_dir;
use std::fmt;
use std::fs::File;
use std::future::Future;
use std::io::{BufRead, BufReader, Cursor, copy};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use std::{env, fs, io};

use anyhow::{Context, anyhow, bail};
use axum::body::Bytes;
use axum::extract::{Multipart, Query};
use axum::response::{Html, IntoResponse, Response};
use axum::{Form, Json};
use chrono::{DateTime, Local, Utc};
use fs_extra::dir::CopyOptions;
use futures_util::TryStreamExt;
use hmac::{Hmac, Mac};
use http::{StatusCode, header};
use reqwest::{Client, ClientBuilder, Url};
use safebox_sdk::{
    EncryptOptions, MAX_NOMINAL_SHARD_SIZE, PublicIdentity, WrittenBundle, encrypt_file,
};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::process::Command;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio::io::AsyncReadExt;
use tokio_util::codec::{BytesCodec, FramedRead};
use tracing::{error, info};
use zip::{
    CompressionMethod, ZipArchive,
    write::{SimpleFileOptions, ZipWriter},
};

use play_shared::constants::DATA_DIR;
use play_shared::{current_timestamp, timestamp_to_date_str};

use crate::config::{
    CloudflareDnsRecordConfig, Config, OneKeyChangeIpConfig, get_config_path, read_config_file,
    save_config_file,
};
use crate::tables::change_log::ChangeLog;
use crate::{HTML, R, S, data_dir, files_dir, method_router, promise, return_error, template};

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

static ONE_KEY_CHANGE_IP_RUNNING: AtomicBool = AtomicBool::new(false);

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

    fn is_attached(&self) -> bool {
        self.attached_to
            .as_deref()
            .map(str::trim)
            .map(|attached_to| !attached_to.is_empty())
            .unwrap_or(false)
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

#[derive(Debug)]
struct ApiResponseError {
    description: String,
    status: StatusCode,
    body: String,
}

impl fmt::Display for ApiResponseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} failed ({}): {}",
            self.description, self.status, self.body
        )
    }
}

impl std::error::Error for ApiResponseError {}

#[derive(Debug)]
struct OneKeyChangeIpRunGuard;

impl OneKeyChangeIpRunGuard {
    fn try_acquire() -> anyhow::Result<Self> {
        ONE_KEY_CHANGE_IP_RUNNING
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .map_err(|_| anyhow!("one-key-change-ip is already running"))?;
        Ok(Self)
    }
}

impl Drop for OneKeyChangeIpRunGuard {
    fn drop(&mut self) {
        ONE_KEY_CHANGE_IP_RUNNING.store(false, Ordering::SeqCst);
    }
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

const SBOX_DOWNLOAD_CONTENT_TYPE: &str = "application/octet-stream";
const GITHUB_BACKUP_RELEASE_URL: &str =
    "https://api.github.com/repos/zhouzhipeng/play/releases/tags/backup";

fn parse_sbox_public_identity(value: &str) -> anyhow::Result<PublicIdentity> {
    let value = value.trim();
    if value.is_empty() {
        bail!(
            "backup_config.sbox_public_key is not configured; paste the sboxpk1: key copied by SafeBox's 复制公钥 button"
        );
    }

    PublicIdentity::from_encoded(value).context(
        "backup_config.sbox_public_key is not a valid SafeBox public key (sboxpk1: or legacy JSON)",
    )
}

fn create_sbox_backup(
    identity: &PublicIdentity,
    database_url: &str,
    files_path: &Path,
    config_file_path: &Path,
    temporary_root: &Path,
    original_name: &str,
) -> anyhow::Result<WrittenBundle> {
    let folder_path = temporary_root.join("backup");
    fs::create_dir(&folder_path)?;

    let sqlite_path = database_url
        .strip_prefix("sqlite://")
        .filter(|path| !path.is_empty())
        .ok_or_else(|| anyhow!("database.url must be a sqlite:// file URL for backups"))?;
    let items = vec![
        files_path.to_path_buf(),
        PathBuf::from(sqlite_path),
        config_file_path.to_path_buf(),
    ];

    fs_extra::copy_items(
        &items,
        &folder_path,
        &CopyOptions {
            copy_inside: true,
            ..Default::default()
        },
    )?;

    let zip_path = temporary_root.join(original_name);
    crate::controller::files_controller::zip_dir(&folder_path, &zip_path)?;

    let mut options = EncryptOptions::new(original_name, "application/zip");
    options.title = Some("Play server backup".to_string());
    options.tags = vec!["backup".to_string(), "play-server".to_string()];
    // Prefer one downloadable .sbox object while retaining protocol-compliant
    // multipart output for backups larger than 512 MiB.
    options.target_nominal_shard_size = MAX_NOMINAL_SHARD_SIZE;

    encrypt_file(identity, &zip_path, temporary_root.join("sbox"), &options)
        .context("failed to encrypt the ZIP backup with the SafeBox public key")
}

async fn create_sbox_backup_off_thread(
    identity: PublicIdentity,
    database_url: String,
    files_path: PathBuf,
    config_file_path: PathBuf,
    temporary_directory: tempfile::TempDir,
    original_name: String,
) -> anyhow::Result<(tempfile::TempDir, WrittenBundle)> {
    let result = tokio::task::spawn_blocking(move || {
        let bundle = create_sbox_backup(
            &identity,
            &database_url,
            &files_path,
            &config_file_path,
            temporary_directory.path(),
            &original_name,
        )?;
        Ok::<_, anyhow::Error>((temporary_directory, bundle))
    })
    .await
    .context("SBOX backup worker stopped unexpectedly")??;

    Ok(result)
}

fn prepare_sbox_download(
    bundle: &WrittenBundle,
    temporary_root: &Path,
    backup_label: &str,
) -> anyhow::Result<(PathBuf, String, &'static str)> {
    if bundle.objects.len() == 1 {
        let path = bundle.objects[0].path.clone();
        let basename = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| anyhow!("SafeBox SDK returned an invalid object path"))?
            .to_string();
        return Ok((path, basename, SBOX_DOWNLOAD_CONTENT_TYPE));
    }

    // Browsers download one response at a time. Preserve every canonical SBOX
    // shard in an unencrypted transport ZIP so the user can extract and import
    // the complete Bundle into SafeBox.
    let archive_name = format!("{}_sbox_bundle.zip", backup_label);
    let archive_path = temporary_root.join(&archive_name);
    let mut archive = ZipWriter::new(File::create(&archive_path)?);
    for object in &bundle.objects {
        let basename = object
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| anyhow!("SafeBox SDK returned an invalid object path"))?;
        archive.start_file(
            basename,
            SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
        )?;
        copy(&mut File::open(&object.path)?, &mut archive)?;
    }
    archive.finish()?;

    Ok((archive_path, archive_name, "application/zip"))
}

async fn prepare_sbox_download_off_thread(
    temporary_directory: tempfile::TempDir,
    bundle: WrittenBundle,
    backup_label: String,
) -> anyhow::Result<(tempfile::TempDir, PathBuf, String, &'static str)> {
    let result = tokio::task::spawn_blocking(move || {
        let (path, name, content_type) =
            prepare_sbox_download(&bundle, temporary_directory.path(), &backup_label)?;
        Ok::<_, anyhow::Error>((temporary_directory, path, name, content_type))
    })
    .await
    .context("SBOX download worker stopped unexpectedly")??;

    Ok(result)
}

struct SboxDownloadState {
    file: tokio::fs::File,
    _temporary_directory: tempfile::TempDir,
}

async fn stream_sbox_download(
    path: &Path,
    download_name: &str,
    content_type: &'static str,
    temporary_directory: tempfile::TempDir,
) -> anyhow::Result<Response> {
    let file = tokio::fs::File::open(path).await?;
    let content_length = file.metadata().await?.len();
    let stream = futures_util::stream::try_unfold(
        SboxDownloadState {
            file,
            _temporary_directory: temporary_directory,
        },
        |mut state| async move {
            let mut buffer = vec![0_u8; 64 * 1024];
            let read = state.file.read(&mut buffer).await?;
            if read == 0 {
                return Ok::<_, io::Error>(None);
            }
            buffer.truncate(read);
            Ok(Some((Bytes::from(buffer), state)))
        },
    );

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CONTENT_LENGTH, content_length)
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", download_name),
        )
        .header(header::CACHE_CONTROL, "no-store")
        .body(axum::body::Body::from_stream(stream))
        .context("failed to build the SBOX download response")
}

fn expired_sbox_backup_assets(assets: &[Value], keep_bundles: usize) -> Vec<(String, String)> {
    let mut groups: HashMap<String, (String, Vec<(String, String)>)> = HashMap::new();

    for asset in assets {
        let Some(name) = asset["name"].as_str() else {
            continue;
        };
        let Some(label) = asset["label"].as_str() else {
            continue;
        };
        let Some(id) = asset["id"].as_u64() else {
            continue;
        };
        let Some(created_at) = asset["created_at"].as_str() else {
            continue;
        };
        if !name.ends_with(".sbox") || !label.starts_with("play_backup_") {
            continue;
        }

        let group = groups
            .entry(label.to_string())
            .or_insert_with(|| (created_at.to_string(), Vec::new()));
        if created_at > group.0.as_str() {
            group.0 = created_at.to_string();
        }
        group.1.push((name.to_string(), id.to_string()));
    }

    let mut groups: Vec<_> = groups
        .into_iter()
        .map(|(label, (created_at, assets))| (label, created_at, assets))
        .collect();
    groups.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| right.0.cmp(&left.0)));

    groups
        .into_iter()
        .skip(keep_bundles)
        .flat_map(|(_, _, assets)| assets)
        .collect()
}

async fn backup_encrypted_to_cloud(s: S) -> R<String> {
    let github_token = s.config.misc_config.github_token.clone();
    if github_token.trim().is_empty() {
        return_error!("GitHub token not configured in config.toml");
    }

    // Validate before spawning so configuration errors are returned to the caller.
    let public_identity = parse_sbox_public_identity(&s.config.backup_config.sbox_public_key)?;
    let database_url = s.config.database.url.clone();
    let mail_notify_url = s.config.misc_config.mail_notify_url.clone();
    let data_directory = PathBuf::from(env::var(DATA_DIR)?);
    let files_path = data_directory.join("files");
    let config_file_path = data_directory.join("config.toml");

    tokio::spawn(async move {
        let result = async {
            let temporary_directory = tempfile::Builder::new()
                .prefix("play-sbox-cloud-")
                .tempdir_in(&data_directory)?;
            let timestamp = current_timestamp!();
            let date_str = timestamp_to_date_str!(timestamp);
            let backup_label = format!("play_backup_{}", date_str);
            let original_name = format!("{}.zip", backup_label);
            let (temporary_directory, bundle) = create_sbox_backup_off_thread(
                public_identity,
                database_url,
                files_path,
                config_file_path,
                temporary_directory,
                original_name,
            )
            .await?;
            let _temporary_directory = temporary_directory;
            let object_count = bundle.objects.len();

            let client = ClientBuilder::new()
                .timeout(Duration::from_secs(300))
                .build()?;
            let release_response = client
                .get(GITHUB_BACKUP_RELEASE_URL)
                .bearer_auth(&github_token)
                .header("Accept", "application/vnd.github+json")
                .header("X-GitHub-Api-Version", "2022-11-28")
                .header("User-Agent", "play-server-backup")
                .send()
                .await?;
            let release_status = release_response.status();
            if !release_status.is_success() {
                let body = release_response.text().await?;
                bail!(
                    "failed to load the GitHub backup release ({}): {}",
                    release_status,
                    body
                );
            }

            let release_info: Value = release_response.json().await?;
            let upload_url_template = release_info["upload_url"]
                .as_str()
                .ok_or_else(|| anyhow!("Upload URL not found in release info"))?
                .to_string();
            let release_assets = release_info["assets"]
                .as_array()
                .cloned()
                .unwrap_or_default();
            let object_names: HashSet<String> = bundle
                .objects
                .iter()
                .map(|object| {
                    object
                        .path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .map(str::to_string)
                        .ok_or_else(|| anyhow!("SafeBox SDK returned an invalid object path"))
                })
                .collect::<anyhow::Result<_>>()?;
            let existing_assets: HashMap<String, String> = release_assets
                .iter()
                .filter_map(|asset| {
                    Some((
                        asset["name"].as_str()?.to_string(),
                        asset["id"].as_u64()?.to_string(),
                    ))
                })
                .collect();

            let already_exists = object_names
                .iter()
                .all(|name| existing_assets.contains_key(name));
            if !already_exists {
                // Repair a previous partial publication before uploading the
                // complete Bundle. Canonical SBOX names are content-addressed.
                for name in object_names
                    .iter()
                    .filter(|name| existing_assets.contains_key(*name))
                {
                    let id = &existing_assets[name];
                    let delete_response = client
                        .delete(format!(
                            "https://api.github.com/repos/zhouzhipeng/play/releases/assets/{}",
                            id
                        ))
                        .bearer_auth(&github_token)
                        .header("Accept", "application/vnd.github+json")
                        .header("X-GitHub-Api-Version", "2022-11-28")
                        .header("User-Agent", "play-server-backup")
                        .send()
                        .await?;
                    if !delete_response.status().is_success() {
                        bail!("failed to remove incomplete GitHub backup asset {}", name);
                    }
                }

                // Continuation shards are published before shard zero, which is
                // the root/publication point of a multipart SBOX Bundle.
                let upload_order = bundle
                    .objects
                    .iter()
                    .skip(1)
                    .chain(bundle.objects.iter().take(1));
                for object in upload_order {
                    let basename = object
                        .path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .ok_or_else(|| anyhow!("SafeBox SDK returned an invalid object path"))?;
                    let upload_base = upload_url_template
                        .split_once('{')
                        .map_or(upload_url_template.as_str(), |(base, _)| base);
                    let mut upload_url = Url::parse(upload_base)?;
                    upload_url
                        .query_pairs_mut()
                        .append_pair("name", basename)
                        .append_pair("label", &backup_label);

                    let file = tokio::fs::File::open(&object.path).await?;
                    let content_length = file.metadata().await?.len();
                    let stream =
                        FramedRead::new(file, BytesCodec::new()).map_ok(|bytes| bytes.freeze());
                    let upload_response = client
                        .post(upload_url)
                        .bearer_auth(&github_token)
                        .header("Accept", "application/vnd.github+json")
                        .header("X-GitHub-Api-Version", "2022-11-28")
                        .header("User-Agent", "play-server-backup")
                        .header(header::CONTENT_TYPE, SBOX_DOWNLOAD_CONTENT_TYPE)
                        .header(header::CONTENT_LENGTH, content_length)
                        .body(reqwest::Body::wrap_stream(stream))
                        .send()
                        .await?;

                    if !upload_response.status().is_success() {
                        let status = upload_response.status();
                        let body = upload_response.text().await?;
                        bail!(
                            "failed to upload SBOX object {} to GitHub ({}): {}",
                            basename,
                            status,
                            body
                        );
                    }
                }
            }

            // Keep the latest ten logical backups, deleting every shard in an
            // expired group rather than counting multipart objects separately.
            let assets_response = client
                .get(GITHUB_BACKUP_RELEASE_URL)
                .bearer_auth(&github_token)
                .header("Accept", "application/vnd.github+json")
                .header("X-GitHub-Api-Version", "2022-11-28")
                .header("User-Agent", "play-server-backup")
                .send()
                .await?;
            if assets_response.status().is_success() {
                let release_data: Value = assets_response.json().await?;
                let assets = release_data["assets"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default();
                for (name, id) in expired_sbox_backup_assets(&assets, 10) {
                    let delete_response = client
                        .delete(format!(
                            "https://api.github.com/repos/zhouzhipeng/play/releases/assets/{}",
                            id
                        ))
                        .bearer_auth(&github_token)
                        .header("Accept", "application/vnd.github+json")
                        .header("X-GitHub-Api-Version", "2022-11-28")
                        .header("User-Agent", "play-server-backup")
                        .send()
                        .await?;
                    if delete_response.status().is_success() {
                        info!("Deleted old SBOX backup object: {}", name);
                    } else {
                        error!("Failed to delete old SBOX backup object: {}", name);
                    }
                }
            }

            let message = if already_exists {
                format!(
                    "SBOX backup {} already exists in GitHub releases ({} object(s))",
                    backup_label, object_count
                )
            } else {
                format!(
                    "SBOX backup {} successfully uploaded to GitHub releases ({} object(s))",
                    backup_label, object_count
                )
            };
            Ok::<String, anyhow::Error>(message)
        }
        .await;

        match result {
            Ok(msg) => info!("Cloud backup success: {}", msg),
            Err(e) => {
                error!("Cloud backup failed: {}", e);
                let sender = urlencoding::encode("cloud backup error").into_owned();
                let title = urlencoding::encode(&format!("Backup failed: {}", e)).into_owned();
                let _ = reqwest::get(format!("{}/{}/{}", mail_notify_url, sender, title)).await;
            }
        }
    });

    Ok(
        "SBOX backup to cloud started in background. You will receive a notification when complete."
            .to_string(),
    )
}

async fn one_key_change_ip(s: S) -> R<String> {
    let config = s.config.one_key_change_ip.clone();
    validate_one_key_change_ip_config(&config)?;

    let mail_notify_url = s.config.misc_config.mail_notify_url.clone();
    let data_dir = PathBuf::from(env::var(DATA_DIR)?);
    let run_guard = OneKeyChangeIpRunGuard::try_acquire()?;

    tokio::spawn(async move {
        let _run_guard = run_guard;
        match run_one_key_change_ip_task(config, mail_notify_url.clone(), data_dir).await {
            Ok(result) => {
                info!(
                    "one-key-change-ip completed: old_static_ip_name={:?}, old_ip={:?}, new_static_ip_name={}, new_ip={}",
                    result.old_static_ip_name,
                    result.old_ip,
                    result.new_static_ip_name,
                    result.new_ip
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

    let static_ips_after_attach = lightsail_client.get_static_ips().await?;
    let unused_static_ip_names =
        unused_static_ip_names(&static_ips_after_attach, &new_static_ip_name);
    app_push(
        &mail_notify_url,
        "one-key-change-ip",
        &format!(
            "queried static IP cleanup inventory; {} unused static IP(s) found",
            unused_static_ip_names.len()
        ),
    )
    .await;

    for unused_static_ip_name in unused_static_ip_names {
        lightsail_client
            .release_static_ip(&unused_static_ip_name)
            .await?;
        app_push(
            &mail_notify_url,
            "one-key-change-ip",
            &format!("released unused static IP {}", unused_static_ip_name),
        )
        .await;
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
        match self
            .lightsail_api(
                "AllocateStaticIp",
                json!({ "staticIpName": static_ip_name }),
            )
            .await
        {
            Ok(_) => Ok(()),
            Err(error) if is_lightsail_name_exists_error(&error, static_ip_name) => {
                info!(
                    "Lightsail static IP {} already exists after AllocateStaticIp; continuing",
                    static_ip_name
                );
                Ok(())
            }
            Err(error) => Err(error),
        }
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

fn is_lightsail_name_exists_error(error: &anyhow::Error, static_ip_name: &str) -> bool {
    let Some(api_error) = error.downcast_ref::<ApiResponseError>() else {
        return false;
    };

    if api_error.description != "Lightsail AllocateStaticIp"
        || api_error.status != StatusCode::BAD_REQUEST
    {
        return false;
    }

    let Ok(body) = serde_json::from_str::<Value>(&api_error.body) else {
        return false;
    };

    let code_matches = body.get("code").and_then(Value::as_str) == Some("NameExists");
    let message_matches = body
        .get("message")
        .and_then(Value::as_str)
        .map(|message| message.contains(static_ip_name))
        .unwrap_or(false);

    code_matches && message_matches
}

fn unused_static_ip_names(
    static_ips: &[StaticIpInfo],
    protected_static_ip_name: &str,
) -> Vec<String> {
    static_ips
        .iter()
        .filter(|static_ip| static_ip.name != protected_static_ip_name)
        .filter(|static_ip| !static_ip.is_attached())
        .map(|static_ip| static_ip.name.clone())
        .collect()
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
        return Err(ApiResponseError {
            description: description.to_string(),
            status,
            body: text,
        }
        .into());
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
    let identity = parse_sbox_public_identity(&s.config.backup_config.sbox_public_key)?;
    let data_directory = PathBuf::from(env::var(DATA_DIR)?);
    let files_path = data_directory.join("files");
    let config_file_path = data_directory.join("config.toml");
    let temporary_directory = tempfile::Builder::new()
        .prefix("play-sbox-download-")
        .tempdir_in(data_directory)?;
    let timestamp = current_timestamp!();
    let date_str = timestamp_to_date_str!(timestamp);
    let backup_label = format!("play_backup_{}", date_str);
    let original_name = format!("{}.zip", backup_label);
    let (temporary_directory, bundle) = create_sbox_backup_off_thread(
        identity,
        s.config.database.url.clone(),
        files_path,
        config_file_path,
        temporary_directory,
        original_name,
    )
    .await?;
    let (temporary_directory, download_path, download_name, content_type) =
        prepare_sbox_download_off_thread(temporary_directory, bundle, backup_label).await?;

    Ok(stream_sbox_download(
        &download_path,
        &download_name,
        content_type,
        temporary_directory,
    )
    .await?)
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

    #[test]
    fn sbox_backup_requires_a_public_identity() {
        let error = parse_sbox_public_identity("  ").unwrap_err();

        assert!(error.to_string().contains("backup_config.sbox_public_key"));
    }

    #[test]
    fn creates_zip_backup_as_a_canonical_sbox_object() -> anyhow::Result<()> {
        let root = tempfile::tempdir()?;
        let source = root.path().join("source");
        let files_path = source.join("files");
        fs::create_dir_all(&files_path)?;
        fs::write(files_path.join("note.txt"), b"SafeBox integration test")?;
        let database_path = source.join("play.db");
        fs::write(&database_path, b"sqlite fixture")?;
        let config_path = source.join("config.toml");
        fs::write(&config_path, b"server_port = 3000")?;
        let output = root.path().join("output");
        fs::create_dir(&output)?;

        let legacy_identity =
            PublicIdentity::from_json(include_str!("test_sbox_public_identity.json"))?;
        let compact_key = legacy_identity.to_compact();
        let identity = parse_sbox_public_identity(&compact_key)?;
        assert_eq!(identity.spki_der(), legacy_identity.spki_der());
        assert_eq!(
            parse_sbox_public_identity(include_str!("test_sbox_public_identity.json"))?.spki_der(),
            identity.spki_der(),
        );
        let mut bundle = create_sbox_backup(
            &identity,
            &format!("sqlite://{}", database_path.display()),
            &files_path,
            &config_path,
            &output,
            "play_backup_test.zip",
        )?;

        assert_eq!(bundle.manifest.original_name, "play_backup_test.zip");
        assert_eq!(bundle.manifest.media_type, "application/zip");
        assert_eq!(bundle.objects.len(), 1);
        let object = fs::read(&bundle.objects[0].path)?;
        assert_eq!(&object[..8], b"SBOX\r\n\x1a\n");
        assert_eq!(
            bundle.objects[0]
                .path
                .extension()
                .and_then(|extension| extension.to_str()),
            Some("sbox")
        );

        let bundle_id = bundle.manifest.bundle_id.clone();
        let root_path = output.join(format!("{}_0_2.sbox", bundle_id));
        let continuation_path = output.join(format!("{}_1_2.sbox", bundle_id));
        fs::write(&root_path, b"root shard")?;
        fs::write(&continuation_path, b"continuation shard")?;
        bundle.objects = vec![
            safebox_sdk::WrittenObject {
                shard_index: 0,
                path: root_path,
            },
            safebox_sdk::WrittenObject {
                shard_index: 1,
                path: continuation_path,
            },
        ];

        let (archive_path, archive_name, content_type) =
            prepare_sbox_download(&bundle, &output, "play_backup_test")?;
        assert_eq!(archive_name, "play_backup_test_sbox_bundle.zip");
        assert_eq!(content_type, "application/zip");
        let mut archive = ZipArchive::new(File::open(archive_path)?)?;
        let archived_names = (0..archive.len())
            .map(|index| archive.by_index(index).map(|file| file.name().to_string()))
            .collect::<Result<Vec<_>, _>>()?;
        assert_eq!(
            archived_names,
            vec![
                format!("{}_0_2.sbox", bundle_id),
                format!("{}_1_2.sbox", bundle_id),
            ]
        );
        Ok(())
    }

    #[test]
    fn sbox_backup_retention_keeps_complete_multipart_groups() {
        let assets = vec![
            json!({"name":"new_0_2.sbox","label":"play_backup_2026-08-24-03","id":31,"created_at":"2026-08-24T03:00:00Z"}),
            json!({"name":"new_1_2.sbox","label":"play_backup_2026-08-24-03","id":32,"created_at":"2026-08-24T03:00:01Z"}),
            json!({"name":"middle.sbox","label":"play_backup_2026-08-24-02","id":21,"created_at":"2026-08-24T02:00:00Z"}),
            json!({"name":"old_0_2.sbox","label":"play_backup_2026-08-24-01","id":11,"created_at":"2026-08-24T01:00:00Z"}),
            json!({"name":"old_1_2.sbox","label":"play_backup_2026-08-24-01","id":12,"created_at":"2026-08-24T01:00:01Z"}),
            json!({"name":"legacy.zip","label":"play_backup_legacy","id":1,"created_at":"2020-01-01T00:00:00Z"}),
            json!({"name":"unrelated.sbox","label":"other","id":2,"created_at":"2020-01-01T00:00:00Z"}),
        ];

        let mut expired = expired_sbox_backup_assets(&assets, 2);
        expired.sort();

        assert_eq!(
            expired,
            vec![
                ("old_0_2.sbox".to_string(), "11".to_string()),
                ("old_1_2.sbox".to_string(), "12".to_string()),
            ]
        );
    }

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

    #[test]
    fn detects_lightsail_allocate_name_exists_error() {
        let error = anyhow!(ApiResponseError {
            description: "Lightsail AllocateStaticIp".to_string(),
            status: StatusCode::BAD_REQUEST,
            body: r#"{"__type":"InvalidInputException","code":"NameExists","message":"Some names are already in use: Debian-1-ip-20260627234106"}"#.to_string(),
        });

        assert!(is_lightsail_name_exists_error(
            &error,
            "Debian-1-ip-20260627234106"
        ));
        assert!(!is_lightsail_name_exists_error(
            &error,
            "Debian-1-ip-20260627234107"
        ));
    }

    #[test]
    fn one_key_change_ip_run_guard_rejects_concurrent_run() -> anyhow::Result<()> {
        let guard = OneKeyChangeIpRunGuard::try_acquire()?;

        let error = OneKeyChangeIpRunGuard::try_acquire().unwrap_err();
        assert!(error.to_string().contains("already running"));

        drop(guard);
        let _guard = OneKeyChangeIpRunGuard::try_acquire()?;
        Ok(())
    }

    #[test]
    fn unused_static_ip_names_excludes_attached_and_protected_static_ips() {
        let static_ips = vec![
            StaticIpInfo {
                name: "attached-to-current".to_string(),
                ip_address: "203.0.113.10".to_string(),
                attached_to: Some("Debian-1".to_string()),
            },
            StaticIpInfo {
                name: "unused-old".to_string(),
                ip_address: "203.0.113.11".to_string(),
                attached_to: None,
            },
            StaticIpInfo {
                name: "protected-new".to_string(),
                ip_address: "203.0.113.12".to_string(),
                attached_to: None,
            },
            StaticIpInfo {
                name: "empty-attached-to".to_string(),
                ip_address: "203.0.113.13".to_string(),
                attached_to: Some(" ".to_string()),
            },
        ];

        let names = unused_static_ip_names(&static_ips, "protected-new");

        assert_eq!(
            names,
            vec!["unused-old".to_string(), "empty-attached-to".to_string()]
        );
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
