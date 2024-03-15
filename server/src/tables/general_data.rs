
use serde::{Deserialize, Serialize};
use sqlx::{Error, FromRow};

use crate::tables::{DBPool, DBQueryResult};

#[derive(Clone, FromRow, Debug, Serialize, Deserialize, Default)]
pub struct GeneralData {
    pub id: u32,
    pub meta_id: u32,
    pub data: String,
    pub created: chrono::NaiveDateTime,
    pub updated: chrono::NaiveDateTime,
}


impl GeneralData {
pub async fn insert(t: &GeneralData, pool: &DBPool) -> Result<DBQueryResult, Error> {
    sqlx::query("INSERT INTO general_data (meta_id,data) VALUES (?,?)")
    .bind(&t.meta_id)
    .bind(&t.data)
    .execute(pool)
    .await
}

pub async fn delete(id: i64, pool: &DBPool) -> Result<DBQueryResult, Error> {
    //todo: this is just a template code, write your own business.
    sqlx::query("DELETE from general_data WHERE id =?")
    .bind(&id)
    .execute(pool)
    .await
}

pub async fn update(id: i64, t: &GeneralData, pool: &DBPool) -> Result<DBQueryResult, Error> {
    //todo: this is just a template code, write your own business.
    sqlx::query("UPDATE general_data set meta_id=?,data=?,created=?,updated=? WHERE id =?")
    .bind(&t.meta_id)
    .bind(&t.data)
    .bind(&t.created)
    .bind(&t.updated)
    .bind(&id)
    .execute(pool)
    .await
}

pub async fn query(q: &GeneralData, pool: &DBPool) -> Result<Vec<GeneralData>, Error> {
    sqlx::query_as::<_, GeneralData>("SELECT * FROM general_data where meta_id = ?")
    .bind(&q.meta_id)
    .fetch_all(pool)
    .await
}
pub async fn query_json(meta_id: u32, query_field: &str, query_val: &str,  pool: &DBPool) -> Result<Vec<GeneralData>, Error> {
    sqlx::query_as::<_, GeneralData>(format!("SELECT * FROM general_data where meta_id = ? and json_extract(data, '$.{}') = ?", query_field).as_str())
    .bind(meta_id)
    .bind(query_val)
    .fetch_all(pool)
    .await
}
pub async fn query_all(pool: &DBPool) -> Result<Vec<GeneralData>, Error> {
    //todo: this is just a template code, write your own business.
    sqlx::query_as::<_, GeneralData>("SELECT * FROM general_data")
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
        let r = GeneralData::insert(&GeneralData {
             ..Default::default()
        }, &pool).await?;

        assert_eq!(r.rows_affected(), 1);

        let rows = GeneralData::query(&GeneralData {
            ..Default::default()
        }, &pool).await?;
        assert_eq!(rows.len(), 1);

        let r = GeneralData::update(1, &GeneralData {
            ..Default::default()
        }, &pool).await?;
        assert_eq!(r.rows_affected(), 1);

        let rows = GeneralData::query(&GeneralData {
            ..Default::default()
        }, &pool).await?;
        assert_eq!(rows[0].id, 1);

        let  r = GeneralData::delete(1, &pool).await?;
        assert_eq!(r.rows_affected(),1);

        let rows = GeneralData::query(&GeneralData {
            ..Default::default()
        }, &pool).await?;
        assert_eq!(rows.len(), 0);

        */


        Ok(())
    }
}