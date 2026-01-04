# play-db

SQLite-backed storage helpers for key/value, JSON documents, and chunked assets.

## Setup

```rust
use rusqlite::Connection;
use play_db::{init_asset_chunk_db, init_main_db};

let conn = Connection::open_in_memory()?;
init_main_db(&conn)?;

let chunk_conn = Connection::open_in_memory()?;
init_asset_chunk_db(&chunk_conn)?;
```

## KV API

```rust
use play_db::{kv_create, kv_get, kv_update, kv_delete, kv_list};

kv_create(&conn, "foo", "bar")?;
let value = kv_get(&conn, "foo")?.unwrap();
kv_update(&conn, "foo", "baz")?;
kv_delete(&conn, "foo")?;
let rows = kv_list(&conn)?;
```

## Docs API

Docs are stored as JSON. `docs_create` generates an id in the form
`{doc_type}_{short_uuid}` and returns it. `docs_get` / `docs_list` return a
flattened map that includes system fields (`id`, `created_at`, `updated_at`)
merged into the JSON document.

```rust
use serde_json::json;
use play_db::{docs_create, docs_get, docs_patch, docs_query, DocsJsonFilter};

let doc_id = docs_create(&conn, "note", &json!({"title": "Hi", "count": 1}))?;

let doc = docs_get(&conn, &doc_id)?.unwrap();
assert_eq!(doc.get("title").and_then(|v| v.as_str()), Some("Hi"));

docs_patch(&conn, &doc_id, &json!({"count": 2, "old": null}))?;

let results = docs_query(
    &conn,
    &[DocsJsonFilter::Eq {
        path: "$.count".to_string(),
        value: json!(2),
    }],
    None,
    None,
    None,
)?;
```

## Assets + Chunks

Asset metadata lives in the main DB. Chunk data lives in shard DBs. The asset
metadata contains `chunks: Vec<AssetChunkRef>` with `{ db_index, chunk_index }`
so you can locate each chunk.

```rust
use play_db::{
    AssetChunkInput, AssetChunkRef, AssetMetadataInput, assets_create, assets_get,
    asset_chunk_create, asset_chunk_get,
};
use serde_json::json;

let meta = AssetMetadataInput {
    id: "asset-1".to_string(),
    name: Some("file.bin".to_string()),
    mime_type: Some("application/octet-stream".to_string()),
    size: 6,
    chunk_size: 3,
    chunks: vec![
        AssetChunkRef { db_index: 0, chunk_index: 0 },
        AssetChunkRef { db_index: 0, chunk_index: 1 },
    ],
    checksum: None,
};
assets_create(&conn, &meta)?;
let stored = assets_get(&conn, "asset-1")?.unwrap();

let chunk_id = asset_chunk_create(&chunk_conn, "asset-1", 0, &[1, 2, 3])?;
let chunk = asset_chunk_get(&chunk_conn, chunk_id)?.unwrap();
```

## Asset Service (Async)

The async service reads a file, chunks it, inserts chunks in parallel, stores
metadata, and returns `AssetMetadata` with `valid = true` when chunk data and
checksum match.

```rust
use std::path::PathBuf;
use play_db::{insert_asset_from_path_with_size_str, read_asset_bytes, read_asset_to_file};

let metadata = insert_asset_from_path_with_size_str(
    "/tmp/main.db",
    vec![PathBuf::from("/tmp/chunk0.db"), PathBuf::from("/tmp/chunk1.db")],
    "asset-42",
    Some("video.mp4".to_string()),
    None,          // auto-detect mime type
    "100KB",
    "/path/to/video.mp4",
).await?;

let bytes = read_asset_bytes(
    "/tmp/main.db",
    vec![PathBuf::from("/tmp/chunk0.db"), PathBuf::from("/tmp/chunk1.db")],
    "asset-42",
).await?;

let output = read_asset_to_file(
    "/tmp/main.db",
    vec![PathBuf::from("/tmp/chunk0.db"), PathBuf::from("/tmp/chunk1.db")],
    "asset-42",
    "/tmp/video_out.mp4",
).await?;
```

## Tests

```bash
cargo test
```

Note: `asset_service_inserts_real_file_with_mime` reads a local MP4 file from
`/Users/zhouzhipeng/Downloads/...`. Update the path or skip it with:

```bash
cargo test -- --skip asset_service_inserts_real_file_with_mime
```
