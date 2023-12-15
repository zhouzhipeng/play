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
delete from todo_item;
INSERT OR IGNORE INTO todo_item (title, status) VALUES ('todo 1', 'TODO');
INSERT OR IGNORE INTO todo_item (title, status) VALUES ('todo 2', 'TODO');
INSERT OR IGNORE INTO todo_item (title, status) VALUES ('todo 3', 'TODO');
INSERT OR IGNORE INTO todo_item (title, status) VALUES ('todo 4', 'DONE');