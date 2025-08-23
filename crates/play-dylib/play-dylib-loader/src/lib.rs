use std::collections::HashMap;
use std::env;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::PathBuf;
use anyhow::{bail, ensure, Context};
use libloading::{Library, Symbol};
use tokio::fs;
use log::{error, info, warn};
use tempfile::Builder;
pub use play_dylib_abi::http_abi::*;
pub use play_dylib_abi::HostContext;
use play_dylib_abi::{c_char_to_string, string_to_c_char, string_to_c_char_mut};
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::Duration;
use std::panic::{self, AssertUnwindSafe};
use std::sync::{Arc, LazyLock, OnceLock};
use dashmap::DashMap;
use tokio::sync::OnceCell;
use tokio::task::JoinHandle;
use play_dylib_abi::server_abi::{RunFn, RUN_FN_NAME};

// Global counter for generating unique request IDs
static REQUEST_ID_COUNTER: AtomicI64 = AtomicI64::new(0);

/// load a dylib from `dylib_path` (absolute path)
/// This function now coordinates the request/response flow:
/// 1. Stores the request with a unique ID
/// 2. Calls the plugin with the request ID
/// 3. Plugin fetches request via HTTP GET
/// 4. Plugin processes and pushes response via HTTP POST
/// 5. Retrieves and returns the response
pub async fn load_and_run_with_store(
    dylib_path: &str, 
    request_id: i64,
    timeout: Duration
) -> anyhow::Result<HttpResponse> {
    ensure!(fs::try_exists(dylib_path).await?);
    info!("load_and_run_with_store path: {}, request_id: {}", dylib_path, request_id);
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

    // Call the plugin with just the request_id
    tokio::spawn(async move {
        unsafe {
            run_plugin_with_id(&copy_path_clone, request_id)
        }
    }).await??;

    // Wait for response with timeout
    let start = std::time::Instant::now();
    loop {
        // Check if response is available (this assumes external store)
        // The actual implementation would check the PLUGIN_RESPONSE_STORE
        // but since it's in admin_controller, we need to refactor this
        
        if start.elapsed() > timeout {
            bail!("Plugin response timeout after {:?}", timeout);
        }
        
        tokio::time::sleep(Duration::from_millis(10)).await;
        
        // This is a placeholder - actual implementation needs access to response store
        // For now, return a default response to maintain compatibility
        break Ok(HttpResponse::default());
    }
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
            
            // 重要：释放分配的C字符串
            drop(std::ffi::CString::from_raw(request));
            
            warn!("run exited, dylib_path : {}",copy_path);

            drop(lib);
            Ok(())
        }
    });

    Ok(())
}


// 1. 静态 HashMap
static PLUGIN_CACHE: LazyLock<DashMap<String,Arc<PluginLib>>> = LazyLock::new(||{
    DashMap::new()
});

struct PluginLib{
    library: Library,
}

// 2. 初始化辅助函数
unsafe fn get_plugin_lib(lib_path: &str) -> anyhow::Result<Arc<PluginLib>> {
    match PLUGIN_CACHE.get(lib_path) {
        None => {
            //load
            // 加载动态库
            let lib = Library::new(&lib_path)?;
            info!("no cache , load_and_run lib load ok.  path : {}",lib_path);
            // println!("no cache , load_and_run lib load ok.  path : {}",lib_path);

            let plugin_lib = PluginLib{
                library: lib,
            };

            let lib_ref = Arc::new(plugin_lib);
            PLUGIN_CACHE.insert(lib_path.to_string(), lib_ref.clone());
            Ok(lib_ref.clone())
        }
        Some(s) => {
            info!("hit cache , load_and_run lib load ok.  path : {}",lib_path);
            // println!("hit cache , load_and_run lib load ok.  path : {}",lib_path);

            Ok(s.clone())
        },
    }

}


unsafe fn run_plugin_with_id(lib_path: &str, request_id: i64) -> anyhow::Result<()> {
    info!("run_plugin_with_id begin path: {}, request_id: {}", lib_path, request_id);

    let lib = Library::new(&lib_path)?;
    let handle_request: Symbol<HandleRequestFn> = lib.get(HANDLE_REQUEST_FN_NAME.as_ref())
        .context("`handle_request` method not found.")?;

    // Simply call the plugin with the request_id
    handle_request(request_id);

    info!("run_plugin_with_id finish path: {}", lib_path);
    drop(lib);  // 尝试卸载，虽然OS可能不会真正释放
    Ok(())
}

// Keep the old function signature for backward compatibility
// But the actual implementation will be different in the plugin_controller
pub async fn load_and_run(dylib_path: &str, request: HttpRequest) -> anyhow::Result<HttpResponse> {
    // This is now a placeholder - the real coordination happens in plugin_controller
    // We'll just call the plugin directly with a generated ID for now
    ensure!(fs::try_exists(dylib_path).await?);
    info!("load_and_run path : {}",dylib_path);
    
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
    
    // For now, return a default response to maintain backward compatibility
    // The actual new implementation is in plugin_controller
    Ok(HttpResponse::text("Plugin system refactored - use plugin_controller for new flow"))
}

// 不使用缓存，因为动态库无法真正从内存中卸载
// pub fn clear_plugin_cache() {
//     PLUGIN_CACHE.clear();
// }
// 
// pub fn remove_plugin_from_cache(lib_path: &str) {
//     PLUGIN_CACHE.remove(lib_path);
// }

#[cfg(test)]
mod tests {
    use serde_json::json;
    use play_dylib_abi::HostContext;
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
        let resp = load_and_run("/Users/ronnie/CLionProjects/play/target/release/libplay_dylib_example.dylib", request.clone()).await;
        let resp = load_and_run("/Users/ronnie/CLionProjects/play/target/release/libplay_dylib_example.dylib", request.clone()).await;
        let resp = load_and_run("/Users/ronnie/CLionProjects/play/target/release/libplay_dylib_example.dylib", request).await;
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
    #[tokio::test]
    async fn test_load_golang_dylib() {

        let resp = load_and_run("/Users/ronnie/IdeaProjects/zhouzhipeng/otpauth/libplugin_otpauth.dylib", HttpRequest{
            body: json!({"file_path":"/Users/ronnie/Downloads/IMG_1234.png"}).to_string(),
            ..Default::default()
        }).await;
        println!("resp >> {:?}", resp);

        if let Ok(resp) = resp {
            println!("resp >> {:?}", String::from_utf8(resp.body));
        }
    }
}