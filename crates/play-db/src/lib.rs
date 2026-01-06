pub use rusqlite::{Connection, Error, Result};
use rusqlite::types::{Value as SqlValue, ValueRef};
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

fn sql_error(message: &str) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        message,
    )))
}

fn json_to_sql_value(value: &Value) -> rusqlite::Result<SqlValue> {
    Ok(match value {
        Value::Null => SqlValue::Null,
        Value::Bool(flag) => SqlValue::Integer(i64::from(*flag)),
        Value::Number(number) => {
            if let Some(value) = number.as_i64() {
                SqlValue::Integer(value)
            } else if let Some(value) = number.as_u64() {
                if value <= i64::MAX as u64 {
                    SqlValue::Integer(value as i64)
                } else {
                    SqlValue::Real(value as f64)
                }
            } else if let Some(value) = number.as_f64() {
                SqlValue::Real(value)
            } else {
                SqlValue::Null
            }
        }
        Value::String(value) => SqlValue::Text(value.clone()),
        Value::Array(_) | Value::Object(_) => SqlValue::Text(json_to_string(value)?),
    })
}

fn value_ref_to_json(value: ValueRef<'_>) -> Value {
    match value {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(value) => Value::Number(serde_json::Number::from(value)),
        ValueRef::Real(value) => serde_json::Number::from_f64(value)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        ValueRef::Text(value) => Value::String(String::from_utf8_lossy(value).into_owned()),
        ValueRef::Blob(value) => Value::String(hex::encode(value)),
    }
}

fn parse_param_sets(params_json: &str) -> rusqlite::Result<Vec<Vec<SqlValue>>> {
    let raw = params_json.trim();
    if raw.is_empty() {
        return Ok(Vec::new());
    }
    let value: Value = serde_json::from_str(raw)
        .map_err(|err| rusqlite::Error::ToSqlConversionFailure(Box::new(err)))?;
    let Value::Array(items) = value else {
        return Err(sql_error("params_json must be a JSON array"));
    };
    if items.is_empty() {
        return Ok(Vec::new());
    }
    let all_arrays = items.iter().all(|item| matches!(item, Value::Array(_)));
    if all_arrays {
        items
            .into_iter()
            .map(|item| {
                let Value::Array(values) = item else {
                    return Err(sql_error("params_json must be a JSON array"));
                };
                values
                    .into_iter()
                    .map(|value| json_to_sql_value(&value))
                    .collect()
            })
            .collect()
    } else {
        let params = items
            .into_iter()
            .map(|value| json_to_sql_value(&value))
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(vec![params])
    }
}

pub fn execute_sql(
    db_path: &str,
    sql: &str,
    params_json: &str,
) -> rusqlite::Result<String> {
    let conn = if db_path.trim().is_empty() {
        Connection::open_in_memory()?
    } else {
        Connection::open(db_path)?
    };
    let mut stmt = conn.prepare(sql)?;
    let column_count = stmt.column_count();
    let param_sets = parse_param_sets(params_json)?;

    if column_count > 0 {
        let params = param_sets.into_iter().next().unwrap_or_default();
        let column_names: Vec<String> = (0..column_count)
            .map(|idx| {
                let name = stmt.column_name(idx).unwrap_or("");
                if name.is_empty() {
                    format!("col_{}", idx)
                } else {
                    name.to_string()
                }
            })
            .collect();
        let rows_iter = stmt.query_map(rusqlite::params_from_iter(params), |row| {
            let mut map = serde_json::Map::with_capacity(column_names.len());
            for (idx, name) in column_names.iter().enumerate() {
                let value = row.get_ref(idx)?;
                map.insert(name.clone(), value_ref_to_json(value));
            }
            Ok(Value::Object(map))
        })?;
        let mut rows = Vec::new();
        for row in rows_iter {
            rows.push(row?);
        }
        let json_result = serde_json::json!({
            "success": true,
            "rows": rows,
            "rows_affected": rows.len()
        });
        return serde_json::to_string(&json_result)
            .map_err(|err| rusqlite::Error::ToSqlConversionFailure(Box::new(err)));
    }

    let mut rows_affected = 0;
    if param_sets.is_empty() {
        rows_affected = stmt.execute([])?;
    } else {
        for params in param_sets {
            rows_affected += stmt.execute(rusqlite::params_from_iter(params))?;
        }
    }
    let json_result = serde_json::json!({
        "success": true,
        "rows": [],
        "rows_affected": rows_affected
    });
    serde_json::to_string(&json_result)
        .map_err(|err| rusqlite::Error::ToSqlConversionFailure(Box::new(err)))
}

pub fn random_id() -> String {
    let uuid = uuid::Uuid::new_v4();
    let uuid_hex = uuid.simple().to_string();
    let random_hex = &uuid_hex[..12];
    random_hex.to_string()
}

mod assets;
mod asset_service;
mod docs;
mod docs_view;
mod kv;

pub use assets::*;
pub use asset_service::*;
pub use docs::*;
pub use docs_view::*;
pub use kv::*;
