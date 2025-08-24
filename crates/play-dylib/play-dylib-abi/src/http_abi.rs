use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use anyhow::Context;
use reqwest::Client;
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use crate::HostContext;


pub type HandleRequestFn = unsafe extern "C" fn(i64);
pub const HANDLE_REQUEST_FN_NAME: &'static str = "handle_request";

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Default)]
pub enum HttpMethod {
    #[default]
    GET,
    POST,
    PUT,
    DELETE
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct HttpRequest {
    pub method:HttpMethod,
    pub headers: HashMap<String, String>,
    pub query: String,
    pub url: String,
    pub body: String,
    pub context: HostContext,
}




impl HttpRequest {
    pub async fn fetch_from_host(request_id: i64, host_url: &str) -> anyhow::Result<Self> {
        let client = Client::new();
        let url = format!("{}/admin/get-request-info?request_id={}", host_url, request_id);
        let response = client.get(&url).send().await?;
        let request: HttpRequest = response.json().await?;
        Ok(request)
    }

    pub fn parse_query<T: DeserializeOwned>(&self) -> anyhow::Result<T> {
        let p: T = serde_urlencoded::from_str(&self.query).context("parse query str error!")?;
        Ok(p)
    }
    pub fn parse_body_form<T: DeserializeOwned>(&self) -> anyhow::Result<T> {
        let p: T = serde_urlencoded::from_str(&self.body).context("parse body str error!")?;
        Ok(p)
    }
    pub fn parse_body_json<T: DeserializeOwned>(&self) -> anyhow::Result<T> {
        let p: T = serde_json::from_str(&self.body).context("parse body str error!")?;
        Ok(p)
    }
    pub fn get_suffix_url(&self)->String{
        self.url.strip_prefix(&self.context.plugin_prefix_url).unwrap().to_string()
    }

    pub fn match_suffix(&self, suffix: &str)->bool{
        self.get_suffix_url().eq(suffix)
    }
    pub fn match_suffix_default(&self)->bool{
        self.get_suffix_url().eq("")
    }
    
    pub async fn render_template(&self, raw: &str, data: Value) -> anyhow::Result<String> {
        self.context.render_template(raw, data).await
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct HttpResponse {
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
    #[serde(default = "default_status_code")]
    pub status_code: u16,
    /// used to mark current plugin running is success or not (shoule left None for normal bussiness logic)
    /// will be used automatically when `?` triggered in your logic.
    pub error: Option<String>,
}

fn default_status_code() -> u16 {
    200
}

fn print_error(err: &anyhow::Error) -> String {
    // println!("Error: {}", err);
    let mut source = err.source();
    let mut level = 0;
    let mut error_str = format!("[plugin error] {}\n", err);
    while let Some(cause) = source {
        // println!("Cause {}: {}", level, cause);
        error_str.push_str(&format!("Cause by : {} \n", cause));
        source = cause.source();
        level += 1;
        if level >= 5 {
            break;
        }
    }

    error_str
}


impl HttpResponse {
    pub async fn push_to_host(&self, request_id: i64, host_url: &str) -> anyhow::Result<()> {
        let client = Client::new();
        let url = format!("{}/admin/push-response-info?request_id={}", host_url, request_id);
        client.post(&url)
            .json(self)
            .send()
            .await?;
        Ok(())
    }
    
    pub fn from_anyhow(r: anyhow::Result<Self>) -> HttpResponse {
        r.unwrap_or_else(|e| {
            HttpResponse {
                error: Some(print_error(&e)),
                ..Self::default()
            }
        })
    }
    pub fn from_panic_error(err: String) -> HttpResponse {
        HttpResponse {
            error: Some(err),
            ..Self::default()
        }
    }

    pub fn text(body: &str) -> Self {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "text/plain;charset=UTF-8".to_string());
        Self {
            headers,
            body: body.as_bytes().to_vec(),
            status_code: default_status_code(),
            ..Self::default()
        }
    }
    pub fn bytes(body: &[u8], content_type: &str) -> Self {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), content_type.to_string());
        Self {
            headers,
            body: body.to_vec(),
            status_code: default_status_code(),
            ..Self::default()
        }
    }
    pub fn page_404() -> Self {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "text/plain;charset=UTF-8".to_string());
        Self {
            headers,
            body: "page not found".as_bytes().to_vec(),
            status_code: 404,
            ..Self::default()
        }
    }
    pub fn html(body: &str) -> Self {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "text/html;charset=UTF-8".to_string());
        Self {
            headers,
            body: body.as_bytes().to_vec(),
            status_code: default_status_code(),
            ..Self::default()
        }
    }
    pub fn json<T: Serialize>(body: &T) -> Self {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json;charset=UTF-8".to_string());
        Self {
            headers,
            body: serde_json::to_string(body).unwrap().as_bytes().to_vec(),
            status_code: default_status_code(),
            ..Self::default()
        }
    }
}



