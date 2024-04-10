use std::error::Error;
use std::io;
use std::io::{Cursor, Write};
use std::path::{Component, PathBuf};
use std::time::UNIX_EPOCH;
use anyhow::anyhow;

use axum::body::{Body, Bytes, HttpBody, StreamBody};
use axum::{BoxError, Json};
use axum::extract::{Path, Query};
use axum::response::{IntoResponse, Response};
use chrono::{DateTime, Local};
use futures::Stream;
use futures_util::TryStreamExt;
use http::{header, HeaderValue, StatusCode};
use infer::Infer;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use tokio::fs;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, BufWriter};
use tokio_util::codec::{BytesCodec, FramedRead};
use tokio_util::io::StreamReader;
use tracing::info;

use shared::current_timestamp;

use crate::{data_dir, files_dir, JSON, method_router, R, return_error};
use crate::extractor::custom_file_upload::CustomFileExtractor;

method_router!(
    post : "/files/upload" -> upload_file,
    put : "/files/upload" -> upload_file,
    get : "/files/*path" -> download_file,
    get : "/files/packed" -> pack_files,
    get : "/files" -> list_files,
);


#[derive(Serialize, Debug)]
struct FileInfo {
    filename: String,
    modify_time: i64,  // 使用 i64 来存储时间戳（毫秒）
}

async fn list_files() -> JSON<Vec<FileInfo>> {
    let path = files_dir!();
    let mut files_info = Vec::new();

    if let Ok(mut entries) = fs::read_dir(path).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.is_file() {
                if let Ok(filename) = entry.file_name().into_string() {
                    if let Ok(metadata) = fs::metadata(&path).await {
                        if let Ok(modify_time) = metadata.modified() {
                            // 使用 chrono 来格式化时间
                            let modify_time = modify_time
                                .duration_since(UNIX_EPOCH)
                                .expect("Time went backwards")
                                .as_millis() as i64;  // 转换为毫秒
                            files_info.push(FileInfo {
                                filename,
                                modify_time,
                            });
                        }
                    }
                }
            }
        }
    }

    // Sort the files by modify_time, descending
    files_info.sort_by(|a, b| b.modify_time.cmp(&a.modify_time));

    Ok(Json(files_info))
}

async fn pack_files() -> R<impl IntoResponse> {
    let folder_path = files_dir!();
    let target_file = data_dir!().join("packed_files.zip");
    fs::remove_file(&target_file).await;
    zip_dir(&folder_path, &target_file)?;
    match File::open(&target_file).await {
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
use walkdir::WalkDir;
use zip::write::{FileOptions, ZipWriter};

fn zip_dir<T: AsRef<std::path::Path>>(src_dir: T, dst_file: T) -> zip::result::ZipResult<()> {
    let src_path = src_dir.as_ref();
    let dst_path = dst_file.as_ref();

    let file = std::fs::File::create(&dst_path)?;
    let walkdir = WalkDir::new(&src_path);
    let it = walkdir.into_iter();

    let mut zip = ZipWriter::new(file);
    let options = FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);

    for entry in it.filter_map(|e| e.ok()) {
        let path = entry.path();
        let name = path.strip_prefix(&src_path).unwrap();

        // 如果是文件，则添加文件到压缩包
        if path.is_file() {
            info!("Adding file: {:?}", name);
            zip.start_file(name.display().to_string(), options)?;
            let mut f = std::fs::File::open(path)?;
            std::io::copy(&mut f, &mut zip)?;
        } else if name.as_os_str().len() != 0 {
            // 如果是目录，则添加目录到压缩包
            info!("Adding directory: {:?}", name);
            zip.start_file(name.display().to_string(), options)?;
        }
    }
    zip.finish()?;
    Ok(())
}

#[derive(Deserialize, Debug, Default)]
struct UploadOption{
    #[serde(default)]
    random_name: bool
}

// #[debug_handler]
async fn upload_file(
    Query(option): Query<UploadOption>,
    body: CustomFileExtractor
) -> R<String> {
    info!("upload option : {:?}", option);
    return match body {
        CustomFileExtractor::MULTIPART(mut multipart) => {
            let mut target_path = vec![];
            while let Some(field) = multipart.next_field().await.unwrap() {
                let mut file_name = if let Some(file_name) = field.file_name() {
                    file_name.to_owned()
                } else {
                    continue;
                };

                if file_name.is_empty(){
                    //not valid upload.
                    continue;
                }

                if option.random_name{
                    let mut extension = extract_extension(&file_name);;
                    if extension.is_empty(){
                        file_name = format!("{}", current_timestamp!());
                    }else{
                        file_name = format!("{}.{}", current_timestamp!(), extension);
                    }

                    info!("new file name : {}", file_name);
                }


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

    let mime_type = mime_guess::from_path(&safe_path).first_or_text_plain();


    // Attempt to open the file
    match File::open(&safe_path).await {
        Ok(mut file) => {
            let mut contents = Vec::new();
            // Read the file contents into a buffer
            if let Ok(_) = file.read_to_end(&mut contents).await {
                // Create a response with the file contents
                let mut response = Response::builder()
                    .status(StatusCode::OK)
                    .header(
                        header::CONTENT_TYPE,
                        HeaderValue::from_str(mime_type.as_ref()).unwrap()
                    )
                    .body(Body::from(contents))
                    .expect("Failed to build response"); // Convert Vec<u8> into Body


                // You can add or modify response headers here
                // response.headers_mut().insert(
                //     "Content-Disposition",
                //     HeaderValue::from_str(&format!("attachment; filename=\"{}\"", safe_path.file_name().unwrap().to_str().unwrap())).unwrap(),
                // );

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
fn extract_extension(filename: &str) -> &str {
    let path = std::path::Path::new(filename);
    let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");
    extension
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
    use crate::{mock_server, mock_state};
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

    #[test]
    fn test_split() {
        let extension = extract_extension("sdfsdf . sddf sdfs sd.png");

        println!("{}", extension);
    }

}