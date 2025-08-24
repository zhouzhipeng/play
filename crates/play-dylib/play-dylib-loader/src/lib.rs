use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use anyhow::{bail, ensure, Context};
use libloading::{Library, Symbol};
use tokio::fs;
use log::{error, info, warn};
use tempfile::Builder;
pub use play_dylib_abi::http_abi::*;
pub use play_dylib_abi::HostContext;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::Duration;
use std::panic::{self, AssertUnwindSafe};
use std::sync::{Arc, LazyLock, OnceLock};
use dashmap::DashMap;
use tokio::sync::OnceCell;
use tokio::task::JoinHandle;
use play_dylib_abi::server_abi::{RunFn, RUN_FN_NAME};

// Global counter for generating unique request IDs
pub static REQUEST_ID_COUNTER: AtomicI64 = AtomicI64::new(0);

// Store for plugin requests and responses
pub static PLUGIN_REQUEST_STORE: LazyLock<DashMap<i64, HttpRequest>> = LazyLock::new(|| DashMap::new());
pub static PLUGIN_RESPONSE_STORE: LazyLock<DashMap<i64, HttpResponse>> = LazyLock::new(|| DashMap::new());

/// Generate a new unique request ID
pub fn generate_request_id() -> i64 {
    REQUEST_ID_COUNTER.fetch_add(1, Ordering::SeqCst)
}

/// Store a request with the given ID
pub fn store_request(request_id: i64, request: HttpRequest) {
    PLUGIN_REQUEST_STORE.insert(request_id, request);
}

/// Get a request by ID
pub fn get_request(request_id: i64) -> Option<HttpRequest> {
    PLUGIN_REQUEST_STORE.get(&request_id).map(|r| r.clone())
}

/// Remove a request by ID
pub fn remove_request(request_id: i64) -> Option<HttpRequest> {
    PLUGIN_REQUEST_STORE.remove(&request_id).map(|(_, v)| v)
}

/// Store a response with the given ID
pub fn store_response(request_id: i64, response: HttpResponse) {
    PLUGIN_RESPONSE_STORE.insert(request_id, response);
}

/// Get and remove a response by ID
pub fn take_response(request_id: i64) -> Option<HttpResponse> {
    PLUGIN_RESPONSE_STORE.remove(&request_id).map(|(_, v)| v)
}
pub async fn load_and_run_server(dylib_path: &str, host_context: HostContext) -> anyhow::Result<()> {
    ensure!(fs::try_exists(dylib_path).await?);
    info!("load_and_run_server path: {}", dylib_path);

    let copy_path = dylib_path.to_string();
    let _: JoinHandle<anyhow::Result<()>> = tokio::spawn(async move {
        unsafe {
            // 加载动态库
            let lib = Library::new(&copy_path)?;
            info!("load_and_run_server lib load ok. path: {}", copy_path);
            let run: Symbol<RunFn> = lib.get(RUN_FN_NAME.as_ref())?;

            // 设置环境变量供插件使用
            std::env::set_var("HOST", &host_context.host_url);
            std::env::set_var("PLUGIN_PREFIX_URL", &host_context.plugin_prefix_url);
            std::env::set_var("DATA_DIR", &host_context.data_dir);
            
            // 如果有配置内容，写入固定路径的临时文件
            if let Some(config) = &host_context.config_text {
                let config_dir = std::env::temp_dir().join("play-dylib-configs");
                std::fs::create_dir_all(&config_dir)?;
                let config_file_path = config_dir.join(format!("config_{}.toml", 
                    std::process::id()));
                std::fs::write(&config_file_path, config)?;
                std::env::set_var("CONFIG_FILE_PATH", &config_file_path);
                
                info!("Config written to: {:?}", config_file_path);
            }

            // 调用新的简化接口（无参数）
            run();
            
            warn!("run exited, dylib_path: {}", copy_path);

            drop(lib);
            Ok(())
        }
    });

    Ok(())
}



// Plugin library cache
static PLUGIN_CACHE: LazyLock<DashMap<String,Arc<PluginLib>>> = LazyLock::new(||{
    DashMap::new()
});

struct PluginLib{
    library: Library,
}

// Public functions to manage plugin cache
pub fn clear_plugin_cache() {
    PLUGIN_CACHE.clear();
    info!("Plugin library cache cleared");
}

pub fn remove_plugin_from_cache(lib_path: &str) {
    if PLUGIN_CACHE.remove(lib_path).is_some() {
        info!("Removed plugin from cache: {}", lib_path);
    }
}

pub fn get_cached_plugin_count() -> usize {
    PLUGIN_CACHE.len()
}


unsafe fn run_plugin_with_id(lib_path: &str, request_id: i64) -> anyhow::Result<()> {
    info!("run_plugin_with_id begin path: {}, request_id: {}", lib_path, request_id);

    // Try to use cached library first
    let lib = match PLUGIN_CACHE.get(lib_path) {
        Some(cached) => {
            info!("Using cached library for: {}", lib_path);
            cached.clone()
        },
        None => {
            info!("Loading new library: {}", lib_path);
            let lib = Library::new(&lib_path)?;
            let plugin_lib = Arc::new(PluginLib { library: lib });
            PLUGIN_CACHE.insert(lib_path.to_string(), plugin_lib.clone());
            plugin_lib
        }
    };
    
    let handle_request: Symbol<HandleRequestFn> = lib.library.get(HANDLE_REQUEST_FN_NAME.as_ref())
        .context("`handle_request` method not found.")?;

    // Simply call the plugin with the request_id
    handle_request(request_id);

    info!("run_plugin_with_id finish path: {}", lib_path);
    // Note: We don't drop the lib anymore since it's cached
    Ok(())
}

/// Coordinated load and run function that uses internal request stores
pub async fn load_and_run_coordinated(
    dylib_path: &str,
    request: HttpRequest,
) -> anyhow::Result<HttpResponse> {
    let request_id = generate_request_id();
    let timeout = Duration::from_secs(30);
    
    // Store the request
    store_request(request_id, request);
    
    load_and_run_with_id(dylib_path, request_id, timeout).await
}

/// Internal function that works with a pre-assigned request ID
pub async fn load_and_run_with_id(
    dylib_path: &str,
    request_id: i64,
    timeout: Duration,
) -> anyhow::Result<HttpResponse> {
    ensure!(fs::try_exists(dylib_path).await?);
    info!("load_and_run_coordinated path: {}, request_id: {}", dylib_path, request_id);
    
    let dylib_path = dylib_path.to_string();
    
    // Call the plugin in a separate task
    tokio::spawn(async move {
        if let Err(e) = unsafe { run_plugin_with_id(&dylib_path, request_id) } {
            error!("Error calling plugin: {:?}", e);
        }
    });
    
    // Wait for response with timeout
    let start = std::time::Instant::now();
    
    loop {
        // Check if response is available
        if let Some(response) = take_response(request_id) {
            // Clean up request from store
            remove_request(request_id);
            return Ok(response);
        }
        
        if start.elapsed() > timeout {
            // Clean up request from store
            remove_request(request_id);
            bail!("Plugin response timeout after {:?}", timeout);
        }
        
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

// Keep the old function signature for backward compatibility
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