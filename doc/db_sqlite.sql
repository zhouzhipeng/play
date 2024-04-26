
CREATE TABLE IF NOT EXISTS api_entry
(
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    url        VARCHAR,
    method     VARCHAR,
    url_params VARCHAR,
    headers    VARCHAR,
    body       VARCHAR,
    updated    DATETIME DEFAULT CURRENT_TIMESTAMP
);



CREATE TABLE IF NOT EXISTS general_data
(
    id      INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    cat     VARCHAR,
    data    text,
    created DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS change_log
(
    id      INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    data_id INTEGER,
    op      VARCHAR,  -- INSERT, UPDATE, DELETE
    data_before    text,
    data_after    text,
    created DATETIME DEFAULT CURRENT_TIMESTAMP
);
