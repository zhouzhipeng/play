pub mod models;
pub mod constants;

#[cfg(feature = "utils")]
pub mod utils;


pub trait MyTrait{
    fn bark(&self)->String;
}

#[cfg(feature = "macros")]
pub use macros::*;