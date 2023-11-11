use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::{Error, FromRow};

use crate::tables::{DBPool, DBQueryResult, Table};

#[derive(Clone, FromRow, Debug, Serialize)]
pub struct User {
    pub id: i64,
    pub name: String,
}

#[derive(Deserialize)]
pub struct AddUser {
    pub name: String,
}

#[derive(Deserialize)]
pub struct UpdateUser {
    pub name: String,
}

#[derive(Deserialize)]
pub struct QueryUser {
    pub name: String,
}

#[async_trait]
impl Table<i64, User, QueryUser, UpdateUser, AddUser> for User {
    async fn insert(t: AddUser, pool: &DBPool) -> Result<DBQueryResult, Error> {
        sqlx::query("INSERT INTO users (name) VALUES (?)")
            .bind(&t.name)
            .execute(pool)
            .await
    }

    async fn delete(id: i64, pool: &DBPool) -> Result<DBQueryResult, Error> {
        sqlx::query("DELETE from users WHERE id =?")
            .bind(id)
            .execute(pool)
            .await
    }

    async fn update(id: i64, t: UpdateUser, pool: &DBPool) -> Result<DBQueryResult, Error> {
        sqlx::query("UPDATE users set name=? WHERE id =?")
            .bind(t.name)
            .bind(id)
            .execute(pool)
            .await
    }

    async fn query(q: QueryUser, pool: &DBPool) -> Result<Vec<User>, Error> {
        sqlx::query_as::<_, User>("SELECT id, name FROM users where name = ?")
            .bind(q.name)
            .fetch_all(pool)
            .await
    }
}

