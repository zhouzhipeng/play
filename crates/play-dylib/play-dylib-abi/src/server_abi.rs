pub type RunFn = unsafe extern "C" fn();
pub const RUN_FN_NAME: &'static str = "run";

/// Optimized server ABI following http_abi pattern.
/// Eliminates FFI memory management by using simple parameter passing.
/// 
/// Usage: `async_run!(run_impl);`
/// ```rust
/// async fn run_impl() -> anyhow::Result<()> {
///     // Get context from environment
///     let context = HostContext::from_env()?;
///     // Your server logic here
///     Ok(())
/// }
/// ```
/// 
/// The macro handles all the runtime management:
/// 1. Creates tokio runtime 
/// 2. Calls your function
/// 3. Handles errors and panic recovery
#[macro_export]
macro_rules! async_run {
    ($func:ident) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn run() {
            use std::panic::{self, AssertUnwindSafe};
            use tokio::runtime::Runtime;
            
            let result = panic::catch_unwind(|| {
                let rt = Runtime::new().unwrap();
                rt.block_on(async move {
                    // Call user's server function
                    match $func().await {
                        Ok(_) => {
                            println!("Server function completed successfully");
                            Ok(())
                        }
                        Err(e) => {
                            eprintln!("Server function error: {:?}", e);
                            Err(e)
                        }
                    }
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
                eprintln!("Server run panic: {}", err_msg);
            } else if let Ok(Err(e)) = result {
                eprintln!("Server error: {:?}", e);
            }
        }
    };
}

