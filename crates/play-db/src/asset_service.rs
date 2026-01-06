use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use tokio::task;

use crate::{
    asset_chunk_create, asset_chunks_delete_for_asset, assets_get, init_asset_chunk_db,
    init_main_db, random_id, AssetChunkRef, AssetMetadata, AssetMetadataInput,
};
use crate::assets::assets_create_with_id;
use rusqlite::types::Value as SqlValue;

#[derive(Debug, Clone)]
struct ChunkJob {
    chunk_index: i64,
    data: Vec<u8>,
}

fn wrap_error<E>(err: E) -> rusqlite::Error
where
    E: std::error::Error + Send + Sync + 'static,
{
    rusqlite::Error::ToSqlConversionFailure(Box::new(err))
}

fn service_error(message: &str) -> rusqlite::Error {
    wrap_error(std::io::Error::new(std::io::ErrorKind::InvalidInput, message))
}

fn checksum_for_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

fn detect_mime_type(data: &[u8]) -> Option<String> {
    infer::get(data).map(|kind| kind.mime_type().to_string())
}

fn parse_chunk_size(value: &str) -> rusqlite::Result<i64> {
    let raw = value.trim();
    if raw.is_empty() {
        return Err(service_error("chunk_size must not be empty"));
    }
    let split_at = raw
        .find(|ch: char| !ch.is_ascii_digit())
        .unwrap_or_else(|| raw.len());
    if split_at == 0 {
        return Err(service_error("chunk_size missing numeric prefix"));
    }
    let number: i64 = raw[..split_at]
        .parse()
        .map_err(|err| wrap_error(err))?;
    let unit = raw[split_at..].trim().to_ascii_uppercase();
    let multiplier = match unit.as_str() {
        "" | "B" => 1_i64,
        "K" | "KB" => 1024_i64,
        "M" | "MB" => 1024_i64 * 1024,
        "G" | "GB" => 1024_i64 * 1024 * 1024,
        _ => return Err(service_error("unsupported chunk_size unit")),
    };
    let bytes = number
        .checked_mul(multiplier)
        .ok_or_else(|| service_error("chunk_size overflow"))?;
    if bytes <= 0 {
        return Err(service_error("chunk_size must be positive"));
    }
    Ok(bytes)
}

fn expected_chunk_count(size: i64, chunk_size: i64) -> i64 {
    if size <= 0 {
        0
    } else {
        (size + chunk_size - 1) / chunk_size
    }
}

async fn cleanup_chunks(asset_id: &str, chunk_db_paths: &[PathBuf]) {
    let mut handles = Vec::with_capacity(chunk_db_paths.len());
    for path in chunk_db_paths {
        let path = path.clone();
        let asset_id = asset_id.to_string();
        handles.push(task::spawn_blocking(move || -> rusqlite::Result<()> {
            let conn = rusqlite::Connection::open(path)?;
            init_asset_chunk_db(&conn)?;
            asset_chunks_delete_for_asset(&conn, &asset_id)?;
            Ok(())
        }));
    }
    for handle in handles {
        let _ = handle.await;
    }
}

fn ensure_chunk_refs_valid(
    chunk_refs: &[AssetChunkRef],
    chunk_db_paths: &[PathBuf],
    size: i64,
    chunk_size: i64,
) -> rusqlite::Result<i64> {
    let expected = expected_chunk_count(size, chunk_size);
    if chunk_refs.len() as i64 != expected {
        return Err(service_error("chunk refs count mismatch"));
    }
    let mut indices: Vec<i64> = chunk_refs.iter().map(|chunk| chunk.chunk_index).collect();
    indices.sort();
    indices.dedup();
    if indices.len() != chunk_refs.len() {
        return Err(service_error("duplicate chunk_index in refs"));
    }
    let expected_indices: Vec<i64> = (0..expected).collect();
    if indices != expected_indices {
        return Err(service_error("chunk_index sequence mismatch"));
    }
    for chunk in chunk_refs {
        if chunk.db_index < 0 || chunk.db_index as usize >= chunk_db_paths.len() {
            return Err(service_error("chunk db_index out of range"));
        }
    }
    Ok(expected)
}

