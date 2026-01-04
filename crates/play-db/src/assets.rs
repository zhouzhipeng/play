use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::blob_size;

fn chunks_to_string(chunks: &[AssetChunkRef]) -> rusqlite::Result<String> {
    serde_json::to_string(chunks)
        .map_err(|err| rusqlite::Error::ToSqlConversionFailure(Box::new(err)))
}

fn chunks_from_string(value: String) -> rusqlite::Result<Vec<AssetChunkRef>> {
    serde_json::from_str(&value).map_err(|err| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(err))
    })
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetChunkRef {
    pub db_index: i64,
    pub chunk_index: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AssetMetadataInput {
    pub id: String,
    pub name: Option<String>,
    pub mime_type: Option<String>,
    pub size: i64,
    pub chunk_size: i64,
    pub chunks: Vec<AssetChunkRef>,
    pub checksum: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AssetChunkInput {
    pub chunk_index: i64,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AssetMetadata {
    pub id: String,
    pub name: Option<String>,
    pub mime_type: Option<String>,
    pub size: i64,
    pub chunk_size: i64,
    pub chunks: Vec<AssetChunkRef>,
    pub checksum: Option<String>,
    pub valid: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AssetChunk {
    pub id: i64,
    pub asset_id: String,
    pub chunk_index: i64,
    pub data: Vec<u8>,
    pub size: i64,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AssetWithChunks {
    pub metadata: AssetMetadata,
    pub chunks: Vec<AssetChunk>,
}

fn expected_chunk_count(size: i64, chunk_size: i64) -> i64 {
    if size <= 0 {
        0
    } else {
        (size + chunk_size - 1) / chunk_size
    }
}

fn checksum_for_chunks(chunks: &[AssetChunk]) -> String {
    let mut ordered: Vec<&AssetChunk> = chunks.iter().collect();
    ordered.sort_by_key(|chunk| chunk.chunk_index);
    let mut hasher = Sha256::new();
    for chunk in ordered {
        hasher.update(&chunk.data);
    }
    hex::encode(hasher.finalize())
}

fn asset_chunks_match_metadata(metadata: &AssetMetadata, chunks: &[AssetChunk]) -> bool {
    let expected = expected_chunk_count(metadata.size, metadata.chunk_size);
    if metadata.chunks.len() as i64 != expected || chunks.len() as i64 != expected {
        return false;
    }

    let mut meta_indices: Vec<i64> = metadata.chunks.iter().map(|chunk| chunk.chunk_index).collect();
    meta_indices.sort();
    meta_indices.dedup();
    if meta_indices.len() != metadata.chunks.len() {
        return false;
    }

    let mut chunk_indices: Vec<i64> = chunks.iter().map(|chunk| chunk.chunk_index).collect();
    chunk_indices.sort();
    chunk_indices.dedup();
    if chunk_indices.len() != chunks.len() || chunk_indices != meta_indices {
        return false;
    }

    if meta_indices != (0..expected).collect::<Vec<_>>() {
        return false;
    }

    let total_size: i64 = chunks.iter().map(|chunk| chunk.size).sum();
    total_size == metadata.size
}

fn asset_checksum_matches(metadata: &AssetMetadata, chunks: &[AssetChunk]) -> bool {
    let Some(expected) = metadata.checksum.as_ref() else {
        return false;
    };
    checksum_for_chunks(chunks).eq_ignore_ascii_case(expected)
}

fn asset_is_valid(metadata: &AssetMetadata, chunks: &[AssetChunk]) -> bool {
    asset_chunks_match_metadata(metadata, chunks) && asset_checksum_matches(metadata, chunks)
}

pub fn assets_create(conn: &Connection, asset: &AssetMetadataInput) -> rusqlite::Result<()> {
    let chunks_json = chunks_to_string(&asset.chunks)?;
    conn.execute(
        "INSERT INTO assets (id, name, mime_type, size, chunk_size, chunks, checksum)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            asset.id.as_str(),
            asset.name.as_deref(),
            asset.mime_type.as_deref(),
            asset.size,
            asset.chunk_size,
            chunks_json,
            asset.checksum.as_deref()
        ],
    )?;
    Ok(())
}

pub fn assets_get(conn: &Connection, id: &str) -> rusqlite::Result<Option<AssetMetadata>> {
    conn.query_row(
        "SELECT id, name, mime_type, size, chunk_size, chunks, checksum, created_at, updated_at
         FROM assets WHERE id = ?1",
        params![id],
        |row| {
            let chunks_json: String = row.get(5)?;
            Ok(AssetMetadata {
                id: row.get(0)?,
                name: row.get(1)?,
                mime_type: row.get(2)?,
                size: row.get(3)?,
                chunk_size: row.get(4)?,
                chunks: chunks_from_string(chunks_json)?,
                checksum: row.get(6)?,
                valid: false,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        },
    )
    .optional()
}

pub fn assets_update(conn: &Connection, asset: &AssetMetadataInput) -> rusqlite::Result<usize> {
    let chunks_json = chunks_to_string(&asset.chunks)?;
    conn.execute(
        "UPDATE assets
         SET name = ?2, mime_type = ?3, size = ?4, chunk_size = ?5, chunks = ?6, checksum = ?7
         WHERE id = ?1",
        params![
            asset.id.as_str(),
            asset.name.as_deref(),
            asset.mime_type.as_deref(),
            asset.size,
            asset.chunk_size,
            chunks_json,
            asset.checksum.as_deref()
        ],
    )
}

pub fn assets_delete(conn: &Connection, id: &str) -> rusqlite::Result<usize> {
    conn.execute("DELETE FROM assets WHERE id = ?1", params![id])
}

pub fn assets_list(conn: &Connection) -> rusqlite::Result<Vec<AssetMetadata>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, mime_type, size, chunk_size, chunks, checksum, created_at, updated_at
         FROM assets ORDER BY id ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        let chunks_json: String = row.get(5)?;
        Ok(AssetMetadata {
            id: row.get(0)?,
            name: row.get(1)?,
            mime_type: row.get(2)?,
            size: row.get(3)?,
            chunk_size: row.get(4)?,
            chunks: chunks_from_string(chunks_json)?,
            checksum: row.get(6)?,
            valid: false,
            created_at: row.get(7)?,
            updated_at: row.get(8)?,
        })
    })?;
    rows.collect()
}

pub fn asset_chunk_create(
    conn: &Connection,
    asset_id: &str,
    chunk_index: i64,
    data: &[u8],
) -> rusqlite::Result<i64> {
    let size = blob_size(data)?;
    conn.execute(
        "INSERT INTO asset_chunks (asset_id, chunk_index, data, size)
         VALUES (?1, ?2, ?3, ?4)",
        params![asset_id, chunk_index, data, size],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn asset_chunk_get(conn: &Connection, id: i64) -> rusqlite::Result<Option<AssetChunk>> {
    conn.query_row(
        "SELECT id, asset_id, chunk_index, data, size, created_at
         FROM asset_chunks WHERE id = ?1",
        params![id],
        |row| {
            Ok(AssetChunk {
                id: row.get(0)?,
                asset_id: row.get(1)?,
                chunk_index: row.get(2)?,
                data: row.get(3)?,
                size: row.get(4)?,
                created_at: row.get(5)?,
            })
        },
    )
    .optional()
}

pub fn asset_chunk_get_by_index(
    conn: &Connection,
    asset_id: &str,
    chunk_index: i64,
) -> rusqlite::Result<Option<AssetChunk>> {
    conn.query_row(
        "SELECT id, asset_id, chunk_index, data, size, created_at
         FROM asset_chunks WHERE asset_id = ?1 AND chunk_index = ?2",
        params![asset_id, chunk_index],
        |row| {
            Ok(AssetChunk {
                id: row.get(0)?,
                asset_id: row.get(1)?,
                chunk_index: row.get(2)?,
                data: row.get(3)?,
                size: row.get(4)?,
                created_at: row.get(5)?,
            })
        },
    )
    .optional()
}

pub fn asset_chunk_update(
    conn: &Connection,
    id: i64,
    data: &[u8],
) -> rusqlite::Result<usize> {
    let size = blob_size(data)?;
    conn.execute(
        "UPDATE asset_chunks SET data = ?2, size = ?3 WHERE id = ?1",
        params![id, data, size],
    )
}

pub fn asset_chunk_delete(conn: &Connection, id: i64) -> rusqlite::Result<usize> {
    conn.execute("DELETE FROM asset_chunks WHERE id = ?1", params![id])
}

pub fn asset_chunks_delete_for_asset(
    conn: &Connection,
    asset_id: &str,
) -> rusqlite::Result<usize> {
    conn.execute(
        "DELETE FROM asset_chunks WHERE asset_id = ?1",
        params![asset_id],
    )
}

pub fn asset_chunks_for_asset(
    conn: &Connection,
    asset_id: &str,
) -> rusqlite::Result<Vec<AssetChunk>> {
    let mut stmt = conn.prepare(
        "SELECT id, asset_id, chunk_index, data, size, created_at
         FROM asset_chunks WHERE asset_id = ?1 ORDER BY chunk_index ASC",
    )?;
    let rows = stmt.query_map(params![asset_id], |row| {
        Ok(AssetChunk {
            id: row.get(0)?,
            asset_id: row.get(1)?,
            chunk_index: row.get(2)?,
            data: row.get(3)?,
            size: row.get(4)?,
            created_at: row.get(5)?,
        })
    })?;
    rows.collect()
}

pub fn assets_create_with_chunks(
    meta_conn: &mut Connection,
    chunks_conn: &mut Connection,
    asset: &AssetMetadataInput,
    chunks: &[AssetChunkInput],
) -> rusqlite::Result<Vec<i64>> {
    let tx_meta = meta_conn.transaction()?;
    let tx_chunks = chunks_conn.transaction()?;

    assets_create(&tx_meta, asset)?;
    let mut ids = Vec::with_capacity(chunks.len());
    for chunk in chunks {
        let id = asset_chunk_create(
            &tx_chunks,
            asset.id.as_str(),
            chunk.chunk_index,
            &chunk.data,
        )?;
        ids.push(id);
    }

    tx_chunks.commit()?;
    if let Err(err) = tx_meta.commit() {
        let _ = asset_chunks_delete_for_asset(chunks_conn, asset.id.as_str());
        return Err(err);
    }

    Ok(ids)
}

pub fn assets_get_with_chunks(
    meta_conn: &Connection,
    chunks_conn: &Connection,
    id: &str,
) -> rusqlite::Result<Option<AssetWithChunks>> {
    let Some(mut metadata) = assets_get(meta_conn, id)? else {
        return Ok(None);
    };
    let chunks = asset_chunks_for_asset(chunks_conn, id)?;
    metadata.valid = asset_is_valid(&metadata, &chunks);
    Ok(Some(AssetWithChunks { metadata, chunks }))
}

pub fn assets_update_with_chunks(
    meta_conn: &mut Connection,
    chunks_conn: &mut Connection,
    asset: &AssetMetadataInput,
    chunks: &[AssetChunkInput],
) -> rusqlite::Result<usize> {
    let tx_meta = meta_conn.transaction()?;
    let tx_chunks = chunks_conn.transaction()?;

    let updated = assets_update(&tx_meta, asset)?;
    if updated == 0 {
        return Ok(0);
    }

    asset_chunks_delete_for_asset(&tx_chunks, asset.id.as_str())?;
    for chunk in chunks {
        asset_chunk_create(
            &tx_chunks,
            asset.id.as_str(),
            chunk.chunk_index,
            &chunk.data,
        )?;
    }

    tx_chunks.commit()?;
    if let Err(err) = tx_meta.commit() {
        let _ = asset_chunks_delete_for_asset(chunks_conn, asset.id.as_str());
        return Err(err);
    }

    Ok(updated)
}

pub fn assets_delete_with_chunks(
    meta_conn: &mut Connection,
    chunks_conn: &mut Connection,
    id: &str,
) -> rusqlite::Result<usize> {
    let tx_meta = meta_conn.transaction()?;
    let tx_chunks = chunks_conn.transaction()?;

    let deleted = assets_delete(&tx_meta, id)?;
    asset_chunks_delete_for_asset(&tx_chunks, id)?;

    tx_meta.commit()?;
    tx_chunks.commit()?;
    Ok(deleted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{init_asset_chunk_db, init_main_db};
    use rusqlite::Connection;

    #[test]
    fn assets_crud() -> rusqlite::Result<()> {
        let conn = Connection::open_in_memory()?;
        init_main_db(&conn)?;

        let asset_v1 = AssetMetadataInput {
            id: "asset-1".to_string(),
            name: Some("file.txt".to_string()),
            mime_type: Some("text/plain".to_string()),
            size: 12,
            chunk_size: 4,
            chunks: vec![AssetChunkRef {
                db_index: 0,
                chunk_index: 0,
            }],
            checksum: Some("abc123".to_string()),
        };
        assets_create(&conn, &asset_v1)?;

        let fetched = assets_get(&conn, "asset-1")?.expect("missing asset row");
        assert_eq!(fetched.name, asset_v1.name);
        assert_eq!(fetched.mime_type, asset_v1.mime_type);
        assert_eq!(fetched.size, asset_v1.size);
        assert_eq!(fetched.chunk_size, asset_v1.chunk_size);
        assert_eq!(fetched.chunks, asset_v1.chunks);
        assert_eq!(fetched.checksum, asset_v1.checksum);

        let list = assets_list(&conn)?;
        assert_eq!(list.len(), 1);

        let asset_v2 = AssetMetadataInput {
            id: "asset-1".to_string(),
            name: Some("file.bin".to_string()),
            mime_type: Some("application/octet-stream".to_string()),
            size: 99,
            chunk_size: 8,
            chunks: vec![
                AssetChunkRef {
                    db_index: 1,
                    chunk_index: 0,
                },
                AssetChunkRef {
                    db_index: 1,
                    chunk_index: 1,
                },
            ],
            checksum: None,
        };
        let updated = assets_update(&conn, &asset_v2)?;
        assert_eq!(updated, 1);

        let fetched = assets_get(&conn, "asset-1")?.expect("missing asset row");
        assert_eq!(fetched.name, asset_v2.name);
        assert_eq!(fetched.mime_type, asset_v2.mime_type);
        assert_eq!(fetched.size, asset_v2.size);
        assert_eq!(fetched.chunk_size, asset_v2.chunk_size);
        assert_eq!(fetched.chunks, asset_v2.chunks);
        assert_eq!(fetched.checksum, asset_v2.checksum);

        let deleted = assets_delete(&conn, "asset-1")?;
        assert_eq!(deleted, 1);
        assert!(assets_get(&conn, "asset-1")?.is_none());
        Ok(())
    }

    #[test]
    fn assets_with_chunks_crud() -> rusqlite::Result<()> {
        let mut meta_conn = Connection::open_in_memory()?;
        let mut chunks_conn = Connection::open_in_memory()?;
        init_main_db(&meta_conn)?;
        init_asset_chunk_db(&chunks_conn)?;

        let asset = AssetMetadataInput {
            id: "asset-2".to_string(),
            name: Some("blob".to_string()),
            mime_type: Some("application/octet-stream".to_string()),
            size: 6,
            chunk_size: 3,
            chunks: vec![
                AssetChunkRef {
                    db_index: 0,
                    chunk_index: 0,
                },
                AssetChunkRef {
                    db_index: 0,
                    chunk_index: 1,
                },
            ],
            checksum: None,
        };
        let chunks = vec![
            AssetChunkInput {
                chunk_index: 0,
                data: vec![1, 2, 3],
            },
            AssetChunkInput {
                chunk_index: 1,
                data: vec![4, 5, 6],
            },
        ];
        let ids = assets_create_with_chunks(&mut meta_conn, &mut chunks_conn, &asset, &chunks)?;
        assert_eq!(ids.len(), 2);

        let fetched =
            assets_get_with_chunks(&meta_conn, &chunks_conn, "asset-2")?.expect("missing asset");
        assert_eq!(fetched.metadata.name, asset.name);
        assert_eq!(fetched.metadata.size, asset.size);
        assert_eq!(fetched.chunks.len(), 2);
        assert_eq!(fetched.chunks[0].chunk_index, 0);
        assert_eq!(fetched.chunks[0].data, vec![1, 2, 3]);

        let asset_v2 = AssetMetadataInput {
            id: "asset-2".to_string(),
            name: Some("blob-v2".to_string()),
            mime_type: Some("application/data".to_string()),
            size: 4,
            chunk_size: 2,
            chunks: vec![
                AssetChunkRef {
                    db_index: 1,
                    chunk_index: 0,
                },
                AssetChunkRef {
                    db_index: 1,
                    chunk_index: 1,
                },
            ],
            checksum: Some("deadbeef".to_string()),
        };
        let chunks_v2 = vec![
            AssetChunkInput {
                chunk_index: 0,
                data: vec![9, 9],
            },
            AssetChunkInput {
                chunk_index: 1,
                data: vec![8, 8],
            },
        ];
        let updated =
            assets_update_with_chunks(&mut meta_conn, &mut chunks_conn, &asset_v2, &chunks_v2)?;
        assert_eq!(updated, 1);

        let fetched =
            assets_get_with_chunks(&meta_conn, &chunks_conn, "asset-2")?.expect("missing asset");
        assert_eq!(fetched.metadata.name, asset_v2.name);
        assert_eq!(fetched.metadata.size, asset_v2.size);
        assert_eq!(fetched.metadata.checksum, asset_v2.checksum);
        assert_eq!(fetched.chunks.len(), 2);
        assert_eq!(fetched.chunks[0].data, vec![9, 9]);
        assert_eq!(fetched.chunks[1].data, vec![8, 8]);

        let deleted = assets_delete_with_chunks(&mut meta_conn, &mut chunks_conn, "asset-2")?;
        assert_eq!(deleted, 1);
        assert!(assets_get(&meta_conn, "asset-2")?.is_none());
        let remaining = asset_chunks_for_asset(&chunks_conn, "asset-2")?;
        assert!(remaining.is_empty());
        Ok(())
    }

    #[test]
    fn asset_chunks_crud() -> rusqlite::Result<()> {
        let conn = Connection::open_in_memory()?;
        init_asset_chunk_db(&conn)?;

        let chunk_id = asset_chunk_create(&conn, "asset-1", 0, &[1, 2, 3])?;
        let fetched = asset_chunk_get(&conn, chunk_id)?.expect("missing chunk row");
        assert_eq!(fetched.asset_id, "asset-1");
        assert_eq!(fetched.chunk_index, 0);
        assert_eq!(fetched.data, vec![1, 2, 3]);
        assert_eq!(fetched.size, 3);

        let fetched_by_index =
            asset_chunk_get_by_index(&conn, "asset-1", 0)?.expect("missing chunk row");
        assert_eq!(fetched_by_index.id, chunk_id);

        let updated = asset_chunk_update(&conn, chunk_id, &[9, 8, 7, 6])?;
        assert_eq!(updated, 1);
        let fetched = asset_chunk_get(&conn, chunk_id)?.expect("missing chunk row");
        assert_eq!(fetched.data, vec![9, 8, 7, 6]);
        assert_eq!(fetched.size, 4);

        let list = asset_chunks_for_asset(&conn, "asset-1")?;
        assert_eq!(list.len(), 1);

        let deleted = asset_chunk_delete(&conn, chunk_id)?;
        assert_eq!(deleted, 1);
        assert!(asset_chunk_get(&conn, chunk_id)?.is_none());
        Ok(())
    }
}
