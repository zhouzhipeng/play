-- =========================================================
-- ASSET SHARD DATABASE SCHEMA (asset_xx.db)
-- Execute the following separately on each asset shard DB
-- =========================================================

PRAGMA journal_mode = OFF;

CREATE TABLE IF NOT EXISTS asset_chunks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    asset_id TEXT NOT NULL,
    chunk_index INTEGER NOT NULL,
    data BLOB NOT NULL,
    size INTEGER NOT NULL CHECK (size > 0),
    created_at INTEGER NOT NULL DEFAULT (CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER))
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_asset_chunk_unique
ON asset_chunks(asset_id, chunk_index);

CREATE INDEX IF NOT EXISTS idx_asset_chunks_asset_id
ON asset_chunks(asset_id);
