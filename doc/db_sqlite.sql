create table IF NOT EXISTS todo_item
(
    id     integer primary key AUTOINCREMENT,
    title  varchar(255) not null,
    status varchar(10)  not null
);


-- test data
-- delete from todo_item;
-- INSERT OR IGNORE INTO todo_item (title, status) VALUES ('todo 1', 'TODO');
-- INSERT OR IGNORE INTO todo_item (title, status) VALUES ('todo 2', 'TODO');
-- INSERT OR IGNORE INTO todo_item (title, status) VALUES ('todo 3', 'TODO');
-- INSERT OR IGNORE INTO todo_item (title, status) VALUES ('todo 4', 'DONE');


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

insert into email_inbox(from_mail, to_mail, send_date, subject, plain_content, html_content, full_body, attachments, create_time)
values ('aa@qq.com', 'bb@cc.com,111@cc.com', '10:11', 'test111', 'test html content', 'test html content', '', '',1703918268267);
insert into email_inbox(from_mail, to_mail, send_date, subject, plain_content, html_content, full_body, attachments, create_time)
values ('aa@qq.com', 'bb@cc.com,111@cc.com', '10:11', 'test111', 'test html content', 'test html content', '', '',1703918268267)
