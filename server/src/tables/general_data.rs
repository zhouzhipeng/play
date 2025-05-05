use crate::promise;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize, Serializer};
use serde_json::{Map, Value};
use sqlx::{Error, FromRow};
use std::collections::HashMap;
use anyhow::Context;
use tracing::info;

use crate::tables::{DBPool, DBQueryResult};

#[derive(Clone, FromRow, Debug, Serialize, Deserialize, Default)]
pub struct GeneralData {
    pub id: u32,
    pub cat: String,
    pub data: String,
    #[sqlx(default)]
    pub is_deleted: bool,
    #[serde(serialize_with = "serialize_as_timestamp")]
    pub created: NaiveDateTime,
    #[serde(serialize_with = "serialize_as_timestamp")]
    pub updated: NaiveDateTime,
}

// Custom serialization function for NaiveDateTime
fn serialize_as_timestamp<S>(date: &NaiveDateTime, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_i64(date.timestamp_millis())
}

impl GeneralData {
    pub fn to_flat_map(&self)->anyhow::Result<Map<String, Value>>{
        let data_val:Value = serde_json::from_str(&self.data)?;
        let mut json_val = serde_json::to_value(&self)?;
        let mut json_val = json_val.as_object_mut().context("Invalid JSON object")?;

        json_val.remove("data");
        for (k,v) in data_val.as_object().context("Invalid JSON object")? {
            json_val.insert(k.to_string(), v.clone());
        }

        Ok(json_val.clone())
    }
    pub fn extract_data(&self)->anyhow::Result<Map<String, Value>>{
        let data_val:Value = serde_json::from_str(&self.data)?;
        Ok(data_val.as_object().context("Invalid JSON object")?.clone())
    }
    pub fn new(cat: String, data: String) -> Self {
        GeneralData {
            cat,
            data,
            ..Default::default()
        }
    }
    pub async fn insert(cat: &str, data: &str, pool: &DBPool) -> Result<DBQueryResult, Error> {
        let r = sqlx::query("INSERT INTO general_data (cat,data) VALUES (?,?)")
            .bind(cat)
            .bind(data)
            .execute(pool)
            .await;

        r
    }

    pub async fn delete(id: u32, pool: &DBPool) -> Result<DBQueryResult, Error> {
        let r = sqlx::query("DELETE from general_data WHERE id =?")
            .bind(&id)
            .execute(pool)
            .await;

        r
    }
    pub async fn soft_delete(id: u32, pool: &DBPool) -> Result<DBQueryResult, Error> {
        let r = sqlx::query(
            "update  general_data set is_deleted=1, updated=CURRENT_TIMESTAMP WHERE id =?",
        )
        .bind(&id)
        .execute(pool)
        .await;

        r
    }
    pub async fn delete_by_cat(cat: &str, pool: &DBPool) -> Result<DBQueryResult, Error> {
        let r = sqlx::query("DELETE from general_data WHERE cat =?")
            .bind(cat)
            .execute(pool)
            .await;
        r
    }

    fn convert_fields(field: &str) -> String {
        let mut fields = "*".to_string();
        if field != "*" {
            fields = field
                .split(",")
                .map(|f| f.trim())
                .map(|f| format!("'{}', json_extract(data, '$.{}')", f, f))
                .collect::<Vec<String>>() // Collect the strings into a new vector.
                .join(", ");
            info!("fields >> {}", fields);
            format!(
                "id, cat, is_deleted, created,updated, json_object({}) as data",
                fields
            )
        } else {
            fields
        }
    }

