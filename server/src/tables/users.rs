
use serde::{Deserialize, Serialize};
use sqlx::{Error, FromRow};

use crate::tables::{DBPool, DBQueryResult};

#[derive(Clone, FromRow, Debug, Serialize, Deserialize)]
pub struct Users {
    pub id: i64,
    pub name: String,
}

#[derive(Serialize,Deserialize)]
pub struct AddUsers {
    pub name: String,
}

#[derive(Serialize,Deserialize)]
pub struct UpdateUsers {
    pub name: String,
}

#[derive(Serialize,Deserialize)]
pub struct QueryUsers {
    pub name: String,

}

#[derive(Serialize,Deserialize)]
pub struct UsersVo {
    pub id: i64,
    pub name: String,
}


impl Users {
pub async fn insert(t: AddUsers, pool: &DBPool) -> Result<DBQueryResult, Error> {
    //todo: this is just a template code, write your own business.
    sqlx::query("INSERT INTO users (name) VALUES (?)")
    .bind(&t.name)
    .execute(pool)
    .await
}

pub async fn delete(id: i64, pool: &DBPool) -> Result<DBQueryResult, Error> {
    //todo: this is just a template code, write your own business.
    sqlx::query("DELETE from users WHERE id =?")
    .bind(&id)
    .execute(pool)
    .await
}

pub async fn update(id: i64, t: UpdateUsers, pool: &DBPool) -> Result<DBQueryResult, Error> {
    //todo: this is just a template code, write your own business.
    sqlx::query("UPDATE users set name=? WHERE id =?")
    .bind(&t.name)
    .bind(&id)
    .execute(pool)
    .await
}

pub async fn query(q: QueryUsers, pool: &DBPool) -> Result<Vec<Users>, Error> {
    //todo: this is just a template code, write your own business.
    sqlx::query_as::<_, Users>("SELECT * FROM users where name = ?")
    .bind(&q.name)
    .fetch_all(pool)
    .await
}
pub async fn query_all(pool: &DBPool) -> Result<Vec<Users>, Error> {
    //todo: this is just a template code, write your own business.
    sqlx::query_as::<_, Users>("SELECT * FROM users")
    .fetch_all(pool)
    .await
    }
}



#[cfg(test)]
mod tests {
    use crate::tables::init_test_pool;

    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[ignore]
    #[tokio::test]
    async fn test_all() -> anyhow::Result<()> {
        let pool = init_test_pool().await;

        todo!();

        //todo: uncomment below code and write your tests.
        /*
        let r = Users::insert(AddUsers {

        }, &pool).await?;

        assert_eq!(r.rows_affected(), 1);

        let rows = Users::query(QueryUsers {

        }, &pool).await?;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, 1);

        let r = Users::update(1, UpdateUsers {

        }, &pool).await?;
        assert_eq!(r.rows_affected(), 1);

        let rows = Users::query(QueryUsers {

        }, &pool).await?;
        assert_eq!(rows[0].id, 1);

        let  r = Users::delete(1, &pool).await?;
        assert_eq!(r.rows_affected(),1);

        let rows = Users::query(QueryUsers {

        }, &pool).await?;
        assert_eq!(rows.len(), 0);

        */


        Ok(())
    }
}