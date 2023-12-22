
create table IF NOT EXISTS todo_item(
  id integer primary key AUTOINCREMENT,
  title varchar(255) not null,
  status varchar(10) not null
);


-- test data
-- delete from todo_item;
-- INSERT OR IGNORE INTO todo_item (title, status) VALUES ('todo 1', 'TODO');
-- INSERT OR IGNORE INTO todo_item (title, status) VALUES ('todo 2', 'TODO');
-- INSERT OR IGNORE INTO todo_item (title, status) VALUES ('todo 3', 'TODO');
-- INSERT OR IGNORE INTO todo_item (title, status) VALUES ('todo 4', 'DONE');


CREATE TABLE IF NOT EXISTS api_entry (
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 url VARCHAR,
 method VARCHAR,
 url_params VARCHAR,
 headers VARCHAR,
 body VARCHAR,
 updated DATETIME DEFAULT CURRENT_TIMESTAMP
);


CREATE TABLE IF NOT EXISTS english_card (
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 name VARCHAR,
 phonetic VARCHAR,
 meaning VARCHAR,
 updated DATETIME DEFAULT CURRENT_TIMESTAMP
);


