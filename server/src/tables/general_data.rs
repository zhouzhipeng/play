use serde::{Deserialize, Serialize};
use sqlx::{Error, FromRow};
use tracing::info;

use crate::tables::{DBPool, DBQueryResult};

#[derive(Clone, FromRow, Debug, Serialize, Deserialize, Default)]
pub struct GeneralData {
    pub id: u32,
    pub cat: String,
    pub data: String,
    pub created: chrono::NaiveDateTime,
    pub updated: chrono::NaiveDateTime,
}


impl GeneralData {
    pub fn new(cat: String, data: String)->Self{
        GeneralData{
            cat,
            data,
            ..Default::default()
        }
    }
    pub async fn insert(t: &GeneralData, pool: &DBPool) -> Result<DBQueryResult, Error> {
        sqlx::query("INSERT INTO general_data (cat,data) VALUES (?,?)")
            .bind(&t.cat)
            .bind(&t.data)
            .execute(pool)
            .await
    }

    pub async fn delete(id: u32, pool: &DBPool) -> Result<DBQueryResult, Error> {
        sqlx::query("DELETE from general_data WHERE id =?")
            .bind(&id)
            .execute(pool)
            .await
    }

    fn convert_fields(field: &str) -> String {
        let mut fields = "*".to_string();
        if field != "*" {
            fields = field.split(",").map(|f| f.trim())
                .map(|f| format!("'{}', json_extract(data, '$.{}')", f, f))
                .collect::<Vec<String>>() // Collect the strings into a new vector.
                .join(", ");
            info!("fields >> {}", fields);
            format!("id, cat, created,updated, json_object({}) as data", fields)
        }else{
            fields
        }


    }

    pub async fn query_by_cat(fields: &str, cat: &str, pool: &DBPool) -> Result<Vec<GeneralData>, Error> {
        let sql = &format!("SELECT {} FROM general_data where cat = ?", Self::convert_fields(fields));
        sqlx::query_as::<_, GeneralData>(sql)
            .bind(cat)
            .fetch_all(pool)
            .await
    }
    pub async fn query_by_cat_simple(cat: &str, pool: &DBPool) -> Result<Vec<GeneralData>, Error> {
        let sql = &format!("SELECT * FROM general_data where cat = ?");
        sqlx::query_as::<_, GeneralData>(sql)
            .bind(cat)
            .fetch_all(pool)
            .await
    }
    pub async fn query_latest_by_cat_with_limit(cat: &str, limit: u32, pool: &DBPool) -> Result<Vec<GeneralData>, Error> {
        let sql = &format!("SELECT * FROM general_data where cat = ? order by updated desc limit {}", limit);
        sqlx::query_as::<_, GeneralData>(sql)
            .bind(cat)
            .fetch_all(pool)
            .await
    }
    pub async fn query_by_json_field(fields: &str, cat: &str, query_field: &str, query_val: &str, pool: &DBPool) -> Result<Vec<GeneralData>, Error> {
        sqlx::query_as::<_, GeneralData>(format!("SELECT {} FROM general_data where cat = ? and json_extract(data, '$.{}') = ?", Self::convert_fields(fields), query_field).as_str())
            .bind(cat)
            .bind(query_val)
            .fetch_all(pool)
            .await
    }
    pub async fn query_by_id(data_id: u32, pool: &DBPool) -> Result<Vec<GeneralData>, Error> {
        sqlx::query_as::<_, GeneralData>("SELECT * FROM general_data where id = ? ")
            .bind(data_id)
            .fetch_all(pool)
            .await
    }
    pub async fn update_json_field_by_id(data_id: u32, query_field: &str, query_val: &str, pool: &DBPool) -> Result<DBQueryResult, Error> {
        sqlx::query(format!("update  general_data set data = json_set(data, '$.{}', ?), updated=CURRENT_TIMESTAMP where id = ?", query_field).as_str())
            .bind(query_val)
            .bind(data_id)
            .execute(pool)
            .await
    }
    pub async fn update_data_by_id(data_id: u32, data: &str, pool: &DBPool) -> Result<DBQueryResult, Error> {
        sqlx::query("update  general_data set data = ?, updated=CURRENT_TIMESTAMP where id = ?")
            .bind(data)
            .bind(data_id)
            .execute(pool)
            .await
    }
    pub async fn update_data_by_cat(cat: &str, data: &str, pool: &DBPool) -> Result<DBQueryResult, Error> {
        sqlx::query("update  general_data set data = ?, updated=CURRENT_TIMESTAMP where cat = ?")
            .bind(data)
            .bind(cat)
            .execute(pool)
            .await
    }
}


#[cfg(test)]
mod tests {
    use crate::tables::init_test_pool;
    use super::*;


    #[tokio::test]
    async fn test_convert_fiels() -> anyhow::Result<()> {

        let f = GeneralData::convert_fields("*");
        println!("{}", f);
        Ok(())
    }


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