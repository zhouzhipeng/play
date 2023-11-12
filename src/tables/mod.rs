use sqlx::{migrate::MigrateDatabase, Pool, Sqlite, SqlitePool};
use sqlx::sqlite::SqliteQueryResult;
use tracing::info;

pub mod user;
mod article;

const DB_URL: &str = "sqlite://sqlite.db";
const DB_TEST_URL: &str = ":memory:";

pub type DBPool = Pool<Sqlite>;
pub type DBQueryResult = SqliteQueryResult;


pub async fn init_pool() -> DBPool {
    if !Sqlite::database_exists(DB_URL).await.unwrap_or(false) {
        info!("Creating database {}", DB_URL);
        match Sqlite::create_database(DB_URL).await {
            Ok(_) => println!("Create db success"),
            Err(error) => panic!("error: {}", error),
        }
    } else {
        info!("Database already exists");
    }

    let db = SqlitePool::connect(DB_URL).await.unwrap();
    let result = sqlx::query(include_str!("db.sql")).execute(&db).await.unwrap();
    info!("Create  table result: {:?}", result);
    db
}

pub async fn init_test_pool() -> DBPool {
    let db = SqlitePool::connect(DB_TEST_URL).await.unwrap();
    let result = sqlx::query(include_str!("db.sql")).execute(&db).await.unwrap();
    info!("Create  table result: {:?}", result);
    db
}


