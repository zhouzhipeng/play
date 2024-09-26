use tokio::runtime::Runtime;
use play_abi::http_abi::*;
use play_abi::{async_request_handler, request_handler};

// 异步处理函数
// async fn handle_request_impl(request: HttpRequest) -> anyhow::Result<HttpResponse> {
//     // 模拟一些异步操作
//     println!("Request handled request : {:?}", request);
//
//     let response = reqwest::get("https://crab.rs").await?.text().await?;
//     println!("response : {}", response);
//
//     Ok(HttpResponse{
//         headers: Default::default(),
//         body: "abcdefssss".to_string(),
//         status_code: 200,
//     })
// }
//
// async_request_handler!(handle_request_impl);


fn handle_request_impl(request: HttpRequest) -> anyhow::Result<HttpResponse> {
    // 模拟一些异步操作
    println!("Request handled request : {:?}", request);

    // let response = reqwest::get("https://crab.rs").await?.text().await?;
    // println!("response : {}", response);

    Ok(HttpResponse {
        headers: Default::default(),
        body: "sss".to_string(),
        status_code: 200,
    })
}

request_handler!(handle_request_impl);