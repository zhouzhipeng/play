use serde::{Deserialize, Serialize};
use sqlx::{Error, FromRow};

use crate::tables::{DBPool, DBQueryResult};

#[derive(Clone, FromRow, Debug, Serialize, Deserialize)]
pub struct ApiEntry {
    pub id: i64,
    pub url: String,
    pub method: String,
    pub url_params: String,
    pub headers: String,
    pub body: String,
    pub updated: String,
}


#[derive(Serialize,Deserialize)]
pub struct UpdateApiEntry {
    pub id: Option<i64>,
    pub url: String,
    pub method: String,
    pub url_params: String,
    pub headers: String,
    pub body: String,
}

#[derive(Serialize,Deserialize)]
pub struct QueryApiEntry {
    pub url: String,

}

#[derive(Serialize,Deserialize)]
pub struct ApiEntryVo {
    pub id: i64,
    pub url: String,
    pub method: String,
    pub url_params: String,
    pub headers: String,
    pub body: String,
    pub updated: String,
}


impl ApiEntry {
    pub async fn insert(t: UpdateApiEntry, pool: &DBPool) -> Result<DBQueryResult, Error> {
        //todo: this is just a template code, write your own business.
        sqlx::query("INSERT INTO api_entry (url,method,url_params,headers,body) VALUES (?,?,?,?,?)")
            .bind(&t.url)
            .bind(&t.method)
            .bind(&t.url_params)
            .bind(&t.headers)
            .bind(&t.body)
            .execute(pool)
            .await
    }

    pub async fn delete(id: i64, pool: &DBPool) -> Result<DBQueryResult, Error> {
        //todo: this is just a template code, write your own business.
        sqlx::query("DELETE from api_entry WHERE id =?")
            .bind(&id)
            .execute(pool)
            .await
    }

    pub async fn update(id: i64, t: UpdateApiEntry, pool: &DBPool) -> Result<DBQueryResult, Error> {
        //todo: this is just a template code, write your own business.
        sqlx::query("UPDATE api_entry set url=?,method=?,url_params=?,headers=?,body=? WHERE id =?")
            .bind(&t.url)
            .bind(&t.method)
            .bind(&t.url_params)
            .bind(&t.headers)
            .bind(&t.body)
            .bind(&id)
            .execute(pool)
            .await
    }

    pub async fn query(q: QueryApiEntry, pool: &DBPool) -> Result<Vec<ApiEntry>, Error> {
        //todo: this is just a template code, write your own business.
        sqlx::query_as::<_, ApiEntry>("SELECT * FROM api_entry where url = ?")
            .bind(&q.url)
            .fetch_all(pool)
            .await
    }
    pub async fn query_by_id(id : i32, pool: &DBPool) -> Result<Vec<ApiEntry>, Error> {
        sqlx::query_as::<_, ApiEntry>("SELECT * FROM api_entry where id = ?")
            .bind(id)
            .fetch_all(pool)
            .await
    }
    pub async fn query_all(pool: &DBPool) -> Result<Vec<ApiEntry>, Error> {
        sqlx::query_as::<_, ApiEntry>("SELECT * FROM api_entry")
            .fetch_all(pool)
            .await
    }
}



#[cfg(test)]
mod tests {
    use crate::tables::init_test_pool;

    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[tokio::test]
    async fn test_all() -> anyhow::Result<()> {
        // let pool = init_test_pool().await;
        //
        //
        // let r = ApiEntry::insert(AddApiEntry {
        //     url: "/a/b".to_string(),
        //     method: "GET".to_string(),
        //     url_params: "a=1&b=2".to_string(),
        //     headers: "{}".to_string(),
        //     body: "".to_string(),
        //     updated: "".to_string(),
        // }, &pool).await?;
        //
        // assert_eq!(r.rows_affected(), 1);

        // let rows = ApiEntry::query(QueryApiEntry {
        //
        // }, &pool).await?;
        // assert_eq!(rows.len(), 1);
        // assert_eq!(rows[0].id, 1);
        //
        // let r = ApiEntry::update(1, UpdateApiEntry {
        //
        // }, &pool).await?;
        // assert_eq!(r.rows_affected(), 1);
        //
        // let rows = ApiEntry::query(QueryApiEntry {
        //
        // }, &pool).await?;
        // assert_eq!(rows[0].id, 1);
        //
        // let  r = ApiEntry::delete(1, &pool).await?;
        // assert_eq!(r.rows_affected(),1);
        //
        // let rows = ApiEntry::query(QueryApiEntry {
        //
        // }, &pool).await?;
        // assert_eq!(rows.len(), 0);



        Ok(())
    }
}