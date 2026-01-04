-- =========================================================
-- SQLite Schema: KV / Docs / Assets with Changelog & Chunked Storage
-- Author: Optimized Version
-- Description:
--   Enhanced schema with automatic timestamp management via triggers
--   and changelog tracking for kv, docs, and assets tables.
--
--   Features:
--     1. Automatic created_at/updated_at management
--     2. Automatic changelog recording via triggers
--     3. Optimized indexing strategy
--     4. Asset binary data stored in sharded databases
-- =========================================================

PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;
-- Better concurrency

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
);

-- =========================================================
-- 2. KV CHANGELOG TABLE
-- Records every insert/update/delete on kv table
-- Timestamps are in milliseconds
-- =========================================================
CREATE TABLE IF NOT EXISTS kv_changelog
(
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    key        TEXT    NOT NULL,
    old_value  TEXT,
    new_value  TEXT,
    op         TEXT    NOT NULL CHECK (op IN ('INSERT', 'UPDATE', 'DELETE')),
    changed_at INTEGER NOT NULL DEFAULT (CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER))
);

CREATE INDEX IF NOT EXISTS idx_kv_changelog_key
    ON kv_changelog (key);

CREATE INDEX IF NOT EXISTS idx_kv_changelog_time
    ON kv_changelog (changed_at DESC);

-- =========================================================
-- 3. DOCS TABLE
-- JSON document storage with dynamic schema
-- Timestamps are in milliseconds
-- =========================================================
CREATE TABLE IF NOT EXISTS docs
(
    id         TEXT PRIMARY KEY,
    doc        JSON    NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)),
    updated_at INTEGER NOT NULL DEFAULT (CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER))
);

-- =========================================================
-- 4. DOCS CHANGELOG TABLE
-- Records every insert/update/delete on docs table
-- Timestamps are in milliseconds
-- =========================================================
CREATE TABLE IF NOT EXISTS docs_changelog
(
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    doc_id     TEXT    NOT NULL,
    old_doc    JSON,
    new_doc    JSON,
    op         TEXT    NOT NULL CHECK (op IN ('INSERT', 'UPDATE', 'DELETE')),
    changed_at INTEGER NOT NULL DEFAULT (CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER))
);

CREATE INDEX IF NOT EXISTS idx_docs_changelog_doc_id
    ON docs_changelog (doc_id);

CREATE INDEX IF NOT EXISTS idx_docs_changelog_time
    ON docs_changelog (changed_at DESC);

-- =========================================================
-- 5. ASSETS TABLE (METADATA ONLY)
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
    chunks     JSON    NOT NULL,
    checksum   TEXT,
    created_at INTEGER NOT NULL DEFAULT (CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)),
    updated_at INTEGER NOT NULL DEFAULT (CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER))
);

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

-- Trigger: Log INSERT operations
CREATE TRIGGER IF NOT EXISTS trg_kv_insert_log
    AFTER INSERT
    ON kv
    FOR EACH ROW
BEGIN
    INSERT INTO kv_changelog (key, old_value, new_value, op, changed_at)
    VALUES (NEW.key, NULL, NEW.value, 'INSERT', CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER));
END;

-- Trigger: Log UPDATE operations
CREATE TRIGGER IF NOT EXISTS trg_kv_update_log
    AFTER UPDATE
    ON kv
    FOR EACH ROW
BEGIN
    INSERT INTO kv_changelog (key, old_value, new_value, op, changed_at)
    VALUES (NEW.key, OLD.value, NEW.value, 'UPDATE', CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER));
END;

-- Trigger: Log DELETE operations
CREATE TRIGGER IF NOT EXISTS trg_kv_delete_log
    AFTER DELETE
    ON kv
    FOR EACH ROW
BEGIN
    INSERT INTO kv_changelog (key, old_value, new_value, op, changed_at)
    VALUES (OLD.key, OLD.value, NULL, 'DELETE', CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER));
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

-- Trigger: Log INSERT operations
CREATE TRIGGER IF NOT EXISTS trg_docs_insert_log
    AFTER INSERT
    ON docs
    FOR EACH ROW
BEGIN
    INSERT INTO docs_changelog (doc_id, old_doc, new_doc, op, changed_at)
    VALUES (NEW.id, NULL, NEW.doc, 'INSERT', CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER));
END;

-- Trigger: Log UPDATE operations
CREATE TRIGGER IF NOT EXISTS trg_docs_update_log
    AFTER UPDATE
    ON docs
    FOR EACH ROW
BEGIN
    INSERT INTO docs_changelog (doc_id, old_doc, new_doc, op, changed_at)
    VALUES (NEW.id, OLD.doc, NEW.doc, 'UPDATE', CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER));
END;

-- Trigger: Log DELETE operations
CREATE TRIGGER IF NOT EXISTS trg_docs_delete_log
    AFTER DELETE
    ON docs
    FOR EACH ROW
BEGIN
    INSERT INTO docs_changelog (doc_id, old_doc, new_doc, op, changed_at)
    VALUES (OLD.id, OLD.doc, NULL, 'DELETE', CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER));
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

-- View: Recent KV changes
CREATE VIEW IF NOT EXISTS v_kv_recent_changes AS
SELECT key,
       op,
       old_value,
       new_value,
       datetime(changed_at / 1000, 'unixepoch', 'localtime') as changed_at_readable,
       changed_at
FROM kv_changelog
ORDER BY changed_at DESC
LIMIT 100;

-- View: Recent docs changes
CREATE VIEW IF NOT EXISTS v_docs_recent_changes AS
SELECT doc_id,
       op,
       datetime(changed_at / 1000, 'unixepoch', 'localtime') as changed_at_readable,
       changed_at
FROM docs_changelog
ORDER BY changed_at DESC
LIMIT 100;

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