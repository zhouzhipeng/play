use std::time::Duration;
use axum::extract::Query;
use axum::response::Html;
use serde::Deserialize;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tracing::{error, info};
 use futures_util::StreamExt;
use crate::{ensure, HTML, R, S};
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
    #[serde(default)]
    header : String,
}

async fn download_remote(s: S,  Query(req): Query<DownloadRemoteReq>) -> R<String> {
    tokio::spawn(async move{
        let r = download_in_background(req).await;
        info!("download_in_background result : {:?}", r);
    });



    Ok("ok.".to_string())
}



async fn download_in_background(req : DownloadRemoteReq)->anyhow::Result<()>{
    // URL地址，从这里下载文件

    let url = req.remote_url;
    // 目标文件路径，把下载的文件保存到这里
    let file_path = req.local_file;



    // 发送请求下载文件
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()?;


    let response =  if req.header.is_empty(){
        client.get(url).send().await?
    }else{
        let vals: Vec<&str> = req.header.split("=").collect();
        ensure!(vals.len()==2, "header is configured wrong!");
        client.get(url).header(vals[0], vals[1]).send().await?
    };


    // 确保HTTP请求成功
    if response.status().is_success() {
        let mut file = File::create(file_path).await?;

        // 使用流式传输，逐块写入文件
        let mut stream = response.bytes_stream();

        while let Some(item) = stream.next().await {
            let chunk = item.expect("Failed to read chunk");
            file.write_all(&chunk).await?;
        }

        info!("File downloaded successfully.");
    } else {
        error!("File download error: {}", response.status());
        println!("download response: {}", response.text().await?);

    }

    Ok(())
}


#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_download_config() {
        let r = download_in_background(DownloadRemoteReq{
            remote_url: "https://zhouzhipeng.com/download-config".to_string(),
            local_file: "/Users/zhouzhipeng/Library/Mobile Documents/com~apple~CloudDocs/big_mac_gogo_backups/play.conf.txt".to_string(),
            header: "X-Browser-Fingerprint=f05ec5b4e899848b1e686aebbcb76cbb2e9d6a41f1064afa09ab874616a9b7af".to_string(),
        }).await;

        println!("{:?}", r);
    }
    #[tokio::test]
    async fn test_download_db() {
        let r = download_in_background(DownloadRemoteReq{
            remote_url: "https://zhouzhipeng.com/download-db".to_string(),
            local_file: "/Users/zhouzhipeng/Library/Mobile Documents/com~apple~CloudDocs/big_mac_gogo_backups/play.db".to_string(),
            header: "X-Browser-Fingerprint=111".to_string(),
        }).await;

        println!("{:?}", r);
    }
    #[tokio::test]
    async fn test_create_file() ->anyhow::Result<()>{
        let local_file = "/Users/zhouzhipeng/Library/Mobile Documents/com~apple~CloudDocs/big_mac_gogo_backups/play.conf.txt";
        // let local_file = "/Users/zhouzhipeng/Downloads/play.conf.txt";


        // let file = std::fs::File::create(local_file)?;
        let  file = File::create(local_file).await?;

        println!("{:?}", file);
        Ok(())
    }
}