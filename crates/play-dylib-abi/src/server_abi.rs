pub type RunFn = unsafe extern "C" fn(*mut std::os::raw::c_char);
pub const RUN_FN_NAME: &'static str = "run";


/// needs tokio runtime.
/// usage: `async_run!(run);`
/// ```rust
/// use std::task::Context;
///
///  async fn run_impl(context: Context){}
/// ```
#[macro_export]
macro_rules! async_run {
    ($func:ident) => {

       #[no_mangle]
        pub extern "C" fn run(request: *mut std::os::raw::c_char){

            use play_dylib_abi::*;
            use std::panic::{self, AssertUnwindSafe};

            let result = panic::catch_unwind(||{
                 let name = c_char_to_string(request);
                let request: HostContext = serde_json::from_str(&name).unwrap();

                use tokio::runtime::Runtime;
                let rt = Runtime::new().unwrap();
                let result = rt.block_on($func(request));
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