fn read_chunks_for_db(
    db_path: PathBuf,
    asset_id: String,
    chunk_indices: Vec<i64>,
) -> rusqlite::Result<Vec<(i64, Vec<u8>)>> {
    if chunk_indices.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders = std::iter::repeat("?")
        .take(chunk_indices.len())
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "SELECT chunk_index, data FROM asset_chunks WHERE asset_id = ?1 AND chunk_index IN ({})",
        placeholders
    );
    let mut params: Vec<SqlValue> = Vec::with_capacity(1 + chunk_indices.len());
    params.push(SqlValue::Text(asset_id));
    for index in chunk_indices {
        params.push(SqlValue::Integer(index));
    }
    let conn = rusqlite::Connection::open(db_path)?;
    init_asset_chunk_db(&conn)?;
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::params_from_iter(params), |row| {
        Ok((row.get(0)?, row.get(1)?))
    })?;
    rows.collect()
}

pub async fn insert_asset_from_path(
    meta_db_path: impl AsRef<Path>,
    chunk_db_paths: Vec<PathBuf>,
    name: Option<String>,
    mime_type: Option<String>,
    chunk_size: i64,
    file_path: impl AsRef<Path>,
) -> rusqlite::Result<AssetMetadata> {
    if chunk_db_paths.is_empty() {
        return Err(service_error("chunk_db_paths must not be empty"));
    }
    if chunk_size <= 0 {
        return Err(service_error("chunk_size must be positive"));
    }

    let meta_db_path = meta_db_path.as_ref().to_path_buf();
    let file_path = file_path.as_ref().to_path_buf();
    let asset_id = random_id();
    let chunk_size_usize = usize::try_from(chunk_size).map_err(wrap_error)?;

    let data = tokio::fs::read(&file_path).await.map_err(wrap_error)?;
    let size = i64::try_from(data.len()).map_err(wrap_error)?;
    let checksum = checksum_for_bytes(&data);
    let resolved_mime = match mime_type {
        Some(value) if !value.trim().is_empty() => Some(value),
        _ => detect_mime_type(&data).or_else(|| Some("application/octet-stream".to_string())),
    };

    let mut chunk_refs = Vec::new();
    let mut jobs_by_db: Vec<Vec<ChunkJob>> = vec![Vec::new(); chunk_db_paths.len()];
    for (index, chunk) in data.chunks(chunk_size_usize).enumerate() {
        let db_index = index % chunk_db_paths.len();
        let chunk_index = index as i64;
        jobs_by_db[db_index].push(ChunkJob {
            chunk_index,
            data: chunk.to_vec(),
        });
        chunk_refs.push(AssetChunkRef {
            db_index: db_index as i64,
            chunk_index,
        });
    }

    let chunk_db_paths_for_cleanup = chunk_db_paths.clone();
    let mut handles = Vec::with_capacity(chunk_db_paths.len());
    for (db_path, jobs) in chunk_db_paths.into_iter().zip(jobs_by_db.into_iter()) {
        let asset_id = asset_id.clone();
        handles.push(task::spawn_blocking(move || -> rusqlite::Result<i64> {
            let mut conn = rusqlite::Connection::open(db_path)?;
            init_asset_chunk_db(&conn)?;
            let tx = conn.transaction()?;
            let count = jobs.len() as i64;
            for job in jobs {
                asset_chunk_create(&tx, &asset_id, job.chunk_index, &job.data)?;
            }
            tx.commit()?;
            Ok(count)
        }));
    }

    let mut inserted_total = 0;
    for handle in handles {
        match handle.await {
            Ok(Ok(count)) => inserted_total += count,
            Ok(Err(err)) => {
                cleanup_chunks(&asset_id, &chunk_db_paths_for_cleanup).await;
                return Err(err);
            }
            Err(err) => {
                cleanup_chunks(&asset_id, &chunk_db_paths_for_cleanup).await;
                return Err(wrap_error(err));
            }
        }
    }

    let expected = expected_chunk_count(size, chunk_size);
    let valid = inserted_total == expected;
    let raw_file_path = Some(file_path.to_string_lossy().into_owned());
    let asset_input = AssetMetadataInput {
        name,
        mime_type: resolved_mime,
        size,
        chunk_size,
        chunks: chunk_refs,
        checksum: Some(checksum.clone()),
        raw_file_path: raw_file_path.clone(),
        valid,
    };
    let asset_input_for_db = asset_input.clone();
    let asset_id_for_db = asset_id.clone();
    let meta_db_path_for_db = meta_db_path.clone();
    let meta_insert = task::spawn_blocking(move || -> rusqlite::Result<()> {
        let conn = rusqlite::Connection::open(meta_db_path_for_db)?;
        init_main_db(&conn)?;
        assets_create_with_id(&conn, &asset_id_for_db, &asset_input_for_db)?;
        Ok(())
    })
    .await;
    match meta_insert {
        Ok(Ok(())) => {}
        Ok(Err(err)) => {
            cleanup_chunks(&asset_id, &chunk_db_paths_for_cleanup).await;
            return Err(err);
        }
        Err(err) => {
            cleanup_chunks(&asset_id, &chunk_db_paths_for_cleanup).await;
            return Err(wrap_error(err));
        }
    }

    let meta_db_path_for_get = meta_db_path.clone();
    let asset_id_for_get = asset_id.clone();
    let metadata = task::spawn_blocking(move || -> rusqlite::Result<Option<AssetMetadata>> {
        let conn = rusqlite::Connection::open(meta_db_path_for_get)?;
        assets_get(&conn, &asset_id_for_get)
    })
    .await
    .map_err(wrap_error)??;
    let mut metadata = metadata.ok_or_else(|| service_error("asset metadata missing"))?;
    metadata.valid = valid;
    metadata.raw_file_path = raw_file_path;
    Ok(metadata)
}

