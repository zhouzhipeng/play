use anyhow::ensure;
use libloading::{Library, Symbol};
use tokio::fs;
pub use play_abi::http_abi::*;

/// load a dylib from `dylib_path` (absolute path)
pub async fn load_and_run(dylib_path: &str, request: HttpRequest) -> anyhow::Result<HttpResponse> {
    ensure!(fs::try_exists(dylib_path).await?);

    let copy_path = dylib_path.to_string();
    tokio::spawn(async move {
        unsafe {
            // 加载动态库
            let lib = Library::new(copy_path)?;
            let handle_request: Symbol<HandleRequestFn> = lib.get(HANDLE_REQUEST_FN_NAME.as_ref())?;
            let response = handle_request(request);
            lib.close()?;
            response
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
}