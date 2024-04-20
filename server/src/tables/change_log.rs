
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

#[derive(Clone,sqlx::Type,Debug, Serialize, Deserialize, Default )]
pub enum ChangeLogOp {
    #[default]
    INSERT,
    UPDATE,
    DELETE,
}


impl ChangeLog {
    pub async fn insert(t: &ChangeLog, pool: &DBPool) -> Result<DBQueryResult, Error> {
        //todo: auto delete old logs.
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


    pub async fn query(data_id: u32, pool: &DBPool) -> Result<Vec<ChangeLog>, Error> {
        sqlx::query_as::<_, ChangeLog>("SELECT * FROM change_log where data_id = ?")
            .bind(data_id)
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
        let r = ChangeLog::insert(&ChangeLog {
            data_id: 1,
            op: ChangeLogOp::UPDATE,
            data_before:"111".to_string(),
            data_after:"222".to_string(),
            ..Default::default()
        }, &pool).await?;

        assert_eq!(r.rows_affected(), 1);

        let rows = ChangeLog::query(1, &pool).await?;
        assert_eq!(rows.len(), 1);
        println!("{:?}", rows);


        let  r = ChangeLog::delete(1, &pool).await?;
        assert_eq!(r.rows_affected(),1);


        Ok(())
    }
}