CREATE TABLE IF NOT EXISTS users(
    id INTEGER  PRIMARY KEY,
    name VARCHAR(250) NOT NULL
);

create table IF NOT EXISTS article(
    id integer primary key,
    title varchar(255) not null,
    content text
);

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


-- test
-- INSERT OR IGNORE INTO api_entry ('id','url','method','url_params','headers','body','updated') VALUES ('70','http://127.0.0.1/api/send-ws-msg','GET','UserId=123&Data=你好啊','','','2023-08-09 15:33:58');
