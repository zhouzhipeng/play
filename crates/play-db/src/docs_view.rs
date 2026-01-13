use rusqlite::Connection;

fn view_error(message: &str) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        message,
    )))
}

fn validate_view_name(view_name: &str) -> rusqlite::Result<()> {
    if !view_name.starts_with("v_") {
        return Err(view_error("view name must start with 'v_'"));
    }
    if view_name.is_empty()
        || !view_name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        return Err(view_error("view name must be ASCII alphanumeric or underscore"));
    }
    Ok(())
}

pub fn docs_view_create(
    conn: &Connection,
    view_name: &str,
    select_sql: &str,
) -> rusqlite::Result<()> {
    validate_view_name(view_name)?;
    let select_sql = select_sql.trim();
    if select_sql.is_empty() {
        return Err(view_error("select_sql must not be empty"));
    }
    let select_sql = select_sql.trim_end_matches(';');
    let sql = format!("CREATE VIEW IF NOT EXISTS {} AS {}", view_name, select_sql);
    conn.execute(&sql, [])?;
    Ok(())
}

pub fn docs_view_drop(conn: &Connection, view_name: &str) -> rusqlite::Result<()> {
    validate_view_name(view_name)?;
    let sql = format!("DROP VIEW IF EXISTS {}", view_name);
    conn.execute(&sql, [])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{docs_create, init_main_db};
    use rusqlite::{params, Connection, OptionalExtension};
    use serde_json::json;

    fn view_exists(conn: &Connection, view_name: &str) -> rusqlite::Result<bool> {
        let name = conn
            .query_row(
                "SELECT name FROM sqlite_master WHERE type = 'view' AND name = ?1",
                params![view_name],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        Ok(name.is_some())
    }

    #[test]
    fn docs_view_create_and_query() -> rusqlite::Result<()> {
        let conn = Connection::open_in_memory()?;
        init_main_db(&conn)?;

        docs_view_create(
            &conn,
            "v_notes",
            "SELECT id, tag, doc, created_at, updated_at FROM docs WHERE tag = 'note'",
        )?;

        docs_create(&conn, "note", &json!({"title": "Hello"}))?;
        docs_create(&conn, "task", &json!({"title": "Ignore"}))?;

        let count: i64 = conn.query_row("SELECT COUNT(*) FROM v_notes", [], |row| row.get(0))?;
        assert_eq!(count, 1);
        assert!(view_exists(&conn, "v_notes")?);
        Ok(())
    }

    #[test]
    fn docs_view_requires_prefix() -> rusqlite::Result<()> {
        let conn = Connection::open_in_memory()?;
        init_main_db(&conn)?;

        let result = docs_view_create(
            &conn,
            "notes",
            "SELECT id, tag, doc, created_at, updated_at FROM docs",
        );
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn docs_view_drop_removes_view() -> rusqlite::Result<()> {
        let conn = Connection::open_in_memory()?;
        init_main_db(&conn)?;

        docs_view_create(
            &conn,
            "v_docs_all",
            "SELECT id, tag, doc, created_at, updated_at FROM docs",
        )?;
        assert!(view_exists(&conn, "v_docs_all")?);

        docs_view_drop(&conn, "v_docs_all")?;
        assert!(!view_exists(&conn, "v_docs_all")?);
        Ok(())
    }
}
