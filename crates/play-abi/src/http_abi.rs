use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use anyhow::Context;
use serde::de::DeserializeOwned;


#[derive(Serialize,Deserialize,Debug)]
pub struct HttpRequest {
    pub headers: HashMap<String, String>,
    pub query: String,
    pub url: String,
    pub body: String,
}

impl HttpRequest{
    pub fn parse_query<T: DeserializeOwned>(&self)-> anyhow::Result<T>{
        let p : T = serde_urlencoded::from_str(&self.query).context("parse query str error!")?;
        Ok(p)
    }
}

#[derive(Serialize,Deserialize,Debug, Default)]
pub struct HttpResponse {
    pub headers: HashMap<String, String>,
    pub body: String,
    pub status_code: u16,
    /// used to mark current plugin running is success or not (shoule left None for normal bussiness logic)
    /// will be used automatically when `?` triggered in your logic.
    pub error: Option<String>,
}

fn print_error(err: &anyhow::Error) ->String{
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


impl HttpResponse{
    pub fn from_anyhow(r: anyhow::Result<Self>)->HttpResponse{
        r.unwrap_or_else(|e| {
            HttpResponse {
                error: Some(print_error(&e)),
                ..Self::default()
            }
        })
    }
}

pub type HandleRequestFn =unsafe extern "C" fn(*const std::os::raw::c_char) ->  *const std::os::raw::c_char;
pub const HANDLE_REQUEST_FN_NAME : &'static str = "handle_request";

/// needs tokio runtime.
/// usage: `async_request_handler!(handle_request_impl);`
/// ```rust
/// async fn handle_request_impl(request: Request) -> anyhow::Result<Response>
/// ```
#[macro_export]
macro_rules! async_request_handler {
    ($func:ident) => {

        #[no_mangle]
        pub extern "C" fn handle_request(request: *const std::os::raw::c_char) -> *const std::os::raw::c_char {
            use play_abi::*;
            let name = c_char_to_string(request);
            let request: HttpRequest = serde_json::from_str(&name).unwrap();

            use tokio::runtime::Runtime;

            let rt = Runtime::new().unwrap();
            let response = HttpResponse::from_anyhow(rt.block_on($func(request)));

            let result = serde_json::to_string(&response).unwrap();
            string_to_c_char(&result)
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
        pub extern "C" fn handle_request(request: *const std::os::raw::c_char) -> *const std::os::raw::c_char {
            use play_abi::*;
            let name = c_char_to_string(request);
            let request: HttpRequest = serde_json::from_str(&name).unwrap();

            let response =  HttpResponse::from_anyhow($func(request));
            let result = serde_json::to_string(&response).unwrap();
            string_to_c_char(&result)
        }
    };
}