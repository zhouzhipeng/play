pub use rusqlite::{Connection, Error, Result};
use serde_json::Value;

const MAIN_SCHEMA: &str = include_str!("../sqlite_kv_docs_assets_schema.sql");
const ASSET_CHUNK_SCHEMA: &str = include_str!("../sqlite_asset_chunks_schema.sql");

pub fn init_main_db(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(MAIN_SCHEMA)
}

pub fn init_asset_chunk_db(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(ASSET_CHUNK_SCHEMA)
}

fn json_to_string(value: &Value) -> rusqlite::Result<String> {
    serde_json::to_string(value)
        .map_err(|err| rusqlite::Error::ToSqlConversionFailure(Box::new(err)))
}

fn json_from_string(value: String) -> rusqlite::Result<Value> {
    serde_json::from_str(&value).map_err(|err| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(err))
    })
}

fn blob_size(data: &[u8]) -> rusqlite::Result<i64> {
    i64::try_from(data.len())
        .map_err(|err| rusqlite::Error::ToSqlConversionFailure(Box::new(err)))
}

mod assets;
mod asset_service;
mod docs;
mod kv;

pub use assets::*;
pub use asset_service::*;
pub use docs::*;
pub use kv::*;
