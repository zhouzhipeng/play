use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::{Error, FromRow};

use crate::tables::{DBPool, DBQueryResult, Table};

#[derive(Clone, FromRow, Debug, Serialize)]
pub struct Article {

}

#[derive(Deserialize)]
pub struct AddArticle {

}

#[derive(Deserialize)]
pub struct UpdateArticle {

}

#[derive(Deserialize)]
pub struct QueryArticle {

}

#[async_trait]
impl Table<i64, Article, QueryArticle, UpdateArticle, AddArticle> for Article {
    async fn insert(t: AddArticle, pool: &DBPool) -> Result<DBQueryResult, Error> {
        /*sqlx::query("INSERT INTO article (name) VALUES (?)")
            .bind(&t.name)
            .execute(pool)
            .await */
        todo!()
    }

    async fn delete(id: i64, pool: &DBPool) -> Result<DBQueryResult, Error> {
        /*sqlx::query("DELETE from article WHERE id =?")
            .bind(id)
            .execute(pool)
            .await*/
        todo!()
    }

    async fn update(id: i64, t: UpdateArticle, pool: &DBPool) -> Result<DBQueryResult, Error> {
        /*sqlx::query("UPDATE article set name=? WHERE id =?")
            .bind(t.name)
            .bind(id)
            .execute(pool)
            .await*/

        todo!()

    }

    async fn query(q: QueryArticle, pool: &DBPool) -> Result<Vec<Article>, Error> {
        /*sqlx::query_as::<_, Article>("SELECT id, name FROM article where name = ?")
            .bind(q.name)
            .fetch_all(pool)
            .await*/
        todo!()
    }
}

