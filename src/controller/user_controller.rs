use std::sync::Arc;

use axum::extract::{Query, State};
use axum::Router;
use axum::routing::get;
use serde::Deserialize;

use crate::AppState;
use crate::tables::user::User;

#[derive(Deserialize)]
struct Param {
    name: String,
}

pub fn init() -> Router<Arc<AppState>> {
    Router::new()
        .route("/users", get(user_list))
}

async fn user_list(name: Query<Param>, State(state): State<Arc<AppState>>) -> String {
    let users = User::query_users_by_name(name.0.name.as_str(), &state.db).await.unwrap();

    format!("users : {:?}", users)
}

