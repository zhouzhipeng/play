use tokio::runtime::Runtime;
use play_abi::http_abi::{Request, Response};
use play_abi::request_handler;

// 异步处理函数
// async fn handle_request_impl(request: Request) -> anyhow::Result<Response> {
//     // 模拟一些异步操作
//     println!("Request handled request : {:?}", request);
//
//     let response = reqwest::get("https://crab.rs").await?.text().await?;
//     println!("response : {}", response);
//
//     Ok(Response{
//         headers: Default::default(),
//         body: response,
//         status_code: 200,
//     })
// }
//
// async_request_handler!(handle_request_impl);
fn handle_request_impl(request: Request) -> anyhow::Result<Response> {
    // 模拟一些异步操作
    println!("Request handled request : {:?}", request);

    // let response = reqwest::get("https://crab.rs").await?.text().await?;
    // println!("response : {}", response);

    Ok(Response{
        headers: Default::default(),
        body: "sss".to_string(),
        status_code: 200,
    })
}

request_handler!(handle_request_impl);