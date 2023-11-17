CREATE TABLE IF NOT EXISTS users(
    id INTEGER  PRIMARY KEY AUTO_INCREMENT,
    name VARCHAR(250) NOT NULL
);

create table IF NOT EXISTS article(
    id integer primary key AUTO_INCREMENT,
    title varchar(255) not null,
    content text
);