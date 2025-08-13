use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::process::Command;
use std::path::PathBuf;
use tokio::task;

use super::{Tool, BoxFuture};

pub struct BilibiliDownloadTool {
    download_dir: PathBuf,
}

impl BilibiliDownloadTool {
    pub fn new() -> Self {
        let download_dir = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("bilibili_downloads");
        
        Self { download_dir }
    }
    
    pub fn with_download_dir(dir: PathBuf) -> Self {
        Self { download_dir: dir }
    }
}

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

impl Tool for BilibiliDownloadTool {
    fn name(&self) -> &str {
        "bilibili_download"
    }
    
    fn description(&self) -> &str {
        "Download videos from Bilibili in the background"
    }
    
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "The Bilibili video URL (e.g., https://www.bilibili.com/video/BV14rt1zFECj)"
                },
                "quality": {
                    "type": "string",
                    "description": "Video quality (e.g., '1080p', '720p', '480p'). Default is highest available",
                    "enum": ["1080p", "720p", "480p", "360p", "auto"],
                    "default": "auto"
                },
                "output_dir": {
                    "type": "string",
                    "description": "Custom output directory for downloaded videos"
                }
            },
            "required": ["url"]
        })
    }
    
    fn execute<'a>(&'a self, input: Value) -> BoxFuture<'a, Result<Value>> {
        Box::pin(async move {
            let input: BilibiliDownloadInput = serde_json::from_value(input)?;
            
            let video_id = extract_video_id(&input.url)?;
            
            let output_dir = input.output_dir
                .map(PathBuf::from)
                .unwrap_or_else(|| self.download_dir.clone());
            
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
        })
    }
}

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
            if !result.status.success() {
                let stderr = String::from_utf8_lossy(&result.stderr);
                tracing::error!("Failed to download video {}: {}", video_id, stderr);
                
                tracing::info!("Trying alternative download method for video {}", video_id);
                let alt_output = Command::new("you-get")
                    .arg("-o")
                    .arg(output_dir.to_string_lossy().to_string())
                    .arg("-O")
                    .arg(video_id.clone())
                    .arg(url)
                    .output();
                
                match alt_output {
                    Ok(alt_result) => {
                        if alt_result.status.success() {
                            tracing::info!("Successfully downloaded video {} using alternative method", video_id);
                        } else {
                            let alt_stderr = String::from_utf8_lossy(&alt_result.stderr);
                            tracing::error!("Alternative download also failed for {}: {}", video_id, alt_stderr);
                        }
                    }
                    Err(e) => {
                        tracing::error!("Alternative download command failed for {}: {}", video_id, e);
                    }
                }
            } else {
                tracing::info!("Successfully downloaded video {} to {:?}", video_id, output_path);
            }
        }
        Err(e) => {
            tracing::error!("Failed to execute download command for {}: {}", video_id, e);
        }
    }
    
    Ok(())
}