use play_abi::http_abi::*;
use play_abi::*;
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
    //
    // let response = reqwest::get("https://crab.rs").await?.text().await?;
    // println!("response : {}", response);

    // let params = request.parse_query::<Param>()?;
    //
    // let data_api = DataAPI::<Param>::new(&request.host_env.host_url,"test_param", None);
    // let insert_r = data_api.insert(&params).await?;
    //
    // let r = data_api.get(insert_r.id).await?;
    panic!("sdf");

    Ok(HttpResponse::text("test"))
}


fn handle_request_impl(request: HttpRequest) -> anyhow::Result<HttpResponse> {
    // 模拟一些异步操作
    println!("Request handled request : {:?}", request);

    // let params = request.parse_query::<Param>()?;

    // let response = reqwest::get("https://crab.rs").await?.text().await?;
    // panic!("zzzz");

    Ok(HttpResponse::json(&json!({
        "name":"zzsss",
        "age" : 20,
    })))
}

// async_request_handler!(async_handle_request_impl);
request_handler!(handle_request_impl);

async_run!(run_server);

async fn run_server(host_context: HostContext){
    println!("host_context : {:?}", host_context);
    loop{
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        println!("Play again in 5 seconds...");
    }
}