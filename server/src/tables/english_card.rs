
use serde::{Deserialize, Serialize};
use sqlx::{Error, FromRow};

use crate::tables::{DBPool, DBQueryResult};

#[derive(Clone, FromRow, Debug, Serialize, Deserialize, Default)]
pub struct EnglishCard {
    pub id: i64,
    pub name: String,
    pub phonetic: String,
    pub meaning: String,
    pub updated: String,
}


impl EnglishCard {
pub async fn insert(t: &EnglishCard, pool: &DBPool) -> Result<DBQueryResult, Error> {
    //todo: this is just a template code, write your own business.
    sqlx::query("INSERT INTO english_card (name,phonetic,meaning,updated) VALUES (?,?,?,?)")
    .bind(&t.name)
    .bind(&t.phonetic)
    .bind(&t.meaning)
    .bind(&t.updated)
    .execute(pool)
    .await
}

pub async fn delete(id: i64, pool: &DBPool) -> Result<DBQueryResult, Error> {
    //todo: this is just a template code, write your own business.
    sqlx::query("DELETE from english_card WHERE id =?")
    .bind(&id)
    .execute(pool)
    .await
}

pub async fn update(id: i64, t: &EnglishCard, pool: &DBPool) -> Result<DBQueryResult, Error> {
    //todo: this is just a template code, write your own business.
    sqlx::query("UPDATE english_card set name=?,phonetic=?,meaning=?,updated=? WHERE id =?")
    .bind(&t.name)
    .bind(&t.phonetic)
    .bind(&t.meaning)
    .bind(&t.updated)
    .bind(&id)
    .execute(pool)
    .await
}

pub async fn query(q: &EnglishCard, pool: &DBPool) -> Result<Vec<EnglishCard>, Error> {
    //todo: this is just a template code, write your own business.
    sqlx::query_as::<_, EnglishCard>("SELECT * FROM english_card where name = ?")
    .bind(&q.name)
    .fetch_all(pool)
    .await
}
pub async fn query_all(pool: &DBPool) -> Result<Vec<EnglishCard>, Error> {
    //todo: this is just a template code, write your own business.
    sqlx::query_as::<_, EnglishCard>("SELECT * FROM english_card")
    .fetch_all(pool)
    .await
    }
}



#[cfg(test)]
mod tests {
    use crate::tables::init_test_pool;

    use super::*;

    #[tokio::test]
    async fn test_all() -> anyhow::Result<()> {
        //the test pool is just a memory sqlite.
        let pool = init_test_pool().await;

        //todo: uncomment below code and write your tests.
        /*
        let r = EnglishCard::insert(&EnglishCard {
             ..Default::default()
        }, &pool).await?;

        assert_eq!(r.rows_affected(), 1);

        let rows = EnglishCard::query(&EnglishCard {
            ..Default::default()
        }, &pool).await?;
        assert_eq!(rows.len(), 1);

        let r = EnglishCard::update(1, &EnglishCard {
            ..Default::default()
        }, &pool).await?;
        assert_eq!(r.rows_affected(), 1);

        let rows = EnglishCard::query(&EnglishCard {
            ..Default::default()
        }, &pool).await?;
        assert_eq!(rows[0].id, 1);

        let  r = EnglishCard::delete(1, &pool).await?;
        assert_eq!(r.rows_affected(),1);

        let rows = EnglishCard::query(&EnglishCard {
            ..Default::default()
        }, &pool).await?;
        assert_eq!(rows.len(), 0);

        */


        Ok(())
    }
}