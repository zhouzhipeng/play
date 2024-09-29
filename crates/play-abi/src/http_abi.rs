use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use anyhow::Context;
use serde::de::DeserializeOwned;

/// env info provided by host
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct HostContext {
    /// host http url , eg. http://127.0.0.1:3000
    pub host_url: String,
    pub plugin_prefix_url: String,
}



#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum HttpMethod {
   GET,POST,PUT,DELETE
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct HttpRequest {
    pub method:HttpMethod,
    pub headers: HashMap<String, String>,
    pub query: String,
    pub url: String,
    pub body: String,
    pub context: HostContext,
}




impl HttpRequest {
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

pub type HandleRequestFn = unsafe extern "C" fn(*mut std::os::raw::c_char) -> *mut std::os::raw::c_char;
pub type FreeCStringFn = unsafe extern "C" fn(*mut std::os::raw::c_char);
pub const HANDLE_REQUEST_FN_NAME: &'static str = "handle_request";

/// needs tokio runtime.
/// usage: `async_request_handler!(handle_request_impl);`
/// ```rust
/// async fn handle_request_impl(request: Request) -> anyhow::Result<Response>
/// ```
#[macro_export]
macro_rules! async_request_handler {
    ($func:ident) => {

       #[no_mangle]
        pub extern "C" fn handle_request(request: *mut std::os::raw::c_char) -> *mut std::os::raw::c_char {

            use play_abi::*;
            use std::panic::{self, AssertUnwindSafe};

            let result = panic::catch_unwind(||{
                let name = c_char_to_string(request);
                let request: HttpRequest = serde_json::from_str(&name).unwrap();
                use tokio::runtime::Runtime;
                let rt = Runtime::new().unwrap();
                let response = HttpResponse::from_anyhow(rt.block_on($func(request)));
                drop(rt);
                response
            });

            let response  = result.unwrap_or_else(|panic_info| {
                let err_msg = if let Some(s) = panic_info.downcast_ref::<String>() {
                    format!("Panic occurred: {}", s)
                } else if let Some(s) = panic_info.downcast_ref::<&str>() {
                    format!("Panic occurred: {}", s)
                } else {
                    "Panic occurred: Unknown panic info".to_string()
                };

                let response =HttpResponse::from_panic_error(err_msg);
                response
            });

            //convert response to c char string (make it compatible with ABI)
            let result = serde_json::to_string(&response).unwrap();
            string_to_c_char_mut(&result)
        }


      #[no_mangle]
        pub extern "C" fn free_c_string(ptr: *mut std::os::raw::c_char) {
            if !ptr.is_null() {
                // 将裸指针转换回 CString，并自动释放内存
                unsafe {
                    drop(std::ffi::CString::from_raw(ptr));
                }
            }
        }
    };
}


/// usage: `request_handler!(handle_request_impl);`
/// ```rust
///  fn handle_request_impl(request: Request) -> anyhow::Result<Response>
/// ```
#[macro_export]
macro_rules! request_handler {
    ($func:ident) => {

       #[no_mangle]
        pub extern "C" fn handle_request(request: *mut std::os::raw::c_char) -> *mut std::os::raw::c_char {

            use play_abi::*;
            use std::panic::{self, AssertUnwindSafe};

            let result = panic::catch_unwind(||{
                let name = c_char_to_string(request);
                let request: HttpRequest = serde_json::from_str(&name).unwrap();
                let response = HttpResponse::from_anyhow($func(request));
                response
            });

            let response  = result.unwrap_or_else(|panic_info| {
                let err_msg = if let Some(s) = panic_info.downcast_ref::<String>() {
                    format!("Panic occurred: {}", s)
                } else if let Some(s) = panic_info.downcast_ref::<&str>() {
                    format!("Panic occurred: {}", s)
                } else {
                    "Panic occurred: Unknown panic info".to_string()
                };

                let response =HttpResponse::from_panic_error(err_msg);
                response
            });

            //convert response to c char string (make it compatible with ABI)
            let result = serde_json::to_string(&response).unwrap();
            string_to_c_char_mut(&result)
        }

         #[no_mangle]
        pub extern "C" fn free_c_string(ptr: *mut std::os::raw::c_char) {
            if !ptr.is_null() {
                // 将裸指针转换回 CString，并自动释放内存
                unsafe {
                    drop(std::ffi::CString::from_raw(ptr));
                }
            }
        }
    };
}