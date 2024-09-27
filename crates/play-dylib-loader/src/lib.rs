use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::PathBuf;
use anyhow::{bail, ensure, Context};
use libloading::{Library, Symbol};
use tokio::fs;
use log::{error, info};
use tempfile::Builder;
pub use play_abi::http_abi::*;
use play_abi::{c_char_to_string, string_to_c_char};

/// load a dylib from `dylib_path` (absolute path)
pub async fn load_and_run(dylib_path: &str, request: HttpRequest) -> anyhow::Result<HttpResponse> {
    ensure!(fs::try_exists(dylib_path).await?);
    info!("load_and_run  path : {}",dylib_path);

    //copy to a tmp folder (to mock hot-reloading)
    let source_path = PathBuf::from(dylib_path);

    // 使用 tempfile 创建一个临时目录
    let temp_dir = Builder::new().prefix("play_dylib").tempdir()?;

    // 在临时目录中创建目标文件路径
    let dest_path = temp_dir.path().join(source_path.file_name().context("tmp file error!")?);

    // 异步复制文件
    fs::copy(&source_path, &dest_path).await?;

    let copy_path = dest_path.to_string_lossy().into_owned();

    tokio::spawn(async move {
        unsafe {
            info!("load_and_run begin  path : {}",copy_path);
            // 加载动态库
            let lib = Library::new(&copy_path)?;
            info!("load_and_run lib load ok.  path : {}",copy_path);
            let handle_request: Symbol<HandleRequestFn> = lib.get(HANDLE_REQUEST_FN_NAME.as_ref())?;
            let rust_string = serde_json::to_string(&request)?;
            let request = string_to_c_char(&rust_string);
            let response = handle_request(request);


            let response = c_char_to_string(response);


            //delete temp file
            fs::remove_file(&copy_path).await?;

            // let response = unsafe { CStr::from_ptr(response).to_str().unwrap() };
            let response : HttpResponse =  serde_json::from_str(&response)?;
            info!("load_and_run finish  path : {}",copy_path);
            if let Some(error) = &response.error {
                bail!("run plugin error >> {}", error);
            }
            Ok(response)
        }
    }).await?
}

#[cfg(test)]
mod tests{
    use super::*;
    #[tokio::test]
    async fn test_load_and_run(){
        let request = HttpRequest {
            headers: Default::default(),
            query: Default::default(),
            body: "sdfd".to_string(),
            url: "sdf".to_string(),
        };
        let resp = load_and_run("/Users/zhouzhipeng/RustroverProjects/play/target/release/libplay_dylib_example.dylib", request).await;
        println!("resp >> {:?}", resp);
    }

    #[tokio::test]
    async fn test_load_and_run_in_docker(){
        let request = HttpRequest {
            headers: Default::default(),
            query: Default::default(),
            body: "sdfd".to_string(),
            url: "sdf".to_string(),
        };
        let resp = load_and_run("/app/target/release/libplay_dylib_example.so", request).await;
        println!("resp >> {:?}", resp);
    }
}