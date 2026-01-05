-- =========================================================
-- SQLite Schema: KV / Docs / Assets with Chunked Storage
-- Author: Optimized Version
-- Description:
--   Enhanced schema with automatic timestamp management via triggers.
--
--   Features:
--     1. Automatic created_at/updated_at management
--     2. Optimized indexing strategy
--     3. Asset binary data stored in sharded databases
-- =========================================================

PRAGMA foreign_keys = ON;
PRAGMA journal_mode = OFF;

-- =========================================================
-- 1. KV TABLE
-- Simple key-value storage with automatic timestamp management
-- Timestamps are in milliseconds
-- =========================================================
CREATE TABLE IF NOT EXISTS kv
(
    key        TEXT PRIMARY KEY,
    value      TEXT    NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)),
    updated_at INTEGER NOT NULL DEFAULT (CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER))
)STRICT;

-- =========================================================
-- 2. DOCS TABLE
-- JSON document storage with dynamic schema
-- Timestamps are in milliseconds
-- =========================================================
CREATE TABLE IF NOT EXISTS docs
(
    id         TEXT PRIMARY KEY,
    tag        TEXT    NOT NULL,
    doc        TEXT    NOT NULL CHECK (json_valid(doc)),
    created_at INTEGER NOT NULL DEFAULT (CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)),
    updated_at INTEGER NOT NULL DEFAULT (CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER))
)STRICT;

CREATE INDEX IF NOT EXISTS idx_docs_tag
    ON docs (tag);

-- =========================================================
-- 3. ASSETS TABLE (METADATA ONLY)
-- Binary data is stored in asset shard databases
-- Timestamps are in milliseconds
-- =========================================================
CREATE TABLE IF NOT EXISTS assets
(
    id         TEXT PRIMARY KEY,
    name       TEXT,
    mime_type  TEXT,
    size       INTEGER NOT NULL CHECK (size >= 0),
    chunk_size INTEGER NOT NULL CHECK (chunk_size > 0),
    chunks     TEXT    NOT NULL CHECK (json_valid(chunks)),
    checksum   TEXT,
    raw_file_path TEXT,
    valid      INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL DEFAULT (CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)),
    updated_at INTEGER NOT NULL DEFAULT (CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER))
)STRICT;

CREATE INDEX IF NOT EXISTS idx_assets_mime_type
    ON assets (mime_type);

CREATE INDEX IF NOT EXISTS idx_assets_created_at
    ON assets (created_at DESC);



-- =========================================================
-- TRIGGERS FOR KV TABLE
-- =========================================================

-- Trigger: Auto-update updated_at on UPDATE
CREATE TRIGGER IF NOT EXISTS trg_kv_updated_at
    AFTER UPDATE
    ON kv
    FOR EACH ROW
BEGIN
    UPDATE kv
    SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)
    WHERE key = NEW.key;
END;

-- =========================================================
-- TRIGGERS FOR DOCS TABLE
-- =========================================================

-- Trigger: Auto-update updated_at on UPDATE
CREATE TRIGGER IF NOT EXISTS trg_docs_updated_at
    AFTER UPDATE
    ON docs
    FOR EACH ROW
BEGIN
    UPDATE docs
    SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)
    WHERE id = NEW.id;
END;

-- =========================================================
-- TRIGGERS FOR ASSETS TABLE
-- =========================================================

-- Trigger: Auto-update updated_at on UPDATE
CREATE TRIGGER IF NOT EXISTS trg_assets_updated_at
    AFTER UPDATE
    ON assets
    FOR EACH ROW
BEGIN
    UPDATE assets
    SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)
    WHERE id = NEW.id;
END;



-- =========================================================
-- UTILITY VIEWS (OPTIONAL)
-- =========================================================

-- View: Asset summary
CREATE VIEW IF NOT EXISTS v_assets_summary AS
SELECT id,
       name,
       mime_type,
       size,
       ROUND(size / 1024.0 / 1024.0, 2)                      as size_mb,
       datetime(created_at / 1000, 'unixepoch', 'localtime') as created_at_readable,
       datetime(updated_at / 1000, 'unixepoch', 'localtime') as updated_at_readable
FROM assets
ORDER BY created_at DESC;
