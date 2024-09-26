use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize,Deserialize,Debug)]
pub struct Request {
    pub headers: HashMap<String, String>,
    pub query: HashMap<String, String>,
    pub body: String,
    pub url: String,
}

#[derive(Serialize,Deserialize,Debug)]
pub struct Response {
    pub headers: HashMap<String, String>,
    pub body: String,
    pub status_code: u16,
}

pub type HandleRequestFn =fn(Request) -> anyhow::Result<Response>;
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
        pub extern "C" fn handle_request(request: Request) -> anyhow::Result<Response> {
            use tokio::runtime::Runtime;

            let rt = Runtime::new()?;
            let resp = rt.block_on($func(request))?;
            Ok(resp)
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
        pub extern "C" fn handle_request(request: Request) -> anyhow::Result<Response> {
            $func(request)
        }
    };
}