/// needs tokio runtime.
/// usage: `async_request_handler!(handle_request_impl);`
/// ```rust
/// async fn handle_request_impl(request: HttpRequest) -> anyhow::Result<HttpResponse>
/// ```
/// 
/// The macro handles all the host communication:
/// 1. Fetches request from host using request_id
/// 2. Calls your function with the HttpRequest
/// 3. Pushes the HttpResponse back to host
#[macro_export]
macro_rules! async_request_handler {
    ($func:ident) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn handle_request(request_id: i64) {
            use std::panic::{self, AssertUnwindSafe};
            use tokio::runtime::Runtime;
            use play_dylib_abi::http_abi::{HttpRequest, HttpResponse};
            
            let result = panic::catch_unwind(|| {
                let rt = Runtime::new().unwrap();
                rt.block_on(async move {
                    // Get host URL from environment
                    let host_url = std::env::var("HOST")
                        .unwrap_or_else(|_| "http://127.0.0.1:3000".to_string());
                    
                    // Fetch request from host
                    let request = match HttpRequest::fetch_from_host(request_id, &host_url).await {
                        Ok(req) => req,
                        Err(e) => {
                            eprintln!("Failed to fetch request {}: {:?}", request_id, e);
                            return Err(e);
                        }
                    };
                    
                    // Call user's handler function
                    let response = match $func(request).await {
                        Ok(resp) => resp,
                        Err(e) => {
                            eprintln!("User handler error for request {}: {:?}", request_id, e);
                            // Return error response
                            HttpResponse {
                                status_code: 500,
                                error: Some(format!("{:?}", e)),
                                ..Default::default()
                            }
                        }
                    };
                    
                    // Push response back to host
                    if let Err(e) = response.push_to_host(request_id, &host_url).await {
                        eprintln!("Failed to push response for request {}: {:?}", request_id, e);
                        return Err(e);
                    }
                    
                    Ok(())
                })
            });

            if let Err(panic_info) = result {
                let err_msg = if let Some(s) = panic_info.downcast_ref::<String>() {
                    format!("Panic occurred: {}", s)
                } else if let Some(s) = panic_info.downcast_ref::<&str>() {
                    format!("Panic occurred: {}", s)
                } else {
                    "Panic occurred: Unknown panic info".to_string()
                };
                eprintln!("Plugin panic: {}", err_msg);
                
                // Try to send error response on panic
                let rt = Runtime::new();
                if let Ok(rt) = rt {
                    let _ = rt.block_on(async {
                        let host_url = std::env::var("HOST")
                            .unwrap_or_else(|_| "http://127.0.0.1:3000".to_string());
                        let error_response = HttpResponse {
                            status_code: 500,
                            error: Some(err_msg),
                            ..Default::default()
                        };
                        let _ = error_response.push_to_host(request_id, &host_url).await;
                    });
                }
            } else if let Ok(Err(e)) = result {
                eprintln!("Plugin error: {:?}", e);
            }
        }
    };
}



#[cfg(test)]
mod tests{
    use super::*;

    #[tokio::test]
    async fn test_render_template()->anyhow::Result<()>{
        //host_url: env::var("HOST")?
        let req = HttpRequest{
            context: HostContext {
                host_url: "http://127.0.0.1:3000".to_string(),
                ..Default::default()
            },
            ..Default::default()
        };

        let resp = req.render_template(r#"
        11{{c}}{{a}}222
        % if flag:
        aaa
        % end
        "#, json!({
            "a":"sdfs",
            "c":"你好啊332sss",
            "flag":false,
        })).await?;

        println!("{:#?}", resp);
        Ok(())
    }
}