pub async fn insert_asset_from_bytes(
    meta_db_path: impl AsRef<Path>,
    chunk_db_paths: Vec<PathBuf>,
    name: Option<String>,
    mime_type: Option<String>,
    chunk_size: i64,
    data: Vec<u8>,
) -> rusqlite::Result<AssetMetadata> {
    if chunk_db_paths.is_empty() {
        return Err(service_error("chunk_db_paths must not be empty"));
    }
    if chunk_size <= 0 {
        return Err(service_error("chunk_size must be positive"));
    }

    let meta_db_path = meta_db_path.as_ref().to_path_buf();
    let asset_id = random_id();
    let chunk_size_usize = usize::try_from(chunk_size).map_err(wrap_error)?;

    let size = i64::try_from(data.len()).map_err(wrap_error)?;
    let checksum = checksum_for_bytes(&data);
    let resolved_mime = match mime_type {
        Some(value) if !value.trim().is_empty() => Some(value),
        _ => detect_mime_type(&data).or_else(|| Some("application/octet-stream".to_string())),
    };

    let mut chunk_refs = Vec::new();
    let mut jobs_by_db: Vec<Vec<ChunkJob>> = vec![Vec::new(); chunk_db_paths.len()];
    for (index, chunk) in data.chunks(chunk_size_usize).enumerate() {
        let db_index = index % chunk_db_paths.len();
        let chunk_index = index as i64;
        jobs_by_db[db_index].push(ChunkJob {
            chunk_index,
            data: chunk.to_vec(),
        });
        chunk_refs.push(AssetChunkRef {
            db_index: db_index as i64,
            chunk_index,
        });
    }

    let chunk_db_paths_for_cleanup = chunk_db_paths.clone();
    let mut handles = Vec::with_capacity(chunk_db_paths.len());
    for (db_path, jobs) in chunk_db_paths.into_iter().zip(jobs_by_db.into_iter()) {
        let asset_id = asset_id.clone();
        handles.push(task::spawn_blocking(move || -> rusqlite::Result<i64> {
            let mut conn = rusqlite::Connection::open(db_path)?;
            init_asset_chunk_db(&conn)?;
            let tx = conn.transaction()?;
            let count = jobs.len() as i64;
            for job in jobs {
                asset_chunk_create(&tx, &asset_id, job.chunk_index, &job.data)?;
            }
            tx.commit()?;
            Ok(count)
        }));
    }

    let mut inserted_total = 0;
    for handle in handles {
        match handle.await {
            Ok(Ok(count)) => inserted_total += count,
            Ok(Err(err)) => {
                cleanup_chunks(&asset_id, &chunk_db_paths_for_cleanup).await;
                return Err(err);
            }
            Err(err) => {
                cleanup_chunks(&asset_id, &chunk_db_paths_for_cleanup).await;
                return Err(wrap_error(err));
            }
        }
    }

    let expected = expected_chunk_count(size, chunk_size);
    let valid = inserted_total == expected;
    let asset_input = AssetMetadataInput {
        name,
        mime_type: resolved_mime,
        size,
        chunk_size,
        chunks: chunk_refs,
        checksum: Some(checksum.clone()),
        raw_file_path: None,
        valid,
    };
    let asset_input_for_db = asset_input.clone();
    let asset_id_for_db = asset_id.clone();
    let meta_db_path_for_db = meta_db_path.clone();
    let meta_insert = task::spawn_blocking(move || -> rusqlite::Result<()> {
        let conn = rusqlite::Connection::open(meta_db_path_for_db)?;
        init_main_db(&conn)?;
        assets_create_with_id(&conn, &asset_id_for_db, &asset_input_for_db)?;
        Ok(())
    })
    .await;
    match meta_insert {
        Ok(Ok(())) => {}
        Ok(Err(err)) => {
            cleanup_chunks(&asset_id, &chunk_db_paths_for_cleanup).await;
            return Err(err);
        }
        Err(err) => {
            cleanup_chunks(&asset_id, &chunk_db_paths_for_cleanup).await;
            return Err(wrap_error(err));
        }
    }

    let meta_db_path_for_get = meta_db_path.clone();
    let asset_id_for_get = asset_id.clone();
    let metadata = task::spawn_blocking(move || -> rusqlite::Result<Option<AssetMetadata>> {
        let conn = rusqlite::Connection::open(meta_db_path_for_get)?;
        assets_get(&conn, &asset_id_for_get)
    })
    .await
    .map_err(wrap_error)??;
    let mut metadata = metadata.ok_or_else(|| service_error("asset metadata missing"))?;
    metadata.valid = valid;
    metadata.raw_file_path = None;
    Ok(metadata)
}

