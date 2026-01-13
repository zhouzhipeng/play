use rusqlite::{params, params_from_iter, Connection, OptionalExtension};
use rusqlite::types::Value as SqlValue;
use serde::{Deserialize, Serialize};
use serde_json::{Number, Value};
use std::collections::HashMap;
use uuid::Uuid;

use crate::{json_to_string, random_id};

fn json_object_from_string(value: String) -> rusqlite::Result<HashMap<String, Value>> {
    serde_json::from_str(&value).map_err(|err| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(err))
    })
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocEntry {
    pub id: String,
    pub tag: String,
    #[serde(flatten)]
    pub doc: HashMap<String, Value>,
    pub created_at: i64,
    pub updated_at: i64,
}

fn flatten_doc_entry(mut entry: DocEntry) -> HashMap<String, Value> {
    entry
        .doc
        .insert("id".to_string(), Value::String(entry.id));
    entry
        .doc
        .insert("tag".to_string(), Value::String(entry.tag));
    entry.doc.insert(
        "created_at".to_string(),
        Value::Number(Number::from(entry.created_at)),
    );
    entry.doc.insert(
        "updated_at".to_string(),
        Value::Number(Number::from(entry.updated_at)),
    );
    entry.doc
}

#[derive(Debug, Clone, PartialEq)]
pub enum DocsJsonFilter {
    Eq { path: String, value: Value },
    Ne { path: String, value: Value },
    Gt { path: String, value: Value },
    Gte { path: String, value: Value },
    Lt { path: String, value: Value },
    Lte { path: String, value: Value },
    Like { path: String, pattern: String },
    Exists { path: String },
    NotExists { path: String },
}

