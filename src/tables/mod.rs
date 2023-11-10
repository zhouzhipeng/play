use axum::handler::HandlerWithoutStateExt;
use axum::response::IntoResponse;
use sqlx::{FromRow, migrate::MigrateDatabase, Pool, Row, Sqlite, SqlitePool};
use sqlx::sqlite::SqliteQueryResult;

pub mod user;

const DB_URL: &str = "sqlite://sqlite.db";

pub type DBPool = Pool<Sqlite>;
pub type DBQueryResult = SqliteQueryResult;

pub async fn init_pool() -> DBPool {
    if !Sqlite::database_exists(DB_URL).await.unwrap_or(false) {
        println!("Creating database {}", DB_URL);
        match Sqlite::create_database(DB_URL).await {
            Ok(_) => println!("Create db success"),
            Err(error) => panic!("error: {}", error),
        }
    } else {
        println!("Database already exists");
    }

    let db = SqlitePool::connect(DB_URL).await.unwrap();
    let result = sqlx::query("CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY NOT NULL, name VARCHAR(250) NOT NULL);").execute(&db).await.unwrap();
    println!("Create user table result: {:?}", result);
    db
}

