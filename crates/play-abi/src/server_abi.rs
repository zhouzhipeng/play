use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use anyhow::Context;
use reqwest::Client;
use serde::de::DeserializeOwned;
use serde_json::{json, Value};




pub type RunFn = unsafe extern "C" fn();
pub const RUN_FN_NAME: &'static str = "run";


/// needs tokio runtime.
/// usage: `async_run!(run);`
/// ```rust
/// async fn run(){}
/// ```
#[macro_export]
macro_rules! async_run {
    ($func:ident) => {

       #[no_mangle]
        pub extern "C" fn run(){

            use play_abi::*;
            use std::panic::{self, AssertUnwindSafe};

            let result = panic::catch_unwind(||{

                use tokio::runtime::Runtime;
                let rt = Runtime::new().unwrap();
                let result = rt.block_on($func());
                println!("{:?}", result);
                drop(rt);

            });

            result.unwrap_or_else(|panic_info| {
                let err_msg = if let Some(s) = panic_info.downcast_ref::<String>() {
                    format!("Panic occurred: {}", s)
                } else if let Some(s) = panic_info.downcast_ref::<&str>() {
                    format!("Panic occurred: {}", s)
                } else {
                    "Panic occurred: Unknown panic info".to_string()
                };

            });

        }

    };
}
