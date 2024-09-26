use tokio::runtime::Runtime;
use play_abi::http_abi::*;
use play_abi::{async_request_handler, c_char_to_string, request_handler, string_to_c_char};

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
        is_success: true,
    })
}

// request_handler!(handle_request_impl);


#[no_mangle]
pub extern "C" fn handle_request(request: *const std::os::raw::c_char) -> *const std::os::raw::c_char {
    let name = c_char_to_string(request);
    let request: HttpRequest = serde_json::from_str(&name).unwrap();

    let response = handle_request_impl(request).unwrap_or(HttpResponse {
        headers: Default::default(),
        body: "error".to_string(),
        status_code: 0,
        is_success: false,
    });
    let result = serde_json::to_string(&response).unwrap();
    string_to_c_char(&result)
}