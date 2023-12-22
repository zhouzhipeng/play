use serde::{Deserialize, Serialize};
use sqlx::{Error, FromRow};
use shared::models::article::*;

use crate::tables::{DBPool, DBQueryResult};

#[derive(Clone, FromRow, Debug, Serialize, Deserialize)]
pub struct Article {
    pub id: i64,
    pub title: String,
    pub content: String,
}


impl Article {
    pub async fn insert(t: AddArticle, pool: &DBPool) -> Result<DBQueryResult, Error> {
        //todo: this is just a template code, write your own business.
        sqlx::query("INSERT INTO article (title,content) VALUES (?,?)")
            .bind(&t.title)
            .bind(&t.content)
            .execute(pool)
            .await
    }

    pub async fn delete(id: i64, pool: &DBPool) -> Result<DBQueryResult, Error> {
        //todo: this is just a template code, write your own business.
        sqlx::query("DELETE from article WHERE id =?")
            .bind(&id)
            .execute(pool)
            .await
    }

    pub async fn update(id: i64, t: UpdateArticle, pool: &DBPool) -> Result<DBQueryResult, Error> {
        //todo: this is just a template code, write your own business.
        sqlx::query("UPDATE article set title=?,content=? WHERE id =?")
            .bind(&t.title)
            .bind(&t.content)
            .bind(&id)
            .execute(pool)
            .await
    }

    pub async fn query(q: QueryArticle, pool: &DBPool) -> Result<Vec<Article>, Error> {
        //todo: this is just a template code, write your own business.
        sqlx::query_as::<_, Article>("SELECT * FROM article where title = ?")
            .bind(&q.title)
            .fetch_all(pool)
            .await
    }
    pub async fn query_by_id(id : u32, pool: &DBPool) -> Result<Vec<Article>, Error> {
        sqlx::query_as::<_, Article>("SELECT * FROM article where id = ?")
            .bind(id)
            .fetch_all(pool)
            .await
    }
    pub async fn query_all(pool: &DBPool) -> Result<Vec<Article>, Error> {
        sqlx::query_as::<_, Article>("SELECT * FROM article")
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

        let r = Article::insert(AddArticle {
            title: "test title".to_string(),
            content: "test content".to_string(),
        }, &pool).await?;

        assert_eq!(r.rows_affected(), 1);

        let rows = Article::query(QueryArticle {
            title: "test title".to_string(),
        }, &pool).await?;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, 1);

        let r = Article::update(1, UpdateArticle {
            title: "new title".to_string(),
            content: "new content".to_string(),
        }, &pool).await?;
        assert_eq!(r.rows_affected(), 1);

        let rows = Article::query(QueryArticle {
            title: "new title".to_string(),
        }, &pool).await?;
        assert_eq!(rows[0].id, 1);

        let  r = Article::delete(1, &pool).await?;
        assert_eq!(r.rows_affected(),1);

        let rows = Article::query(QueryArticle {
            title: "new title".to_string(),
        }, &pool).await?;
        assert_eq!(rows.len(), 0);




        Ok(())
    }
}
