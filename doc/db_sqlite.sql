create table IF NOT EXISTS todo_item
(
    id     integer primary key AUTOINCREMENT,
    title  varchar(255) not null,
    status varchar(10)  not null
);

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


CREATE TABLE IF NOT EXISTS english_card
(
    id       INTEGER PRIMARY KEY AUTOINCREMENT,
    name     VARCHAR,
    phonetic VARCHAR,
    meaning  VARCHAR,
    updated  DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS email_inbox
(
    id            INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    from_mail     VARCHAR,
    to_mail       VARCHAR,
    send_date     VARCHAR,
    subject       VARCHAR,
    plain_content VARCHAR,
    html_content  VARCHAR,
    full_body     VARCHAR,
    attachments   VARCHAR,
    create_time   INTEGER
);

CREATE TABLE IF NOT EXISTS general_data
(
    id      INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    cat     VARCHAR,
    data    text,
    created DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated DATETIME DEFAULT CURRENT_TIMESTAMP
);
