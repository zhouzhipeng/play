
use serde::{Deserialize, Serialize};
use sqlx::{Error, FromRow};

use crate::tables::{DBPool, DBQueryResult};

#[derive(Clone, FromRow, Debug, Serialize, Deserialize, Default)]
pub struct GeneralDataMeta {
    pub id: u32,
    pub name: String,
    pub desc: String,
    pub created: chrono::NaiveDateTime,
    pub updated: chrono::NaiveDateTime,
}


impl GeneralDataMeta {
pub async fn insert(t: &GeneralDataMeta, pool: &DBPool) -> Result<DBQueryResult, Error> {
    //todo: this is just a template code, write your own business.
    sqlx::query("INSERT INTO general_data_meta (name,desc,created,updated) VALUES (?,?,?,?)")
    .bind(&t.name)
    .bind(&t.desc)
    .bind(&t.created)
    .bind(&t.updated)
    .execute(pool)
    .await
}

pub async fn delete(id: i64, pool: &DBPool) -> Result<DBQueryResult, Error> {
    //todo: this is just a template code, write your own business.
    sqlx::query("DELETE from general_data_meta WHERE id =?")
    .bind(&id)
    .execute(pool)
    .await
}

pub async fn update(id: i64, t: &GeneralDataMeta, pool: &DBPool) -> Result<DBQueryResult, Error> {
    //todo: this is just a template code, write your own business.
    sqlx::query("UPDATE general_data_meta set name=?,desc=?,created=?,updated=? WHERE id =?")
    .bind(&t.name)
    .bind(&t.desc)
    .bind(&t.created)
    .bind(&t.updated)
    .bind(&id)
    .execute(pool)
    .await
}

pub async fn query(q: &GeneralDataMeta, pool: &DBPool) -> Result<Vec<GeneralDataMeta>, Error> {
    //todo: this is just a template code, write your own business.
    sqlx::query_as::<_, GeneralDataMeta>("SELECT * FROM general_data_meta where name = ?")
    .bind(&q.name)
    .fetch_all(pool)
    .await
}
pub async fn query_all(pool: &DBPool) -> Result<Vec<GeneralDataMeta>, Error> {
    //todo: this is just a template code, write your own business.
    sqlx::query_as::<_, GeneralDataMeta>("SELECT * FROM general_data_meta")
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
        let r = GeneralDataMeta::insert(&GeneralDataMeta {
             ..Default::default()
        }, &pool).await?;

        assert_eq!(r.rows_affected(), 1);

        let rows = GeneralDataMeta::query(&GeneralDataMeta {
            ..Default::default()
        }, &pool).await?;
        assert_eq!(rows.len(), 1);

        let r = GeneralDataMeta::update(1, &GeneralDataMeta {
            ..Default::default()
        }, &pool).await?;
        assert_eq!(r.rows_affected(), 1);

        let rows = GeneralDataMeta::query(&GeneralDataMeta {
            ..Default::default()
        }, &pool).await?;
        assert_eq!(rows[0].id, 1);

        let  r = GeneralDataMeta::delete(1, &pool).await?;
        assert_eq!(r.rows_affected(),1);

        let rows = GeneralDataMeta::query(&GeneralDataMeta {
            ..Default::default()
        }, &pool).await?;
        assert_eq!(rows.len(), 0);

        */


        Ok(())
    }
}