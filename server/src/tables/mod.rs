#[cfg(not(feature = "use_mysql"))]
use sqlx::{migrate::MigrateDatabase, Pool, Sqlite, SqlitePool};
#[cfg(feature = "use_mysql")]
use sqlx::{migrate::MigrateDatabase, MySql, Pool};
#[cfg(feature = "use_mysql")]
use sqlx::mysql::{MySqlPoolOptions, MySqlQueryResult};
#[cfg(not(feature = "use_mysql"))]
use sqlx::sqlite::SqliteQueryResult;
use tracing::{info, warn};

use play_shared::file_path;

use crate::config::Config;


pub mod general_data;

pub mod change_log;
//PLACEHOLDER:TABLE_MOD




#[cfg(not(feature = "use_mysql"))]
pub type DBPool = Pool<Sqlite>;
#[cfg(not(feature = "use_mysql"))]
pub type DBQueryResult = SqliteQueryResult;

#[cfg(feature = "use_mysql")]
pub type DBPool = Pool<MySql>;
#[cfg(feature = "use_mysql")]
pub type DBQueryResult = MySqlQueryResult;

#[cfg(not(feature = "use_mysql"))]
#[macro_export]
macro_rules! get_last_insert_id {
    ($t: expr) => {
        $t.last_insert_rowid()
    }
}
#[cfg(feature = "use_mysql")]
#[macro_export]
macro_rules! get_last_insert_id {
    ($t: expr) => {
        {
            $t.last_insert_id() as i64
        }
    }
}



#[cfg(not(feature = "use_mysql"))]
pub async fn init_pool(config: &Config) -> DBPool {
    let db_url: &str = config.database.url.as_str();

    if !Sqlite::database_exists(db_url).await.unwrap_or(false) {
        info!("Creating database {}", db_url);
        match Sqlite::create_database(db_url).await {
            Ok(_) => info!("Create db success"),
            Err(error) => panic!("error: {}", error),
        }
    } else {
        info!("Database already exists");
    }

    let db = SqlitePool::connect(db_url).await.unwrap();
    let result = sqlx::query(include_str!(file_path!("/../docs/db_sqlite.sql"))).execute(&db).await.unwrap();
    info!("Create  table result: {:?}", result);
    db
}


#[cfg(not(feature = "use_mysql"))]
pub async fn init_test_pool() -> DBPool {
    let db_test_url = ":memory:";
    let db = SqlitePool::connect(db_test_url).await.unwrap();
    let result = sqlx::query(include_str!(file_path!("/../docs/db_sqlite.sql"))).execute(&db).await.unwrap();
    // info!("Create  table result: {:?}", result);
    db
}


#[cfg(feature = "use_mysql")]
pub async fn init_pool(config: &Config) -> DBPool {
    let db_url: &str = config.database.url.as_str();

    if !MySql::database_exists(db_url).await.unwrap_or(false) {
        info!("Creating database {}", db_url);
        match MySql::create_database(db_url).await {
            Ok(_) => info!("Create db success"),
            Err(error) => panic!("error: {}", error),
        }
    }

    let db = MySqlPoolOptions::new()
        .max_connections(5)
        .connect(db_url).await.unwrap();

    for s in include_str!(file_path!("/../../docs/db_mysql.sql")).split(";") {
        if s.trim().is_empty() {
            continue;
        }
        let result = sqlx::query(s).execute(&db).await.unwrap();
        // info!("Create  table result: {:?}", result);
    };


    db
}

#[cfg(feature = "use_mysql")]
pub async fn init_test_pool() -> DBPool {
    const DB_URL: &str = "mysql://localhost:3306/test";

    if MySql::database_exists(DB_URL).await.unwrap_or(false) {
        //delete database
        MySql::drop_database(DB_URL).await.unwrap()
    }
    info!("Creating database {}", DB_URL);
    match MySql::create_database(DB_URL).await {
        Ok(_) => info!("Create db success"),
        Err(error) => warn!("error: {}", error),
    }


    let db = MySqlPoolOptions::new()
        .max_connections(5)
        .connect(DB_URL).await.unwrap();

    for s in include_str!(file_path!("/../../docs/db_mysql.sql")).split(";") {
        if s.trim().is_empty() {
            continue;
        }
        let result = sqlx::query(s).execute(&db).await.unwrap();
        info!("Create  table result: {:?}", result);
    };


    db
}

