pub mod models;
pub mod constants;

#[cfg(feature = "utils")]
pub mod utils;
pub mod tpl_engine_api;


pub trait MyTrait{
    fn bark(&self)->String;
}

use std::path::PathBuf;
use cargo_metadata::MetadataCommand;
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
#[macro_export]
macro_rules! timestamp_to_date_str {
    ($s: expr) => {{
        use chrono::TimeZone;
        chrono::Utc.timestamp_millis_opt($s).unwrap().format("%Y-%m-%d %H:%M:%S").to_string()
    }};
}



pub fn get_workspace_root() -> String {
    let metadata = MetadataCommand::new()
        .no_deps()
        .exec()
        .expect("Failed to get cargo metadata");

    metadata.workspace_root.into_string()
}



#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.

    use super::*;

    #[test]
    fn test_parse() -> anyhow::Result<()> {
        println!("{}", get_workspace_root());
        Ok(())
    }
}