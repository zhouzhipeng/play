use play_dylib_abi::http_abi::*;
use play_dylib_abi::*;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Param {
    a: String,
    b: i32,
}


// 新的异步处理函数 - 使用 request_id
async fn async_handle_request_impl(request: HttpRequest) -> anyhow::Result<HttpResponse> {
    println!("Handling request with ID: {:?}", request);

    // 处理业务逻辑

        // 默认响应
    let response = HttpResponse::json(&json!({
            "query": request.query,
            "age": 20,
            "url": request.url,
            "method": format!("{:?}", request.method),
            "request_id": request_id
        }));

    // 将响应推送回 host
    response.push_to_host(request_id, &host_url).await?;
    println!("Response pushed for request ID: {}", request_id);
    
    Ok(())
}


// 同步版本也更新为新模式（虽然实际上需要异步来获取/推送数据）
// 注意：同步版本无法直接使用 async 函数，所以这里仅作为示例
// 实际使用中推荐使用 async_request_handler

// 使用新的异步处理器宏
async_request_handler!(async_handle_request_impl);

async_run!(run_server);

async fn run_server(host_context: HostContext){
    println!("Server started with context: {:?}", host_context);
    
    // 创建一个更高效的循环，避免内存泄漏
    let mut counter = 0u64;
    
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        counter += 1;
        println!("Server heartbeat #{} - memory efficient", counter);
        
        // 每100次循环强制进行一次垃圾回收提示
        if counter % 100 == 0 {
            // 提示运行时可以进行清理
            tokio::task::yield_now().await;
        }
    }
}