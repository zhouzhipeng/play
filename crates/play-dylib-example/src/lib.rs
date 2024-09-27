use play_abi::http_abi::*;
use play_abi::*;
use serde::Deserialize;
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
//         is_success: true,
//     })
// }
//
// async_request_handler!(handle_request_impl);


#[derive(Deserialize, Debug)]
struct Param {
    a: String,
    b: i32,
}

fn handle_request_impl(request: HttpRequest) -> anyhow::Result<HttpResponse> {
    // 模拟一些异步操作
    println!("Request handled request : {:?}", request);

    let params = request.parse_query::<Param>()?;
    // let response = reqwest::get("https://crab.rs").await?.text().await?;

    Ok(HttpResponse::text(&format!("inputeee111 : {:?}", params)))
}

request_handler!(handle_request_impl);
