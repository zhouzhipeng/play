use rusqlite::{params, Connection, OptionalExtension};

#[derive(Debug, Clone, PartialEq)]
pub struct KvEntry {
    pub key: String,
    pub value: String,
    pub created_at: i64,
    pub updated_at: i64,
}

pub fn kv_create(conn: &Connection, key: &str, value: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO kv (key, value) VALUES (?1, ?2)",
        params![key, value],
    )?;
    Ok(())
}

pub fn kv_get(conn: &Connection, key: &str) -> rusqlite::Result<Option<KvEntry>> {
    conn.query_row(
        "SELECT key, value, created_at, updated_at FROM kv WHERE key = ?1",
        params![key],
        |row| {
            Ok(KvEntry {
                key: row.get(0)?,
                value: row.get(1)?,
                created_at: row.get(2)?,
                updated_at: row.get(3)?,
            })
        },
    )
    .optional()
}

pub fn kv_update(conn: &Connection, key: &str, value: &str) -> rusqlite::Result<usize> {
    conn.execute(
        "UPDATE kv SET value = ?2 WHERE key = ?1",
        params![key, value],
    )
}

pub fn kv_delete(conn: &Connection, key: &str) -> rusqlite::Result<usize> {
    conn.execute("DELETE FROM kv WHERE key = ?1", params![key])
}

pub fn kv_list(conn: &Connection) -> rusqlite::Result<Vec<KvEntry>> {
    let mut stmt = conn.prepare(
        "SELECT key, value, created_at, updated_at FROM kv ORDER BY key ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(KvEntry {
            key: row.get(0)?,
            value: row.get(1)?,
            created_at: row.get(2)?,
            updated_at: row.get(3)?,
        })
    })?;
    rows.collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::init_main_db;
    use rusqlite::Connection;

    #[test]
    fn kv_crud_and_changelog() -> rusqlite::Result<()> {
        let conn = Connection::open_in_memory()?;
        init_main_db(&conn)?;

        kv_create(&conn, "alpha", "1")?;
        let entry = kv_get(&conn, "alpha")?.expect("missing kv row");
        assert_eq!(entry.value, "1");

        let updated = kv_update(&conn, "alpha", "2")?;
        assert_eq!(updated, 1);
        let entry = kv_get(&conn, "alpha")?.expect("missing kv row");
        assert_eq!(entry.value, "2");

        let list = kv_list(&conn)?;
        assert_eq!(list.len(), 1);

        let deleted = kv_delete(&conn, "alpha")?;
        assert_eq!(deleted, 1);
        assert!(kv_get(&conn, "alpha")?.is_none());

        let mut stmt = conn.prepare("SELECT op FROM kv_changelog ORDER BY id ASC")?;
        let ops = stmt
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        assert_eq!(
            ops,
            vec![
                "INSERT".to_string(),
                "UPDATE".to_string(),
                "UPDATE".to_string(),
                "DELETE".to_string(),
            ]
        );
        Ok(())
    }
}
