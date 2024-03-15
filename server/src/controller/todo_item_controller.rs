use std::sync::Arc;
use anyhow::{bail, ensure};
use axum::extract::Query;
use axum::response::Html;

use axum::{Form, Router};
use axum::routing::{get, post};
use serde::Deserialize;
use serde_json::json;

use crate::{AppState, check, get_last_insert_id, template};
use crate::{HTML, S};
use crate::tables::todo_item::{AddTodoItem, TodoItem, UpdateTodoItem};

pub fn init() -> Router<Arc<AppState>> {
    Router::new()
        .route("/todo/list", get(todo_list))
        .route("/todo/mark-done", get(mark_done))
        .route("/todo/add-todo", post(add_todo))
        .route("/todo/delete", get(delete))
}



async fn todo_list(s: S) -> HTML {
    let items = TodoItem::query_all(&s.db).await?;
    template!(s, "frame.html"+"todo_item/list.html", json!({
        "items": items
    }))
}

#[derive(Deserialize)]
struct TodoId {
    id: u32,
}


async fn delete(Query(todoId): Query<TodoId>, s: S) -> HTML {
    let items = TodoItem::delete(todoId.id as i64, &s.db).await?;
    check!(items.rows_affected()==1, format!("item {} delete failed.", todoId.id));
    Ok(Html("".to_string()))
}
async fn mark_done(Query(todoId): Query<TodoId>, s: S) -> HTML {
    let items = TodoItem::get_by_id(todoId.id, &s.db).await?;
    check!(items.len()==1, format!("item {} not found.", todoId.id));

    let update_result = TodoItem::update(todoId.id, UpdateTodoItem { title: (&items[0].title).to_string(), status: "DONE".to_string() }, &s.db).await?;
    check!(update_result.rows_affected()==1, format!("todo item : {} update failed!", todoId.id));

    let items = TodoItem::get_by_id(todoId.id, &s.db).await?;
    check!(items.len()==1, format!("item {} not found.", todoId.id));


    template!(s, "todo_item/todo_item.html", json!({
        "item": items[0]
    }))
}

#[derive(Deserialize)]
struct AddTodoReq{
    title : String,
}

// #[axum::debug_handler]
async fn add_todo(s: S , Form(add_todo): Form<AddTodoReq>) -> HTML {
    let r = TodoItem::insert(AddTodoItem{
        title: add_todo.title,
        status: "TODO".to_string(),
    }, &s.db).await?;

    check!(r.rows_affected()==1 , "insert error!");


    let items = TodoItem::get_by_id(get_last_insert_id!(r) as u32, &s.db).await?;
    check!(items.len()==1 , "get_by_id error!");

    template!(s, "todo_item/todo_item.html", json!({
        "item": items[0]
    }))
}