pub async fn insert_asset_from_path_with_size_str(
    meta_db_path: impl AsRef<Path>,
    chunk_db_paths: Vec<PathBuf>,
    name: Option<String>,
    mime_type: Option<String>,
    chunk_size: &str,
    file_path: impl AsRef<Path>,
) -> rusqlite::Result<AssetMetadata> {
    let chunk_size = parse_chunk_size(chunk_size)?;
    insert_asset_from_path(
        meta_db_path,
        chunk_db_paths,
        name,
        mime_type,
        chunk_size,
        file_path,
    )
    .await
}

pub async fn insert_asset_from_bytes_with_size_str(
    meta_db_path: impl AsRef<Path>,
    chunk_db_paths: Vec<PathBuf>,
    name: Option<String>,
    mime_type: Option<String>,
    chunk_size: &str,
    data: Vec<u8>,
) -> rusqlite::Result<AssetMetadata> {
    let chunk_size = parse_chunk_size(chunk_size)?;
    insert_asset_from_bytes(
        meta_db_path,
        chunk_db_paths,
        name,
        mime_type,
        chunk_size,
        data,
    )
    .await
}

pub async fn read_asset_bytes(
    meta_db_path: impl AsRef<Path>,
    chunk_db_paths: Vec<PathBuf>,
    asset_id: &str,
) -> rusqlite::Result<Vec<u8>> {
    if chunk_db_paths.is_empty() {
        return Err(service_error("chunk_db_paths must not be empty"));
    }

    let meta_db_path = meta_db_path.as_ref().to_path_buf();
    let asset_id = asset_id.to_string();
    let metadata = task::spawn_blocking(move || -> rusqlite::Result<Option<AssetMetadata>> {
        let conn = rusqlite::Connection::open(meta_db_path)?;
        assets_get(&conn, &asset_id)
    })
    .await
    .map_err(wrap_error)??;
    let metadata = metadata.ok_or_else(|| service_error("asset metadata missing"))?;
    let expected = ensure_chunk_refs_valid(&metadata.chunks, &chunk_db_paths, metadata.size, metadata.chunk_size)?;

    let mut indices_by_db = vec![Vec::new(); chunk_db_paths.len()];
    for chunk_ref in &metadata.chunks {
        indices_by_db[chunk_ref.db_index as usize].push(chunk_ref.chunk_index);
    }

    let mut handles = Vec::with_capacity(chunk_db_paths.len());
    for (db_path, indices) in chunk_db_paths.into_iter().zip(indices_by_db.into_iter()) {
        let asset_id = metadata.id.clone();
        handles.push(task::spawn_blocking(move || read_chunks_for_db(db_path, asset_id, indices)));
    }

    let mut chunk_map: std::collections::HashMap<i64, Vec<u8>> = std::collections::HashMap::new();
    for handle in handles {
        let chunk_rows = handle.await.map_err(wrap_error)??;
        for (chunk_index, data) in chunk_rows {
            chunk_map.insert(chunk_index, data);
        }
    }

    if chunk_map.len() as i64 != expected {
        return Err(service_error("missing chunk data"));
    }

    let mut combined = Vec::with_capacity(metadata.size as usize);
    for index in 0..expected {
        let data = chunk_map
            .get(&index)
            .ok_or_else(|| service_error("chunk data missing"))?;
        combined.extend_from_slice(data);
    }

    if combined.len() as i64 != metadata.size {
        return Err(service_error("combined size mismatch"));
    }

    if let Some(expected_checksum) = metadata.checksum.as_ref() {
        let actual_checksum = checksum_for_bytes(&combined);
        if !actual_checksum.eq_ignore_ascii_case(expected_checksum) {
            return Err(service_error("checksum mismatch"));
        }
    }

    Ok(combined)
}

