pub mod models;
pub mod constants;

#[cfg(feature = "utils")]
pub mod utils;
pub mod redis_api;
pub mod tpl_engine_api;


pub trait MyTrait{
    fn bark(&self)->String;
}

#[cfg(feature = "proc_macros")]
pub use proc_macros::*;


#[macro_export]
macro_rules! file_path {
    ($s:expr) => {
        concat!(env!("CARGO_MANIFEST_DIR"),$s)
    };
}

#[macro_export]
macro_rules! current_timestamp {
    () => {{
        chrono::Utc::now().timestamp_millis()
    }};
}
