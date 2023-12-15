use serde::{Deserialize, Serialize};
use sqlx::{Error, FromRow};

use crate::tables::{DBPool, DBQueryResult};

#[derive(Clone, FromRow, Debug, Serialize, Deserialize)]
pub struct TodoItem {
    pub id: i64,
    pub title: String,
    pub status: String,
}

#[derive(Serialize,Deserialize)]
pub struct AddTodoItem {
    pub title: String,
    pub status: String,
}

#[derive(Serialize,Deserialize)]
pub struct UpdateTodoItem {
    pub title: String,
    pub status: String,
}

#[derive(Serialize,Deserialize)]
pub struct QueryTodoItem {
    pub title: String,

}

#[derive(Serialize,Deserialize)]
pub struct TodoItemVo {
    pub id: i64,
    pub title: String,
    pub status: String,
}


impl TodoItem {
    pub async fn insert(t: AddTodoItem, pool: &DBPool) -> Result<DBQueryResult, Error> {
        //todo: this is just a template code, write your own business.
        sqlx::query("INSERT INTO todo_item (title,status) VALUES (?,?)")
            .bind(&t.title)
            .bind(&t.status)
            .execute(pool)
            .await
    }

    pub async fn delete(id: i64, pool: &DBPool) -> Result<DBQueryResult, Error> {
        //todo: this is just a template code, write your own business.
        sqlx::query("DELETE from todo_item WHERE id =?")
            .bind(&id)
            .execute(pool)
            .await
    }

    pub async fn update(id: u32, t: UpdateTodoItem, pool: &DBPool) -> Result<DBQueryResult, Error> {
        //todo: this is just a template code, write your own business.
        sqlx::query("UPDATE todo_item set title=?,status=? WHERE id =?")
            .bind(&t.title)
            .bind(&t.status)
            .bind(&id)
            .execute(pool)
            .await
    }

    pub async fn query(q: QueryTodoItem, pool: &DBPool) -> Result<Vec<TodoItem>, Error> {
        //todo: this is just a template code, write your own business.
        sqlx::query_as::<_, TodoItem>("SELECT * FROM todo_item where title = ?")
            .bind(&q.title)
            .fetch_all(pool)
            .await
    }
    pub async fn get_by_id(id: u32, pool: &DBPool) -> Result<Vec<TodoItem>, Error> {
        sqlx::query_as::<_, TodoItem>("SELECT * FROM todo_item where id = ?")
            .bind(id)
            .fetch_all(pool)
            .await
    }
    pub async fn query_all(pool: &DBPool) -> Result<Vec<TodoItem>, Error> {
        sqlx::query_as::<_, TodoItem>("SELECT * FROM todo_item order by id desc")
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

        let r = TodoItem::insert(AddTodoItem {
            title: "todo 1".to_string(),
            status: "TODO".to_string(),
        }, &pool).await?;

        assert_eq!(r.rows_affected(), 1);

        let rows = TodoItem::query(QueryTodoItem {
            title: "todo 1".to_string(),
        }, &pool).await?;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, 1);

        let r = TodoItem::update(1, UpdateTodoItem {
            title: "todo 2".to_string(),
            status: "DONE".to_string(),
        }, &pool).await?;
        assert_eq!(r.rows_affected(), 1);

        let rows = TodoItem::query(QueryTodoItem {
            title: "todo 2".to_string(),
        }, &pool).await?;
        assert_eq!(rows[0].id, 1);

        let  r = TodoItem::delete(1, &pool).await?;
        assert_eq!(r.rows_affected(),1);

        let rows = TodoItem::query(QueryTodoItem {
            title: "todo 1".to_string(),
        }, &pool).await?;
        assert_eq!(rows.len(), 0);

        Ok(())
    }
}