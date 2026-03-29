use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;

use crate::{blob_size, random_id};

fn chunks_to_string(chunks: &[AssetChunkRef]) -> rusqlite::Result<String> {
    let compact: Vec<[i64; 2]> = chunks
        .iter()
        .map(|chunk| [chunk.db_index, chunk.chunk_index])
        .collect();
    serde_json::to_string(&compact)
        .map_err(|err| rusqlite::Error::ToSqlConversionFailure(Box::new(err)))
}

fn chunks_from_string(value: String) -> rusqlite::Result<Vec<AssetChunkRef>> {
    let compact: Vec<[i64; 2]> = serde_json::from_str(&value).map_err(|err| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(err))
    })?;
    Ok(compact
        .into_iter()
        .map(|pair| AssetChunkRef {
            db_index: pair[0],
            chunk_index: pair[1],
        })
        .collect())
}

fn normalize_asset_name(name: Option<&str>) -> Option<String> {
    let name = name?.trim();
    if name.is_empty() {
        return None;
    }
    let path = Path::new(name);
    if let Some(file_name) = path.file_name().and_then(|value| value.to_str()) {
        Some(file_name.to_string())
    } else {
        Some(name.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetChunkRef {
    pub db_index: i64,
    pub chunk_index: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AssetMetadataInput {
    pub name: Option<String>,
    pub mime_type: Option<String>,
    pub size: i64,
    pub chunk_size: i64,
    pub chunks: Vec<AssetChunkRef>,
    pub checksum: Option<String>,
    pub raw_file_path: Option<String>,
    pub valid: bool,
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
    pub raw_file_path: Option<String>,
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

pub(crate) fn assets_create_with_id(
    conn: &Connection,
    id: &str,
    asset: &AssetMetadataInput,
) -> rusqlite::Result<()> {
    let chunks_json = chunks_to_string(&asset.chunks)?;
    let name = normalize_asset_name(asset.name.as_deref());
    conn.execute(
        "INSERT INTO assets (id, name, mime_type, size, chunk_size, chunks, checksum, raw_file_path, valid)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            id,
            name.as_deref(),
            asset.mime_type.as_deref(),
            asset.size,
            asset.chunk_size,
            chunks_json,
            asset.checksum.as_deref(),
            asset.raw_file_path.as_deref(),
            if asset.valid { 1 } else { 0 }
        ],
    )?;
    Ok(())
}

pub fn assets_create(conn: &Connection, asset: &AssetMetadataInput) -> rusqlite::Result<String> {
    let asset_id = random_id();
    assets_create_with_id(conn, &asset_id, asset)?;
    Ok(asset_id)
}

pub fn assets_get(conn: &Connection, id: &str) -> rusqlite::Result<Option<AssetMetadata>> {
    conn.query_row(
        "SELECT id, name, mime_type, size, chunk_size, chunks, checksum, raw_file_path, valid, created_at, updated_at
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
                raw_file_path: row.get(7)?,
                valid: row.get::<_, i64>(8)? != 0,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
            })
        },
    )
    .optional()
}

pub fn assets_update(
    conn: &Connection,
    id: &str,
    asset: &AssetMetadataInput,
) -> rusqlite::Result<usize> {
    let chunks_json = chunks_to_string(&asset.chunks)?;
    let name = normalize_asset_name(asset.name.as_deref());
    let valid = if asset.valid { 1 } else { 0 };
    conn.execute(
        "UPDATE assets
         SET name = ?2, mime_type = ?3, size = ?4, chunk_size = ?5, chunks = ?6, checksum = ?7,
             raw_file_path = ?8, valid = ?9
         WHERE id = ?1",
        params![
            id,
            name.as_deref(),
            asset.mime_type.as_deref(),
            asset.size,
            asset.chunk_size,
            chunks_json,
            asset.checksum.as_deref(),
            asset.raw_file_path.as_deref(),
            valid
        ],
    )
}

pub fn assets_delete(conn: &Connection, id: &str) -> rusqlite::Result<usize> {
    conn.execute("DELETE FROM assets WHERE id = ?1", params![id])
}

pub fn assets_list(conn: &Connection) -> rusqlite::Result<Vec<AssetMetadata>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, mime_type, size, chunk_size, chunks, checksum, raw_file_path, valid, created_at, updated_at
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
            raw_file_path: row.get(7)?,
            valid: row.get::<_, i64>(8)? != 0,
            created_at: row.get(9)?,
            updated_at: row.get(10)?,
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
) -> rusqlite::Result<(String, Vec<i64>)> {
    let tx_meta = meta_conn.transaction()?;
    let tx_chunks = chunks_conn.transaction()?;

    let asset_id = assets_create(&tx_meta, asset)?;
    let mut ids = Vec::with_capacity(chunks.len());
    for chunk in chunks {
        let id = asset_chunk_create(&tx_chunks, &asset_id, chunk.chunk_index, &chunk.data)?;
        ids.push(id);
    }

    tx_chunks.commit()?;
    if let Err(err) = tx_meta.commit() {
        let _ = asset_chunks_delete_for_asset(chunks_conn, &asset_id);
        return Err(err);
    }

    Ok((asset_id, ids))
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
    id: &str,
    asset: &AssetMetadataInput,
    chunks: &[AssetChunkInput],
) -> rusqlite::Result<usize> {
    let tx_meta = meta_conn.transaction()?;
    let tx_chunks = chunks_conn.transaction()?;

    let updated = assets_update(&tx_meta, id, asset)?;
    if updated == 0 {
        return Ok(0);
    }

    asset_chunks_delete_for_asset(&tx_chunks, id)?;
    for chunk in chunks {
        asset_chunk_create(&tx_chunks, id, chunk.chunk_index, &chunk.data)?;
    }

    tx_chunks.commit()?;
    if let Err(err) = tx_meta.commit() {
        let _ = asset_chunks_delete_for_asset(chunks_conn, id);
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
    use std::path::Path;

    fn file_name(value: &str) -> String {
        Path::new(value)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(value)
            .to_string()
    }

    #[test]
    fn assets_crud() -> rusqlite::Result<()> {
        let conn = Connection::open_in_memory()?;
        init_main_db(&conn)?;

        let asset_v1 = AssetMetadataInput {
            name: Some("/tmp/file.txt".to_string()),
            mime_type: Some("text/plain".to_string()),
            size: 12,
            chunk_size: 4,
            chunks: vec![AssetChunkRef {
                db_index: 0,
                chunk_index: 0,
            }],
            checksum: Some("abc123".to_string()),
            raw_file_path: Some("/tmp/file.txt".to_string()),
            valid: true,
        };
        let asset_id = assets_create(&conn, &asset_v1)?;

        let fetched = assets_get(&conn, &asset_id)?.expect("missing asset row");
        assert_eq!(fetched.name, Some(file_name("/tmp/file.txt")));
        assert_eq!(fetched.mime_type, asset_v1.mime_type);
        assert_eq!(fetched.size, asset_v1.size);
        assert_eq!(fetched.chunk_size, asset_v1.chunk_size);
        assert_eq!(fetched.chunks, asset_v1.chunks);
        assert_eq!(fetched.checksum, asset_v1.checksum);
        assert_eq!(fetched.raw_file_path, asset_v1.raw_file_path);
        assert_eq!(fetched.valid, asset_v1.valid);

        let list = assets_list(&conn)?;
        assert_eq!(list.len(), 1);

        let asset_v2 = AssetMetadataInput {
            name: Some("/var/tmp/file.bin".to_string()),
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
            raw_file_path: None,
            valid: false,
        };
        let updated = assets_update(&conn, &asset_id, &asset_v2)?;
        assert_eq!(updated, 1);
        let unchanged = assets_update(&conn, &asset_id, &asset_v2)?;
        assert_eq!(unchanged, 0);

        let fetched = assets_get(&conn, &asset_id)?.expect("missing asset row");
        assert_eq!(fetched.name, Some(file_name("/var/tmp/file.bin")));
        assert_eq!(fetched.mime_type, asset_v2.mime_type);
        assert_eq!(fetched.size, asset_v2.size);
        assert_eq!(fetched.chunk_size, asset_v2.chunk_size);
        assert_eq!(fetched.chunks, asset_v2.chunks);
        assert_eq!(fetched.checksum, asset_v2.checksum);
        assert_eq!(fetched.raw_file_path, asset_v2.raw_file_path);
        assert_eq!(fetched.valid, asset_v2.valid);

        let deleted = assets_delete(&conn, &asset_id)?;
        assert_eq!(deleted, 1);
        assert!(assets_get(&conn, &asset_id)?.is_none());
        Ok(())
    }

    #[test]
    fn assets_with_chunks_crud() -> rusqlite::Result<()> {
        let mut meta_conn = Connection::open_in_memory()?;
        let mut chunks_conn = Connection::open_in_memory()?;
        init_main_db(&meta_conn)?;
        init_asset_chunk_db(&chunks_conn)?;

        let asset = AssetMetadataInput {
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
            raw_file_path: None,
            valid: false,
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
        let (asset_id, ids) =
            assets_create_with_chunks(&mut meta_conn, &mut chunks_conn, &asset, &chunks)?;
        assert_eq!(ids.len(), 2);

        let fetched =
            assets_get_with_chunks(&meta_conn, &chunks_conn, &asset_id)?.expect("missing asset");
        assert_eq!(fetched.metadata.name, asset.name);
        assert_eq!(fetched.metadata.size, asset.size);
        assert_eq!(fetched.chunks.len(), 2);
        assert_eq!(fetched.chunks[0].chunk_index, 0);
        assert_eq!(fetched.chunks[0].data, vec![1, 2, 3]);

        let asset_v2 = AssetMetadataInput {
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
            raw_file_path: Some("/tmp/blob-v2.bin".to_string()),
            valid: true,
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
            assets_update_with_chunks(&mut meta_conn, &mut chunks_conn, &asset_id, &asset_v2, &chunks_v2)?;
        assert_eq!(updated, 1);

        let fetched =
            assets_get_with_chunks(&meta_conn, &chunks_conn, &asset_id)?.expect("missing asset");
        assert_eq!(fetched.metadata.name, asset_v2.name);
        assert_eq!(fetched.metadata.size, asset_v2.size);
        assert_eq!(fetched.metadata.checksum, asset_v2.checksum);
        assert_eq!(fetched.chunks.len(), 2);
        assert_eq!(fetched.chunks[0].data, vec![9, 9]);
        assert_eq!(fetched.chunks[1].data, vec![8, 8]);

        let deleted = assets_delete_with_chunks(&mut meta_conn, &mut chunks_conn, &asset_id)?;
        assert_eq!(deleted, 1);
        assert!(assets_get(&meta_conn, &asset_id)?.is_none());
        let remaining = asset_chunks_for_asset(&chunks_conn, &asset_id)?;
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
