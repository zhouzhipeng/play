use serde::{Deserialize, Serialize};
use sqlx::{Error, FromRow};

use crate::tables::{DBPool, DBQueryResult};

#[derive(Clone, FromRow, Debug, Serialize, Deserialize, Default)]
pub struct ChangeLog {
    pub id: i64,
    pub data_id: u32,
    pub op: ChangeLogOp,
    pub data_before: String,
    pub data_after: String,
    pub created: chrono::NaiveDateTime,
}

#[derive(Clone, sqlx::Type, Debug, Serialize, Deserialize, Default)]
pub enum ChangeLogOp {
    #[default]
    INSERT,
    UPDATE,
    DELETE,
}

impl ChangeLog {
    pub async fn insert(t: &ChangeLog, pool: &DBPool) -> Result<DBQueryResult, Error> {
        let count = Self::query_count(pool).await?.0;
        if count > 200 {
            let old_id = Self::query_oldest_one(pool).await?.0;
            Self::delete(old_id, pool).await?;
        }

        sqlx::query("INSERT INTO change_log (data_id,op,data_before,data_after) VALUES (?,?,?,?)")
            .bind(&t.data_id)
            .bind(&t.op)
            .bind(&t.data_before)
            .bind(&t.data_after)
            .execute(pool)
            .await
    }

    pub async fn delete(id: i64, pool: &DBPool) -> Result<DBQueryResult, Error> {
        sqlx::query("DELETE from change_log WHERE id =?")
            .bind(&id)
            .execute(pool)
            .await
    }
    pub async fn delete_days_ago(
        timestamp_str: &str,
        pool: &DBPool,
    ) -> Result<DBQueryResult, Error> {
        sqlx::query("DELETE from change_log WHERE created < ?")
            .bind(timestamp_str)
            .execute(pool)
            .await
    }

    pub async fn query(data_id: u32, pool: &DBPool) -> Result<Vec<ChangeLog>, Error> {
        sqlx::query_as::<_, ChangeLog>(
            "SELECT * FROM change_log where data_id = ? order by id desc limit 10",
        )
        .bind(data_id)
        .fetch_all(pool)
        .await
    }
    pub async fn query_count(pool: &DBPool) -> Result<(i64,), Error> {
        sqlx::query_as::<_, (i64,)>("SELECT count(1) FROM change_log")
            .fetch_one(pool)
            .await
    }
    pub async fn query_oldest_one(pool: &DBPool) -> Result<(i64,), Error> {
        sqlx::query_as::<_, (i64,)>("SELECT id FROM change_log order by id asc limit 1")
            .fetch_one(pool)
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

        //insert data
        let r = ChangeLog::insert(
            &ChangeLog {
                data_id: 1,
                op: ChangeLogOp::UPDATE,
                data_before: "111".to_string(),
                data_after: "222".to_string(),
                ..Default::default()
            },
            &pool,
        )
        .await?;

        assert_eq!(r.rows_affected(), 1);

        //query count
        let count = ChangeLog::query_count(&pool).await?;
        println!("{:?}", count.0);

        let r = ChangeLog::insert(
            &ChangeLog {
                data_id: 2,
                op: ChangeLogOp::UPDATE,
                data_before: "333".to_string(),
                data_after: "444".to_string(),
                ..Default::default()
            },
            &pool,
        )
        .await?;

        assert_eq!(r.rows_affected(), 1);
        //query count
        let count = ChangeLog::query_count(&pool).await?;
        println!("{:?}", count.0);

        let r = ChangeLog::insert(
            &ChangeLog {
                data_id: 3,
                op: ChangeLogOp::UPDATE,
                data_before: "555".to_string(),
                data_after: "666".to_string(),
                ..Default::default()
            },
            &pool,
        )
        .await?;

        assert_eq!(r.rows_affected(), 1);

        //query count
        let count = ChangeLog::query_count(&pool).await?;
        println!("{:?}", count.0);

        //delete old logs
        let r = ChangeLog::query_oldest_one(&pool).await?;
        println!("{:?}", r);

        let rows = ChangeLog::query(3, &pool).await?;
        // assert_eq!(rows.len(), 1);
        println!("{:?}", rows);

        let r = ChangeLog::delete(3, &pool).await?;
        assert_eq!(r.rows_affected(), 1);

        Ok(())
    }
}