pub async fn read_asset_to_file(
    meta_db_path: impl AsRef<Path>,
    chunk_db_paths: Vec<PathBuf>,
    asset_id: &str,
    output_path: impl AsRef<Path>,
) -> rusqlite::Result<PathBuf> {
    let bytes = read_asset_bytes(meta_db_path, chunk_db_paths, asset_id).await?;
    let output_path = output_path.as_ref().to_path_buf();
    tokio::fs::write(&output_path, bytes)
        .await
        .map_err(wrap_error)?;
    Ok(output_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    use tokio::fs;
    use uuid::Uuid;

    fn checksum_for_bytes(bytes: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        hex::encode(hasher.finalize())
    }

    #[tokio::test]
    async fn asset_service_inserts_and_marks_valid() -> rusqlite::Result<()> {
        let dir = std::env::temp_dir().join(format!(
            "play_db_asset_service_{}",
            Uuid::new_v4().as_simple()
        ));
        fs::create_dir_all(&dir).await.map_err(wrap_error)?;

        let main_db = dir.join("main.db");
        let chunk_db0 = dir.join("chunk0.db");
        let chunk_db1 = dir.join("chunk1.db");
        let file_path = dir.join("asset.bin");
        let data = b"hello world chunked data";
        fs::write(&file_path, data).await.map_err(wrap_error)?;

        let metadata = insert_asset_from_path(
            &main_db,
            vec![chunk_db0.clone(), chunk_db1.clone()],
            Some("asset.bin".to_string()),
            None,
            4,
            &file_path,
        )
        .await?;
        let asset_id = metadata.id.clone();

        let expected_checksum = checksum_for_bytes(data);
        assert_eq!(metadata.size, data.len() as i64);
        assert_eq!(metadata.checksum.as_deref(), Some(expected_checksum.as_str()));
        assert!(metadata.valid);
        assert_eq!(
            metadata.raw_file_path.as_deref(),
            Some(file_path.to_string_lossy().as_ref())
        );
        assert_eq!(
            metadata.chunks.len() as i64,
            expected_chunk_count(metadata.size, metadata.chunk_size)
        );

        let combined =
            read_asset_bytes(&main_db, vec![chunk_db0.clone(), chunk_db1.clone()], &asset_id)
                .await?;
        assert_eq!(combined, data);

        let output_path = dir.join("asset_out.bin");
        let written_path =
            read_asset_to_file(&main_db, vec![chunk_db0, chunk_db1], &asset_id, &output_path)
                .await?;
        assert_eq!(written_path, output_path);
        let written = fs::read(&written_path).await.map_err(wrap_error)?;
        assert_eq!(written, data);

        let _ = fs::remove_dir_all(&dir).await;
        Ok(())
    }

    #[tokio::test]
    async fn asset_service_inserts_real_file_with_mime() -> rusqlite::Result<()> {
        let file_path = PathBuf::from(
            "/Users/zhouzhipeng/Downloads/kling_20260104_图生视频_2D卡通风格_幼儿动_2958_0.mp4",
        );
        let data = fs::read(&file_path).await.map_err(wrap_error)?;
        let expected_mime = detect_mime_type(&data).unwrap_or_else(|| "application/octet-stream".to_string());

        let dir = std::env::temp_dir().join(format!(
            "play_db_asset_service_real_{}",
            Uuid::new_v4().as_simple()
        ));
        fs::create_dir_all(&dir).await.map_err(wrap_error)?;
        let main_db = dir.join("main.db");
        let chunk_db0 = dir.join("chunk0.db");
        let chunk_db1 = dir.join("chunk1.db");
        let metadata = insert_asset_from_path_with_size_str(
            &main_db,
            vec![chunk_db0.clone(), chunk_db1.clone()],
            Some("kling.mp4".to_string()),
            None,
            "100KB",
            &file_path,
        )
        .await?;
        let asset_id = metadata.id.clone();

        assert_eq!(metadata.mime_type.as_deref(), Some(expected_mime.as_str()));
        assert_eq!(metadata.chunk_size, 1024 * 100);
        assert!(metadata.valid);
        assert_eq!(
            metadata.raw_file_path.as_deref(),
            Some(file_path.to_string_lossy().as_ref())
        );

        let combined =
            read_asset_bytes(&main_db, vec![chunk_db0.clone(), chunk_db1.clone()], &asset_id)
                .await?;
        assert_eq!(combined, data);

        let output_path = dir.join("kling_out.mp4");
        let written_path =
            read_asset_to_file(&main_db, vec![chunk_db0, chunk_db1], &asset_id, &output_path)
                .await?;
        println!("output_path : {:?}", output_path);
        assert_eq!(written_path, output_path);
        let written = fs::read(&written_path).await.map_err(wrap_error)?;
        assert_eq!(written, data);

        let _ = fs::remove_dir_all(&dir).await;
        Ok(())
    }
}
