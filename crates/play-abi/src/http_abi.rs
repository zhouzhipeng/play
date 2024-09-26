use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize,Deserialize,Debug)]
pub struct HttpRequest {
    pub headers: HashMap<String, String>,
    pub query: HashMap<String, String>,
    pub body: String,
    pub url: String,
}

#[derive(Serialize,Deserialize,Debug, Default)]
pub struct HttpResponse {
    pub headers: HashMap<String, String>,
    pub body: String,
    pub status_code: u16,
    pub is_success: bool,
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
            let response = rt.block_on($func(request)).unwrap_or(HttpResponse {
                headers: Default::default(),
                body: "error".to_string(),
                status_code: 0,
                is_success: false,
            });

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

            let response = $func(request).unwrap_or(HttpResponse {
                headers: Default::default(),
                body: "error".to_string(),
                status_code: 0,
                is_success: false,
            });
            let result = serde_json::to_string(&response).unwrap();
            string_to_c_char(&result)
        }
    };
}