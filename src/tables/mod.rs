use async_trait::async_trait;


use sqlx::{Error, migrate::MigrateDatabase, Pool, Sqlite, SqlitePool};
use sqlx::sqlite::SqliteQueryResult;
use tracing::info;

pub mod user;
mod article;

const DB_URL: &str = "sqlite://sqlite.db";

pub type DBPool = Pool<Sqlite>;
pub type DBQueryResult = SqliteQueryResult;

#[async_trait]
pub trait Table<IDType, QueryReturnType, QueryArgType, UpdateType, InsertType> {
    async fn insert(t: InsertType, pool: &DBPool) -> Result<DBQueryResult, Error>;
    async fn delete(id: IDType, pool: &DBPool) -> Result<DBQueryResult, Error>;
    async fn update(id: IDType, t: UpdateType, pool: &DBPool) -> Result<DBQueryResult, Error>;
    async fn query(q: QueryArgType, pool: &DBPool) -> Result<Vec<QueryReturnType>, Error>;
}

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

