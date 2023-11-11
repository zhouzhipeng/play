use std::sync::Arc;
use anyhow::{anyhow, bail, Context};

use axum::{Json, Router};
use axum::extract::{Path, Query, State};
use axum::routing::get;

use crate::AppState;
use crate::controller::{AppError, R, S};
use crate::tables::Table;
use crate::tables::user::{AddUser, QueryUser, UpdateUser, User};

pub fn init() -> Router<Arc<AppState>> {
    Router::new()
        .route("/users", get(user_list))
        .route("/add-user", get(add_user))
        .route("/update-user/:user_id", get(modify_user))
        .route("/delete-user/:user_id", get(delete_user))
}

async fn user_list(Query(q): Query<QueryUser>, State(state): S) -> R<Json<Vec<User>>> {
    let users = User::query(q, &state.db).await?;

    Ok(Json(users))
}

async fn add_user(Query(q): Query<AddUser>, State(state): S) -> R<String> {
    let r = User::insert(q, &state.db).await?;

    Ok(format!("rows affected : {}", r.rows_affected()))
}

async fn modify_user(Path(user_id): Path<i64>, Query(q): Query<UpdateUser>, State(state): S) -> R<String> {
    let r = User::update(user_id, q, &state.db).await?;

    Ok(format!("rows affected : {}", r.rows_affected()))
}

async fn delete_user(Path(user_id): Path<i64>, State(state): S) -> R<String> {
    // let e = AppError(anyhow!("eerr"));
    // bail!("test error");
    // return Err(anyhow!("test error").into())
    let r = User::delete(user_id, &state.db).await?;
    //
    Ok(format!("rows affected : {}", r.rows_affected()))
}

