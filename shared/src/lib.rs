pub mod models;
pub mod constants;

#[cfg(feature = "utils")]
pub mod utils;


pub trait MyTrait{
    fn bark(&self)->String;
}

pub use macros::*;