#[derive(Debug, Clone, PartialEq)]
pub enum DocsOrderBy {
    Id,
    CreatedAt,
    UpdatedAt,
    JsonPath(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum DocsOrderDir {
    Asc,
    Desc,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DocsOrder {
    pub by: DocsOrderBy,
    pub dir: DocsOrderDir,
}

pub fn docs_create(conn: &Connection, tag: &str, doc: &Value) -> rusqlite::Result<String> {
    let doc_json = json_to_string(doc)?;
    let doc_id = random_id();
    conn.execute(
        "INSERT INTO docs (id, tag, doc) VALUES (?1, ?2, ?3)",
        params![doc_id, tag, doc_json],
    )?;
    Ok(doc_id)
}

pub fn docs_get(conn: &Connection, id: &str) -> rusqlite::Result<Option<HashMap<String, Value>>> {
    let entry = conn
        .query_row(
        "SELECT id, tag, doc, created_at, updated_at FROM docs WHERE id = ?1",
        params![id],
        |row| {
            let doc_json: String = row.get(2)?;
            Ok(DocEntry {
                id: row.get(0)?,
                tag: row.get(1)?,
                doc: json_object_from_string(doc_json)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        },
        )
        .optional()?;
    Ok(entry.map(flatten_doc_entry))
}

pub fn docs_update(conn: &Connection, id: &str, doc: &Value) -> rusqlite::Result<usize> {
    let doc_json = json_to_string(doc)?;
    conn.execute(
        "UPDATE docs SET doc = ?2 WHERE id = ?1",
        params![id, doc_json],
    )
}

pub fn docs_patch(
    conn: &Connection,
    tag: &str,
    id: &str,
    patch: &Value,
) -> rusqlite::Result<usize> {
    let patch_json = json_to_string(patch)?;
    conn.execute(
        "UPDATE docs SET doc = json_patch(doc, ?3) WHERE id = ?1 AND tag = ?2",
        params![id, tag, patch_json],
    )
}

pub fn docs_delete(conn: &Connection, id: &str) -> rusqlite::Result<usize> {
    conn.execute("DELETE FROM docs WHERE id = ?1", params![id])
}

pub fn docs_list(
    conn: &Connection,
    tag: &str,
) -> rusqlite::Result<Vec<HashMap<String, Value>>> {
    let mut stmt = conn.prepare(
        "SELECT id, tag, doc, created_at, updated_at FROM docs WHERE tag = ?1 ORDER BY id ASC",
    )?;
    let rows = stmt.query_map(params![tag], |row| {
        let doc_json: String = row.get(2)?;
        Ok(DocEntry {
            id: row.get(0)?,
            tag: row.get(1)?,
            doc: json_object_from_string(doc_json)?,
            created_at: row.get(3)?,
            updated_at: row.get(4)?,
        })
    })?;
    rows.map(|row| row.map(flatten_doc_entry)).collect()
}

pub fn docs_query(conn: &Connection, sql: &str) -> rusqlite::Result<Vec<HashMap<String, Value>>> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map([], |row| {
        let doc_json: String = row.get("doc")?;
        Ok(DocEntry {
            id: row.get("id")?,
            tag: row.get("tag")?,
            doc: json_object_from_string(doc_json)?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    })?;
    rows.map(|row| row.map(flatten_doc_entry)).collect()
}

pub fn docs_query_filters(
    conn: &Connection,
    tag: &str,
    filters: &[DocsJsonFilter],
    order: Option<DocsOrder>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> rusqlite::Result<Vec<HashMap<String, Value>>> {
    let mut sql = String::from("SELECT id, tag, doc, created_at, updated_at FROM docs WHERE tag = ?");
    let mut params: Vec<SqlValue> = vec![SqlValue::Text(tag.to_string())];

    if !filters.is_empty() {
        sql.push_str(" AND ");
        for (index, filter) in filters.iter().enumerate() {
            if index > 0 {
                sql.push_str(" AND ");
            }
            match filter {
                DocsJsonFilter::Eq { path, value } => {
                    if value.is_null() {
                        sql.push_str("json_extract(doc, ?) IS NULL");
                        params.push(SqlValue::Text(path.clone()));
                    } else {
                        sql.push_str("json_extract(doc, ?) = ?");
                        params.push(SqlValue::Text(path.clone()));
                        params.push(json_to_sql_value(value)?);
                    }
                }
                DocsJsonFilter::Ne { path, value } => {
                    if value.is_null() {
                        sql.push_str("json_extract(doc, ?) IS NOT NULL");
                        params.push(SqlValue::Text(path.clone()));
                    } else {
                        sql.push_str("json_extract(doc, ?) != ?");
                        params.push(SqlValue::Text(path.clone()));
                        params.push(json_to_sql_value(value)?);
                    }
                }
                DocsJsonFilter::Gt { path, value } => {
                    sql.push_str("json_extract(doc, ?) > ?");
                    params.push(SqlValue::Text(path.clone()));
                    params.push(json_to_sql_value(value)?);
                }
                DocsJsonFilter::Gte { path, value } => {
                    sql.push_str("json_extract(doc, ?) >= ?");
                    params.push(SqlValue::Text(path.clone()));
                    params.push(json_to_sql_value(value)?);
                }
                DocsJsonFilter::Lt { path, value } => {
                    sql.push_str("json_extract(doc, ?) < ?");
                    params.push(SqlValue::Text(path.clone()));
                    params.push(json_to_sql_value(value)?);
                }
                DocsJsonFilter::Lte { path, value } => {
                    sql.push_str("json_extract(doc, ?) <= ?");
                    params.push(SqlValue::Text(path.clone()));
                    params.push(json_to_sql_value(value)?);
                }
                DocsJsonFilter::Like { path, pattern } => {
                    sql.push_str("json_extract(doc, ?) LIKE ?");
                    params.push(SqlValue::Text(path.clone()));
                    params.push(SqlValue::Text(pattern.clone()));
                }
                DocsJsonFilter::Exists { path } => {
                    sql.push_str("json_type(doc, ?) IS NOT NULL");
                    params.push(SqlValue::Text(path.clone()));
                }
                DocsJsonFilter::NotExists { path } => {
                    sql.push_str("json_type(doc, ?) IS NULL");
                    params.push(SqlValue::Text(path.clone()));
                }
            }
        }
    }

    if let Some(order) = order {
        sql.push_str(" ORDER BY ");
        match order.by {
            DocsOrderBy::Id => sql.push_str("id"),
            DocsOrderBy::CreatedAt => sql.push_str("created_at"),
            DocsOrderBy::UpdatedAt => sql.push_str("updated_at"),
            DocsOrderBy::JsonPath(path) => {
                sql.push_str("json_extract(doc, ?)");
                params.push(SqlValue::Text(path));
            }
        }
        match order.dir {
            DocsOrderDir::Asc => sql.push_str(" ASC"),
            DocsOrderDir::Desc => sql.push_str(" DESC"),
        }
    }

    if let Some(limit) = limit {
        sql.push_str(" LIMIT ?");
        params.push(SqlValue::Integer(limit));
    }
    if let Some(offset) = offset {
        sql.push_str(" OFFSET ?");
        params.push(SqlValue::Integer(offset));
    }

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(params), |row| {
        let doc_json: String = row.get(2)?;
        Ok(DocEntry {
            id: row.get(0)?,
            tag: row.get(1)?,
            doc: json_object_from_string(doc_json)?,
            created_at: row.get(3)?,
            updated_at: row.get(4)?,
        })
    })?;
    rows.map(|row| row.map(flatten_doc_entry)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::init_main_db;
    use rusqlite::Connection;
    use serde_json::json;
    use std::collections::HashMap;

    fn assert_system_fields(doc: &HashMap<String, Value>, tag: &str, id: &str) {
        assert_eq!(doc.get("id").and_then(Value::as_str), Some(id));
        assert_eq!(doc.get("tag").and_then(Value::as_str), Some(tag));
        assert!(matches!(doc.get("created_at"), Some(Value::Number(_))));
        assert!(matches!(doc.get("updated_at"), Some(Value::Number(_))));
    }

    #[test]
    fn docs_crud() -> rusqlite::Result<()> {
        let conn = Connection::open_in_memory()?;
        init_main_db(&conn)?;

        let doc_v1 = json!({"name": "doc", "count": 1});
        let doc_id = docs_create(&conn, "doc", &doc_v1)?;
        assert_eq!(doc_id.len(), 12);

        let fetched = docs_get(&conn, &doc_id)?.expect("missing doc row");
        assert_system_fields(&fetched, "doc", &doc_id);
        assert_eq!(fetched.get("name").and_then(Value::as_str), Some("doc"));
        assert_eq!(fetched.get("count").and_then(Value::as_i64), Some(1));

        let doc_v2 = json!({"name": "doc", "count": 2});
        let updated = docs_update(&conn, &doc_id, &doc_v2)?;
        assert_eq!(updated, 1);
        let unchanged = docs_update(&conn, &doc_id, &doc_v2)?;
        assert_eq!(unchanged, 0);

        let fetched = docs_get(&conn, &doc_id)?.expect("missing doc row");
        assert_system_fields(&fetched, "doc", &doc_id);
        assert_eq!(fetched.get("name").and_then(Value::as_str), Some("doc"));
        assert_eq!(fetched.get("count").and_then(Value::as_i64), Some(2));

        let list = docs_list(&conn, "doc")?;
        assert_eq!(list.len(), 1);
        assert_system_fields(&list[0], "doc", &doc_id);

        let deleted = docs_delete(&conn, &doc_id)?;
        assert_eq!(deleted, 1);
        assert!(docs_get(&conn, &doc_id)?.is_none());

        Ok(())
    }

    #[test]
    fn docs_patch_update() -> rusqlite::Result<()> {
        let conn = Connection::open_in_memory()?;
        init_main_db(&conn)?;

        let doc = json!({
            "name": "doc",
            "count": 1,
            "nested": {"keep": true, "remove": 1},
            "remove_me": "x"
        });
        let doc_id = docs_create(&conn, "doc", &doc)?;

        let patch = json!({
            "count": 2,
            "nested": {"remove": null, "add": "yes"},
            "remove_me": null,
            "extra": {"flag": true}
        });
        let updated = docs_patch(&conn, "doc", &doc_id, &patch)?;
        assert_eq!(updated, 1);

        let fetched = docs_get(&conn, &doc_id)?.expect("missing doc row");
        assert_system_fields(&fetched, "doc", &doc_id);
        let expected = json!({
            "name": "doc",
            "count": 2,
            "nested": {"keep": true, "add": "yes"},
            "extra": {"flag": true}
        });
        assert_eq!(fetched.get("name").and_then(Value::as_str), Some("doc"));
        assert_eq!(fetched.get("count").and_then(Value::as_i64), Some(2));
        let expected_nested = json!({"keep": true, "add": "yes"});
        assert_eq!(fetched.get("nested"), Some(&expected_nested));
        assert_eq!(fetched.get("extra"), expected.get("extra"));
        assert!(!fetched.contains_key("remove_me"));

        let missing = docs_patch(&conn, "doc", "missing", &patch)?;
        assert_eq!(missing, 0);
        Ok(())
    }

    #[test]
    fn docs_query_json_extract() -> rusqlite::Result<()> {
        let conn = Connection::open_in_memory()?;
        init_main_db(&conn)?;

        let id_a = docs_create(
            &conn,
            "doc",
            &json!({"type": "a", "score": 10, "active": true, "label": "alpha"}),
        )?;
        let id_b = docs_create(
            &conn,
            "doc",
            &json!({"type": "b", "score": 15, "active": false, "label": "beta"}),
        )?;
        let id_c = docs_create(
            &conn,
            "doc",
            &json!({"type": "a", "score": 5, "label": "gamma"}),
        )?;

        let results = docs_query(
            &conn,
            "SELECT id, tag, doc, created_at, updated_at \
             FROM docs \
             WHERE tag = 'doc' AND json_extract(doc, '$.type') = 'a'",
        )?;
        let mut ids = results
            .iter()
            .filter_map(|doc| doc.get("id").and_then(Value::as_str))
            .map(str::to_string)
            .collect::<Vec<_>>();
        ids.sort();
        let mut expected = vec![id_a.clone(), id_c.clone()];
        expected.sort();
        assert_eq!(ids, expected);

        let results = docs_query(
            &conn,
            "SELECT id, tag, doc, created_at, updated_at \
             FROM docs \
             WHERE tag = 'doc' \
               AND json_extract(doc, '$.score') > 9 \
               AND json_type(doc, '$.active') IS NOT NULL",
        )?;
        assert_eq!(results.len(), 2);

        let results = docs_query(
            &conn,
            "SELECT id, tag, doc, created_at, updated_at \
             FROM docs \
             WHERE tag = 'doc' AND json_type(doc, '$.active') IS NULL",
        )?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].get("id").and_then(Value::as_str), Some(id_c.as_str()));

        let results = docs_query(
            &conn,
            "SELECT id, tag, doc, created_at, updated_at \
             FROM docs \
             WHERE tag = 'doc' AND json_extract(doc, '$.label') LIKE 'be%'",
        )?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].get("id").and_then(Value::as_str), Some(id_b.as_str()));

        let results = docs_query(
            &conn,
            "SELECT id, tag, doc, created_at, updated_at \
             FROM docs \
             WHERE tag = 'doc' \
             ORDER BY json_extract(doc, '$.score') DESC \
             LIMIT 2",
        )?;
        assert_eq!(results.len(), 2);
        assert_system_fields(&results[0], "doc", &id_b);
        assert_eq!(results[0].get("id").and_then(Value::as_str), Some(id_b.as_str()));
        assert_eq!(results[1].get("id").and_then(Value::as_str), Some(id_a.as_str()));
        Ok(())
    }
}