    pub async fn query_by_cat(
        fields: &str,
        cat: &str,
        limit: i32,
        pool: &DBPool,
    ) -> Result<Vec<GeneralData>, Error> {
        let sql = &format!(
            "SELECT {} FROM general_data where cat = ? order by id desc limit {}",
            Self::convert_fields(fields),
            limit
        );
        sqlx::query_as::<_, GeneralData>(sql)
            .bind(cat)
            .fetch_all(pool)
            .await
    }
    pub async fn query_composite(
        fields: &str,
        cat: &str,
        limit: u32,
        _where: &str,
        order_by: &str,
        pool: &DBPool,
    ) -> Result<Vec<GeneralData>, Error> {
        let sql = &format!(
            "SELECT {} FROM general_data where cat = ? and {} order by {} limit {}",
            Self::convert_fields(fields),
            _where,
            order_by,
            limit
        );
        sqlx::query_as::<_, GeneralData>(sql)
            .bind(cat)
            .fetch_all(pool)
            .await
    }
    pub async fn query_count_composite(
        cat: &str,
        limit: u32,
        _where: &str,
        order_by: &str,
        pool: &DBPool,
    ) -> Result<u32, Error> {
        let sql = &format!(
            "SELECT count(1) FROM general_data where cat = ? and {}  order by {} limit {}",
            _where,
            order_by,
            limit
        );
        let result: (u32,) = sqlx::query_as(sql).bind(cat).fetch_one(pool).await?;
        Ok(result.0)
    }
    pub async fn query_count(cat: &str, pool: &DBPool) -> Result<i64, Error> {
        let sql = "SELECT count(*) FROM general_data where cat = ?";
        let result: (i64,) = sqlx::query_as(sql).bind(cat).fetch_one(pool).await?;
        Ok(result.0)
    }
    pub async fn query_by_cat_simple(
        cat: &str,
        limit: i32,
        pool: &DBPool,
    ) -> Result<Vec<GeneralData>, Error> {
        let sql = &format!("SELECT * FROM general_data where cat = ? limit {}", limit);
        sqlx::query_as::<_, GeneralData>(sql)
            .bind(cat)
            .fetch_all(pool)
            .await
    }
    pub async fn query_latest_by_cat_with_limit(
        cat: &str,
        limit: u32,
        pool: &DBPool,
    ) -> Result<Vec<GeneralData>, Error> {
        let sql = &format!(
            "SELECT * FROM general_data where cat = ? order by updated desc limit {}",
            limit
        );
        sqlx::query_as::<_, GeneralData>(sql)
            .bind(cat)
            .fetch_all(pool)
            .await
    }
    pub async fn query_by_json_field(
        fields: &str,
        cat: &str,
        query_field: &str,
        query_val: &str,
        limit: i32,
        pool: &DBPool,
    ) -> Result<Vec<GeneralData>, Error> {
        sqlx::query_as::<_, GeneralData>(format!("SELECT {} FROM general_data where cat = ? and json_extract(data, '$.{}') = ?  order by id desc limit {}", Self::convert_fields(fields), query_field, limit).as_str())
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
    pub async fn query_by_id_with_select(
        fields: &str,
        data_id: u32,
        pool: &DBPool,
    ) -> Result<Vec<GeneralData>, Error> {
        sqlx::query_as::<_, GeneralData>(&format!(
            "SELECT {} FROM general_data where id = ? ",
            Self::convert_fields(fields)
        ))
        .bind(data_id)
        .fetch_all(pool)
        .await
    }
    pub async fn query_by_id_with_cat_select(
        fields: &str,
        data_id: u32,
        cat: &str,
        pool: &DBPool,
    ) -> Result<Vec<GeneralData>, Error> {
        sqlx::query_as::<_, GeneralData>(&format!(
            "SELECT {} FROM general_data where id = ?  and cat = ?",
            Self::convert_fields(fields)
        ))
        .bind(data_id)
        .bind(cat)
        .fetch_all(pool)
        .await
    }
    pub async fn update_json_field_by_id(
        data_id: u32,
        query_field: &str,
        query_val: &str,
        pool: &DBPool,
    ) -> Result<DBQueryResult, Error> {

        let r = sqlx::query(format!("update  general_data set data = json_set(data, '$.{}', ?), updated=CURRENT_TIMESTAMP where id = ?", query_field).as_str())
            .bind(query_val)
            .bind(data_id)
            .execute(pool)
            .await;

        r
    }

    pub async fn update_with_json_patch(
        pool: &DBPool,
        id: u32,
        updates: Value,
    ) -> Result<DBQueryResult, Error> {
        // 开始构建查询
        let mut query = sqlx::query("UPDATE general_data SET data = json_patch(data, ?), updated=CURRENT_TIMESTAMP WHERE id = ?")
            .bind(updates.to_string()).bind(id);
        // 执行查询
        let r = query.execute(pool).await?;

        Ok(r)
    }
    pub async fn update_data_by_id(
        data_id: u32,
        data: &str,
        pool: &DBPool,
    ) -> Result<DBQueryResult, Error> {

        sqlx::query("update  general_data set data = ?, updated=CURRENT_TIMESTAMP where id = ?")
            .bind(data)
            .bind(data_id)
            .execute(pool)
            .await
    }
    pub async fn update_data_by_cat(
        cat: &str,
        data: &str,
        pool: &DBPool,
    ) -> Result<DBQueryResult, Error> {
        sqlx::query("update  general_data set data = ?, updated=CURRENT_TIMESTAMP where cat = ?")
            .bind(data)
            .bind(cat)
            .execute(pool)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock_state;
    use crate::tables::init_test_pool;

    #[ignore]
    #[tokio::test]
    async fn test_convert_fiels() -> anyhow::Result<()> {
        let f = GeneralData::convert_fields("*");
        println!("{}", f);
        Ok(())
    }
    #[ignore]
    #[tokio::test]
    async fn test_query_count() -> anyhow::Result<()> {
        let s = mock_state!();
        GeneralData::insert("test", "dd", &s.db).await?;
        let f = GeneralData::query_count("test", &s.db).await;
        println!("{:?}", f);
        Ok(())
    }
    #[ignore]
    #[tokio::test]
    async fn test_query_with_limit() -> anyhow::Result<()> {
        let s = mock_state!();
        GeneralData::insert("test", "dd", &s.db).await?;
        GeneralData::insert("test", "dd2", &s.db).await?;
        let f = GeneralData::query_by_cat_simple("test", 2, &s.db).await;
        println!("{:?}", f);
        Ok(())
    }
    #[ignore]
    #[tokio::test]
    async fn test_delete_by_cat() -> anyhow::Result<()> {
        let s = mock_state!();
        GeneralData::insert("test", "dd", &s.db).await?;
        // GeneralData::insert("test","dd2", &s.db).await?;
        let f = GeneralData::delete_by_cat("test", &s.db).await;
        println!("{:?}", f);
        Ok(())
    }

   
}
