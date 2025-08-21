use play_dylib_abi::http_abi::*;
use play_dylib_abi::*;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Param {
    a: String,
    b: i32,
}


// 异步处理函数
async fn async_handle_request_impl(request: HttpRequest) -> anyhow::Result<HttpResponse> {
    // 模拟一些异步操作
    println!("Request handled request : {:?}", request);
    
    // 解析查询参数示例
    // let params = request.parse_query::<Param>()?;
    
    // 异步HTTP请求示例
    // let response = reqwest::get("https://crab.rs").await?.text().await?;
    // println!("response : {}", response);

    Ok(HttpResponse::text("async test response"))
}


fn handle_request_impl(request: HttpRequest) -> anyhow::Result<HttpResponse> {
    println!("Request handled request : {:?}", request);
    
    // 根据不同的URL路径返回不同响应
    if request.match_suffix("/test") {
        // 解析查询参数示例
        if let Ok(params) = request.parse_query::<Param>() {
            return Ok(HttpResponse::json(&json!({
                "parsed_params": params,
                "message": "Query parameters parsed successfully"
            })));
        }
    }
    
    // 默认响应
    Ok(HttpResponse::json(&json!({
        "name": "example",
        "age": 20,
        "url": request.url,
        "method": format!("{:?}", request.method)
    })))
}

// async_request_handler!(async_handle_request_impl);
request_handler!(handle_request_impl);

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