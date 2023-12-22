use std::sync::Arc;

use axum::{Form, Json, Router};
use axum::extract::Query;
use axum::routing::{get, post};
use serde::Deserialize;
use serde_json::json;

use crate::{AppState, check, template};
use crate::{HTML, JSON, R, S};
use crate::tables::api_entry::{ApiEntry, UpdateApiEntry};

pub fn init() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api-entry/list", get(list))
        .route("/api-entry/get", get(get_by_id))
        .route("/api-entry/save", post(save))
        .route("/api-entry/delete", get(delete))
}

async fn list(s: S) -> HTML {
    let items = ApiEntry::query_all(&s.db).await?;
    template!(s, "api_entry/api-manager.html", json!({
        "items": items
    }))
}

#[derive(Deserialize)]
struct Id {
    id: i32,
}

async fn get_by_id(s: S, Query(id): Query<Id>) -> JSON<Vec<ApiEntry>> {
    let items = ApiEntry::query_by_id(id.id, &s.db).await?;
    Ok(Json(items))
}


// #[axum::debug_handler]
async fn save(s: S,Form(entry): Form<UpdateApiEntry>) -> R<String> {
    let r = match entry.id {
        None => {
            //insert
            ApiEntry::insert(entry, &s.db).await?
        }
        Some(id) => {
            //update
            ApiEntry::update(id, entry, &s.db).await?
        }
    };
    check!(r.rows_affected()==1, "update api entry error!");
    Ok(r.rows_affected().to_string())
}
async fn delete(s: S ,Query(id): Query<Id>) -> R<String> {
    let r = ApiEntry::delete(id.id as i64, &s.db).await?;
    check!(r.rows_affected()==1, "delete api entry error!");
    Ok(r.rows_affected().to_string())
}