use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::process::Command;
use std::path::PathBuf;
use tokio::task;

#[derive(Debug, Serialize, Deserialize)]
pub struct BilibiliDownloadInput {
    pub url: String,
    pub quality: Option<String>,
    pub output_dir: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BilibiliDownloadResult {
    pub success: bool,
    pub message: String,
    pub download_path: Option<String>,
    pub video_id: Option<String>,
}

crate::define_mcp_tool!(
    BilibiliDownloadTool,
    "bilibili_download",
    fields: {
        download_dir: PathBuf = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("bilibili_downloads")
    },
    |tool: &BilibiliDownloadTool, input: Value| {
        let download_dir = tool.download_dir.clone();
        async move {
            let input: BilibiliDownloadInput = serde_json::from_value(input)?;
            
            let video_id = extract_video_id(&input.url)?;
            
            let output_dir = input.output_dir
                .map(PathBuf::from)
                .unwrap_or_else(|| download_dir);
        
        std::fs::create_dir_all(&output_dir)?;
        
        let quality = input.quality.unwrap_or_else(|| "auto".to_string());
        
        let url = input.url.clone();
        let output_dir_clone = output_dir.clone();
        let video_id_clone = video_id.clone();
        
        task::spawn(async move {
            download_video_background(url, output_dir_clone, quality, video_id_clone).await
        });
        
            Ok(json!(BilibiliDownloadResult {
                success: true,
                message: format!("Started downloading video {} in background", video_id),
                download_path: Some(output_dir.to_string_lossy().to_string()),
                video_id: Some(video_id),
            }))
        }
    }
);

fn extract_video_id(url: &str) -> Result<String> {
    if url.contains("/video/") {
        let parts: Vec<&str> = url.split("/video/").collect();
        if parts.len() > 1 {
            let id_part = parts[1];
            let id = id_part.split('/').next().unwrap_or(id_part);
            let id = id.split('?').next().unwrap_or(id);
            return Ok(id.to_string());
        }
    }
    
    Err(anyhow!("Could not extract video ID from URL"))
}

async fn download_video_background(
    url: String, 
    output_dir: PathBuf, 
    quality: String,
    video_id: String
) -> Result<()> {
    let output_path = output_dir.join(format!("{}.mp4", video_id));
    
    let quality_arg = match quality.as_str() {
        "1080p" => "80",
        "720p" => "64",
        "480p" => "32",
        "360p" => "16",
        _ => "0",
    };
    
    let output = Command::new("yt-dlp")
        .arg("--format")
        .arg(format!("bestvideo[height<={}]+bestaudio/best", 
            if quality == "auto" { "9999" } else { quality_arg }))
        .arg("--merge-output-format")
        .arg("mp4")
        .arg("--output")
        .arg(output_path.to_string_lossy().to_string())
        .arg("--cookies-from-browser")
        .arg("chrome")
        .arg("--user-agent")
        .arg("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .arg(&url)
        .output();
    
    match output {
        Ok(result) => {
            if result.status.success() {
                tracing::info!("Video {} downloaded successfully", video_id);
            } else {
                let error = String::from_utf8_lossy(&result.stderr);
                tracing::error!("Failed to download video {}: {}", video_id, error);
            }
        }
        Err(e) => {
            tracing::error!("Failed to execute yt-dlp for video {}: {}", video_id, e);
        }
    }
    
    Ok(())
}