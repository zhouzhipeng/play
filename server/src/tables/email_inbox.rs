use serde::{Deserialize, Serialize};
use sqlx::{Error, FromRow};

use crate::tables::{DBPool, DBQueryResult};

#[derive(Clone, FromRow, Debug, Serialize, Deserialize, Default)]
pub struct EmailInbox {
    pub id: i64,
    pub from_mail: String,
    pub to_mail: String,
    pub send_date: String,
    pub subject: String,
    pub plain_content: String,
    pub html_content: String,
    pub full_body: String,
    pub attachments: String,
    pub create_time: i64,
}


impl EmailInbox {
    pub async fn insert(t: &EmailInbox, pool: &DBPool) -> Result<DBQueryResult, Error> {
        sqlx::query("INSERT INTO email_inbox (from_mail,to_mail,send_date,subject,plain_content,html_content,full_body,attachments,create_time) VALUES (?,?,?,?,?,?,?,?,?)")
            .bind(&t.from_mail)
            .bind(&t.to_mail)
            .bind(&t.send_date)
            .bind(&t.subject)
            .bind(&t.plain_content)
            .bind(&t.html_content)
            .bind(&t.full_body)
            .bind(&t.attachments)
            .bind(&t.create_time)
            .execute(pool)
            .await
    }

    pub async fn delete(id: i64, pool: &DBPool) -> Result<DBQueryResult, Error> {
        //todo: this is just a template code, write your own business.
        sqlx::query("DELETE from email_inbox WHERE id =?")
            .bind(&id)
            .execute(pool)
            .await
    }

    pub async fn delete_all(pool: &DBPool) -> Result<DBQueryResult, Error> {
        sqlx::query("DELETE from email_inbox")
            .execute(pool)
            .await
    }

    pub async fn update(id: i64, t: &EmailInbox, pool: &DBPool) -> Result<DBQueryResult, Error> {
        //todo: this is just a template code, write your own business.
        sqlx::query("UPDATE email_inbox set from_mail=?,to_mail=?,send_date=?,subject=?,plain_content=?,html_content=?,full_body=?,attachments=?,create_time=? WHERE id =?")
            .bind(&t.from_mail)
            .bind(&t.to_mail)
            .bind(&t.send_date)
            .bind(&t.subject)
            .bind(&t.plain_content)
            .bind(&t.html_content)
            .bind(&t.full_body)
            .bind(&t.attachments)
            .bind(&t.create_time)
            .bind(&id)
            .execute(pool)
            .await
    }

    pub async fn query(q: &EmailInbox, pool: &DBPool) -> Result<Vec<EmailInbox>, Error> {
        //todo: this is just a template code, write your own business.
        sqlx::query_as::<_, EmailInbox>("SELECT * FROM email_inbox where from_mail = ?")
            .bind(&q.from_mail)
            .fetch_all(pool)
            .await
    }
    pub async fn query_all(pool: &DBPool) -> Result<Vec<EmailInbox>, Error> {
        sqlx::query_as::<_, EmailInbox>("SELECT * FROM email_inbox order by id desc limit 20")
            .fetch_all(pool)
            .await
    }
    pub async fn count(pool: &DBPool) -> Result<i64, Error> {
        let result: (i64,) = sqlx::query_as("SELECT count(*) FROM email_inbox order by id desc limit 20")
            .fetch_one(pool)
            .await?;
        Ok(result.0)
    }
}


#[cfg(test)]
mod tests {
    use crate::tables::init_test_pool;

    use super::*;
    #[tokio::test]
    async fn test_count() -> anyhow::Result<()> {
        let pool = init_test_pool().await;
        EmailInbox::insert(&EmailInbox::default(), &pool).await?;
        let count = EmailInbox::count(&pool).await?;
        assert_eq!(count,1);
        Ok(())
    }
    #[tokio::test]
    async fn test_all() -> anyhow::Result<()> {
        //the test pool is just a memory sqlite.
        let pool = init_test_pool().await;

        //todo: uncomment below code and write your tests.
        /*
        let r = EmailInbox::insert(&EmailInbox {
             ..Default::default()
        }, &pool).await?;

        assert_eq!(r.rows_affected(), 1);

        let rows = EmailInbox::query(&EmailInbox {
            ..Default::default()
        }, &pool).await?;
        assert_eq!(rows.len(), 1);

        let r = EmailInbox::update(1, &EmailInbox {
            ..Default::default()
        }, &pool).await?;
        assert_eq!(r.rows_affected(), 1);

        let rows = EmailInbox::query(&EmailInbox {
            ..Default::default()
        }, &pool).await?;
        assert_eq!(rows[0].id, 1);

        let  r = EmailInbox::delete(1, &pool).await?;
        assert_eq!(r.rows_affected(),1);

        let rows = EmailInbox::query(&EmailInbox {
            ..Default::default()
        }, &pool).await?;
        assert_eq!(rows.len(), 0);

        */


        Ok(())
    }
}