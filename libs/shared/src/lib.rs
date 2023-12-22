pub mod models;
pub mod constants;

#[cfg(feature = "utils")]
pub mod utils;
pub mod redis_api;


pub trait MyTrait{
    fn bark(&self)->String;
}

#[cfg(feature = "proc_macros")]
pub use proc_macros::*;