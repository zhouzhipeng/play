use serde::{Deserialize, Serialize};
use sqlx::{Error, FromRow};
use shared::models::user::{AddUser, QueryUser, UpdateUser};


use crate::tables::{DBPool, DBQueryResult};

#[derive(Clone, FromRow, Debug, Serialize,Deserialize)]
pub struct User {
    pub id: i64,
    pub name: String,
}
impl User {
    pub async fn insert(t: AddUser, pool: &DBPool) -> Result<DBQueryResult, Error> {
        sqlx::query("INSERT INTO users (name) VALUES (?)")
            .bind(&t.name)
            .execute(pool)
            .await
    }

    pub async fn delete(id: i64, pool: &DBPool) -> Result<DBQueryResult, Error> {
        sqlx::query("DELETE from users WHERE id =?")
            .bind(id)
            .execute(pool)
            .await
    }

    pub async fn update(id: i64, t: UpdateUser, pool: &DBPool) -> Result<DBQueryResult, Error> {
        sqlx::query("UPDATE users set name=? WHERE id =?")
            .bind(t.name)
            .bind(id)
            .execute(pool)
            .await
    }

    pub async fn query(q: QueryUser, pool: &DBPool) -> Result<Vec<User>, Error> {
        sqlx::query_as::<_, User>("SELECT id, name FROM users where name = ?")
            .bind(q.name)
            .fetch_all(pool)
            .await
    }
}

