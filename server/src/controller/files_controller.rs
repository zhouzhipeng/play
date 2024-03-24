use std::error::Error;
use std::io;
use std::path::{Component, PathBuf};

use axum::body::{Body, Bytes, HttpBody};
use axum::BoxError;
use axum::extract::Path;
use axum::response::{IntoResponse, Response};
use futures::Stream;
use futures_util::TryStreamExt;
use http::{HeaderValue, StatusCode};
use infer::Infer;
use sqlx::Row;
use tokio::fs;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, BufWriter};
use tokio_util::io::StreamReader;
use tracing::info;

use shared::current_timestamp;

use crate::{files_dir, method_router, R, return_error};
use crate::extractor::custom_file_upload::CustomFileExtractor;

method_router!(
    post : "/files/upload" -> upload_file,
    put : "/files/upload" -> upload_file,
    get : "/files/*path" -> download_file,
);

// 定义允许的最大文件大小
const MAX_CONTENT_LENGTH: u64 = 100 * 1024 * 1024; // 10MB

// #[debug_handler]
async fn upload_file(
    body: CustomFileExtractor
) -> R<String> {
    return match body {
        CustomFileExtractor::MULTIPART(mut multipart) => {
            let mut target_path = vec![];
            while let Some(field) = multipart.next_field().await.unwrap() {
                let file_name = if let Some(file_name) = field.file_name() {
                    file_name.to_owned()
                } else {
                    continue;
                };

                stream_to_file(&file_name, field).await?;

                target_path.push(format!("/files/{}", file_name));
            }
            Ok(target_path.join(","))
        }
        CustomFileExtractor::BODYSTREAM(bodystream) => {
            let local_path = stream_to_file(&format!("{}", current_timestamp!()), bodystream).await?;
            let new_path = rename_file_with_correct_extension(&local_path).await?;
            Ok(format!("/files/{}", new_path))
        }
    };
}

async fn download_file(Path(file_path): Path<String>) -> impl IntoResponse {
    // Sanitize file path and prevent directory traversal
    let safe_path = files_dir!().join(file_path.trim_start_matches('/'));
    if safe_path.components().any(|component| component == Component::ParentDir) {
        return Err((StatusCode::FORBIDDEN, "Access denied"));
    }

    // Attempt to open the file
    match File::open(&safe_path).await {
        Ok(mut file) => {
            let mut contents = Vec::new();
            // Read the file contents into a buffer
            if let Ok(_) = file.read_to_end(&mut contents).await {
                // Create a response with the file contents
                let mut response = Response::builder()
                    .status(StatusCode::OK)
                    .body(Body::from(contents))
                    .expect("Failed to build response"); // Convert Vec<u8> into Body

                // You can add or modify response headers here
                response.headers_mut().insert(
                    "Content-Disposition",
                    HeaderValue::from_str(&format!("attachment; filename=\"{}\"", safe_path.file_name().unwrap().to_str().unwrap())).unwrap(),
                );

                Ok(response)
            } else {
                // If file reading fails
                Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to read the file"))
            }
        }
        Err(_) => {
            // If file opening fails
            Err((StatusCode::NOT_FOUND, "File not found"))
        }
    }
}


// Save a `Stream` to a file
async fn stream_to_file<S, E>(path: &str, stream: S) -> anyhow::Result<PathBuf>
    where
        S: Stream<Item=Result<Bytes, E>>,
        E: Into<BoxError>,
{

    // Convert the stream into an `AsyncRead`.
    let body_with_io_error = stream.map_err(|err| io::Error::new(io::ErrorKind::Other, err));
    let body_reader = StreamReader::new(body_with_io_error);
    futures::pin_mut!(body_reader);

    // Create the file. `File` implements `AsyncWrite`.
    let path = files_dir!().join(path);
    let mut file = BufWriter::new(File::create(&path).await?);

    // Copy the body into the file.
    tokio::io::copy(&mut body_reader, &mut file).await?;

    Ok(path)
}

async fn rename_file_with_correct_extension(path: &std::path::Path) -> anyhow::Result<String> {
    // 创建 Infer 实例
    let inferrer = Infer::new();

    // 异步地打开和读取文件
    // let mut buffer = vec![0; 4096]; // 读取文件的前 4096 字节用于类型推断
    // let mut file = File::open(path).await?;
    // let size = file.read_to_end(&mut buffer).await?;
    // if size<=0{
    //     return_error!("read file error!")
    // }
    // ;
    // 推断文件类型
    let new_file_name = if let Some(kind) = inferrer.get_from_path(path)? {
        println!("Detected type: {}", kind.mime_type());
        ;
        // 构建新的文件名
        let new_filename = match path.file_stem() {
            Some(stem) => stem.to_string_lossy().into_owned() + "." + kind.extension(),
            None => return_error!("get filename error!"), // 如果没有文件名（不太可能），就直接返回
        };
        new_filename
    } else {
        // 默认为 txt 文件
        info!("File type unknown, defaulting to .txt");
        format!("{}.txt", path.file_name().unwrap().to_str().unwrap())
    };


    let new_path = path.with_file_name(&new_file_name);

    // 重命名文件
    fs::rename(path, new_path).await?;
    Ok(new_file_name)
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_rename() -> anyhow::Result<()> {
        let path = std::path::Path::new("/Users/zhouzhipeng/RustroverProjects/play/server/output_dir/files/test");
        println!("new name : {}", rename_file_with_correct_extension(path).await?);

        Ok(())
    }

    #[test]
    fn test_pathbuf() {
        let path_buf = PathBuf::from("/some/path");
        let path_str = path_buf.display().to_string();
        println!("{}", path_str);
    }
}