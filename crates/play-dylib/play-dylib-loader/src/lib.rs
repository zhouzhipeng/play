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
pub async fn load_and_run_server(dylib_path: &str) -> anyhow::Result<()> {
    ensure!(fs::try_exists(dylib_path).await?);
    info!("load_and_run_server path: {}", dylib_path);

    let copy_path = dylib_path.to_string();
    let _: JoinHandle<anyhow::Result<()>> = tokio::spawn(async move {
        unsafe {
            // 加载动态库
            let lib = Library::new(&copy_path)?;
            info!("load_and_run_server lib load ok. path: {}", copy_path);
            let run: Symbol<RunFn> = lib.get(RUN_FN_NAME.as_ref())?;


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
