use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use anyhow::anyhow;

use axum::{Form, Json, Router};
use axum::routing::post;
use reqwest::ClientBuilder;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::AppState;
use crate::controller::{HTML, JSON, render_fragment, S, Template};

pub fn init() -> Router<Arc<AppState>> {
    Router::new()
        //pages
        .route("/functions/str-joiner", post(str_joiner))
        .route("/functions/py-runner", post(py_runner))
        .route("/functions/run-http-request", post(run_http_request))
}

#[derive(Deserialize)]
struct Data{
    s : String,
}

async fn str_joiner(s: S, Form(data): Form<Data> ) -> HTML {

   render_fragment(&s, Template::DynamicTemplate{
       name: "<string>".to_string(),
       content: data.s,
   }, json!({})).await
}
async fn py_runner(s: S, Form(data): Form<Data> ) -> HTML {
    render_fragment(&s, Template::PythonCode{
        name: "<tmp_code>".to_string(),
        content: data.s,
    }, json!({})).await
}

#[derive(Deserialize)]
struct HttpRequestData{
    url : String,
    method : String,
    body : String,
    headers:  String,
}
#[derive(Serialize)]
struct HttpResponseData{
    Body : String,
    Headers : HashMap<String,String>,
    Status : u16,
    StatusMsg:  String,
}
async fn run_http_request(s: S, Form(data): Form<HttpRequestData> ) -> JSON<HttpResponseData> {
    let client = ClientBuilder::new().timeout(Duration::from_secs(3)).build()?;

    let response = match data.method.as_str(){
        "GET"=>{
            client.get(&data.url).send().await?
        }
        "DELETE"=>{
            client.delete(&data.url).send().await?
        }
        "POST"=>{
            client.post(&data.url).body(data.body).send().await?
        }
        "PUT"=>{
            client.put(&data.url).body(data.body).send().await?
        }
        _=> return Err(anyhow!("Err >> method : {} not supported", data.method).into())
    };

    let mut headers = HashMap::new();
    for (k,v) in response.headers() {
        headers.insert(k.to_string(),v.to_str().unwrap_or("").to_string());
    }
    let status_code = response.status();
    let resp_body = response.text().await?;
    Ok(Json(HttpResponseData{
        Body: resp_body,
        Headers: headers,
        Status: status_code.as_u16(),
        StatusMsg: status_code.to_string(),
    }))
}
