
CREATE TABLE IF NOT EXISTS general_data
(
    id      INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    cat     VARCHAR,
    data    text,
    is_deleted  INTEGER DEFAULT 0,
    created DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_general_data_cat ON general_data(cat);


CREATE TABLE IF NOT EXISTS change_log
(
    id      INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    data_id INTEGER,
    op      VARCHAR,  -- INSERT, UPDATE, DELETE
    data_before    text,
    data_after    text,
    created DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 创建触发器:自动保存修改记录
CREATE TRIGGER IF NOT EXISTS after_general_data_update
AFTER UPDATE ON general_data
WHEN NEW.data != OLD.data
BEGIN
-- 插入新的记录
INSERT INTO change_log (data_id, op, data_before, data_after)
VALUES (OLD.id, 'UPDATE', OLD.data, NEW.data);
END;