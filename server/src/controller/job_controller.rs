use axum::extract::Query;
use axum::response::Html;
use serde::Deserialize;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tracing::{error, info};
 use futures_util::StreamExt;
use crate::{HTML, R, S};
use crate::method_router;

method_router!(
    get : "/job/test"-> root,
    get : "/job/download-remote"-> download_remote,
);

// #[axum::debug_handler]
async fn root(s: S) -> HTML {
    Ok(Html("job done".to_string()))
}


#[derive(Deserialize)]
struct DownloadRemoteReq{
    remote_url: String,
    local_file: String,
}

async fn download_remote(s: S,  Query(req): Query<DownloadRemoteReq>) -> R<String> {
    // URL地址，从这里下载文件

    let url = &req.remote_url;
    // 目标文件路径，把下载的文件保存到这里
    let file_path = &req.local_file;

    // 发送请求下载文件
    let response = reqwest::get(url).await?;

    // 确保HTTP请求成功
    if response.status().is_success() {
        let mut file = File::create(file_path).await?;

        // 使用流式传输，逐块写入文件
        let mut stream = response.bytes_stream();

        while let Some(item) = stream.next().await {
            let chunk = item.expect("Failed to read chunk");
            file.write_all(&chunk).await.expect("Failed to write to file");
        }

        info!("File downloaded successfully.");
    } else {
        error!("File download error: {}", response.status());
    }


    Ok("ok.".to_string())
}



