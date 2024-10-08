use std::env;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::PathBuf;
use anyhow::{bail, ensure, Context};
use libloading::{Library, Symbol};
use tokio::fs;
use log::{error, info, warn};
use tempfile::Builder;
pub use play_abi::http_abi::*;
pub use play_abi::HostContext;
use play_abi::{c_char_to_string, string_to_c_char, string_to_c_char_mut};
use std::panic::{self, AssertUnwindSafe};
use tokio::task::JoinHandle;
use play_abi::server_abi::{RunFn, RUN_FN_NAME};

/// load a dylib from `dylib_path` (absolute path)
pub async fn load_and_run(dylib_path: &str, request: HttpRequest) -> anyhow::Result<HttpResponse> {
    ensure!(fs::try_exists(dylib_path).await?);
    info!("load_and_run  path : {}",dylib_path);
    let mut copy_path = dylib_path.to_string();
    #[cfg(feature = "hot-reloading")]
    let tmp_dir={
        //copy to a tmp folder (to mock hot-reloading)
        let source_path = PathBuf::from(dylib_path);

        // 使用 tempfile 创建一个临时目录
        let temp_dir = Builder::new().prefix("play_dylib").tempdir()?;

        // 在临时目录中创建目标文件路径
        let dest_path = temp_dir.path().join(source_path.file_name().context("tmp file error!")?);

        // 异步复制文件
        fs::copy(&source_path, &dest_path).await?;

        copy_path =  dest_path.to_string_lossy().into_owned();
        temp_dir
    };




    let copy_path_clone = copy_path.clone();

    let result = tokio::spawn(async move {
        unsafe {
            run_plugin(&copy_path_clone, request)
        }
    }).await?;


    result
}
pub async fn load_and_run_server(dylib_path: &str, host_context: HostContext) -> anyhow::Result<()> {
    ensure!(fs::try_exists(dylib_path).await?);
    info!("load_and_run_server  path : {}",dylib_path);

    let copy_path = dylib_path.to_string();
    let _:JoinHandle<anyhow::Result<()>> = tokio::spawn(async move {
        unsafe {
            // 加载动态库
            let lib = Library::new(&copy_path)?;
            info!("load_and_run_server lib load ok.  path : {}",copy_path);
            let run: Symbol<RunFn> = lib.get(RUN_FN_NAME.as_ref())?;

            let rust_string = serde_json::to_string(&host_context)?;
            let request = string_to_c_char_mut(&rust_string);
            run(request);
            warn!("run failed? dylib_path : {}",copy_path);

            drop(lib);
            Ok(())
        }
    });

    Ok(())
}


unsafe fn run_plugin(copy_path: &str, request: HttpRequest) -> anyhow::Result<HttpResponse> {
    info!("load_and_run begin  path : {}",copy_path);
    // 加载动态库
    let lib = Library::new(&copy_path)?;
    info!("load_and_run lib load ok.  path : {}",copy_path);
    let handle_request: Symbol<HandleRequestFn> = lib.get(HANDLE_REQUEST_FN_NAME.as_ref())?;
    let free_c_string: Symbol<FreeCStringFn> = lib.get(FREE_C_STRING_FN_NAME.as_ref())?;

    let rust_string = serde_json::to_string(&request)?;
    let request = string_to_c_char_mut(&rust_string);
    let response_ptr = handle_request(request);


    let response = c_char_to_string(response_ptr);
    free_c_string(request);
    free_c_string(response_ptr);
    drop(lib);

    // let response = unsafe { CStr::from_ptr(response).to_str().unwrap() };
    let response: HttpResponse = serde_json::from_str(&response)?;
    info!("load_and_run finish  path : {}",copy_path);
    if let Some(error) = &response.error {
        bail!("run plugin error >> {}", error);
    }
    Ok(response)
}


#[cfg(test)]
mod tests {
    use play_abi::HostContext;
    use super::*;
    #[tokio::test]
    async fn test_load_and_run() {
        let request = HttpRequest {
            method: HttpMethod::GET,
            headers: Default::default(),
            query: "a=1aa&b=2".to_string(),
            body: "sdfd".to_string(),
            url: "sdf".to_string(),
            context: HostContext { host_url: "http://127.0.0.1:3000".to_string(), plugin_prefix_url: "".to_string(), data_dir: "".to_string(), config_text: None },
        };
        let resp = load_and_run("/Users/zhouzhipeng/RustroverProjects/play/target/release/libplay_dylib_example.dylib", request).await;
        println!("resp >> {:?}", resp);
    }

    #[tokio::test]
    async fn test_load_and_run_in_docker() {
        let request = HttpRequest {
            method: HttpMethod::GET,
            headers: Default::default(),
            query: Default::default(),
            body: "sdfd".to_string(),
            url: "sdf".to_string(),
            context: HostContext { host_url: "http://127.0.0.1:3000".to_string(), plugin_prefix_url: "".to_string(), data_dir: "".to_string(), config_text: None },
        };
        let resp = load_and_run("/app/target/release/libplay_dylib_example.so", request).await;
        println!("resp >> {:?}", resp);
    }
    #[tokio::test]
    async fn test_load_and_run_server() {

        let resp = load_and_run_server("/Users/zhouzhipeng/RustroverProjects/play/target/release/libplay_dylib_example.dylib", HostContext{
            host_url: "".to_string(),
            plugin_prefix_url: "".to_string(),
            data_dir: "".to_string(),
            config_text: None,
        }).await;
        tokio::time::sleep(std::time::Duration::from_secs(50)).await;
        println!("resp >> {:?}", resp);
    }
}