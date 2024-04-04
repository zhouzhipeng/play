use std::collections::HashMap;

use axum::{Form, Json};
use axum::extract::{Path, Query};
use axum::response::{IntoResponse, Response};
use axum_macros::debug_handler;
use serde::{Deserialize, Serialize, Serializer};
use serde_json::Value;
use tracing::info;

use crate::{ensure, JSON, method_router, R, return_error, S};
use crate::tables::general_data::GeneralData;

method_router!(
    get : "/data/id-:data_id"-> get_data,
    put : "/data/id-:data_id"-> update_data,
    patch : "/data/id-:data_id"-> update_field,
    delete : "/data/id-:data_id"-> delete_data,
    post : "/data/cat-g-:cat"-> override_data,  // g-xxx  means a global category , should have only one item. so when call insert_data twice it will override data.
    get : "/data/cat-g-:cat"-> query_data_global,
    post : "/data/cat-:cat"-> insert_data,
    get : "/data/cat-:cat"-> query_data, // cat-pages?title=xxx&_select=title,url&_data_json=true

);



#[derive(Serialize, Debug)]
struct MsgResp {
    msg: String,
    id_or_count: u32,
}

#[derive(Deserialize)]
struct InsertDataReq {
    msg: String,
}

#[derive(Deserialize)]
struct UpdateDataTextReq {
    data: String,
}

async fn insert_data(s: S, Path(cat): Path<String>, body: String) -> JSON<MsgResp> {
    //validation
    ensure!(!vec!["id", "data","get","update","delete","list", "query"].contains(&cat.as_str()));
    // check!(serde_json::from_str::<Value>(&body).is_ok());

    let data = GeneralData {
        cat,
        data: body.trim().to_string(),
        ..GeneralData::default()
    };

    let ret = GeneralData::insert(&data, &s.db).await?;
    let id = ret.rows_affected();
    ensure!(id==1);

    Ok(Json(MsgResp { msg: "ok".to_string(), id_or_count: ret.last_insert_rowid() as u32 }))
}

async fn override_data(s: S, Path(cat): Path<String>, body: String) -> JSON<MsgResp> {
    let cat = format!("g-{}", cat);

    let list_data = GeneralData::query("*", &cat, &s.db).await?;
    ensure!(list_data.len()<=1, "A global category should have only one item!");

    if list_data.len() == 0 {
        //insert
        let data = GeneralData {
            cat,
            data: body.trim().to_string(),
            ..GeneralData::default()
        };

        let ret = GeneralData::insert(&data, &s.db).await?;
        let id = ret.rows_affected();
        ensure!(id==1);
        Ok(Json(MsgResp { msg: "ok".to_string(), id_or_count: ret.last_insert_rowid() as u32 }))
    } else {
        //update
        let data = GeneralData::update_text_global(&cat, &body, &s.db).await?;
        return Ok(Json(MsgResp { msg: "update ok".to_string(), id_or_count: data.rows_affected() as u32 }));
    }
}
#[derive(Debug, Serialize, Default)]
pub struct GeneralDataJson {
    pub id: u32,
    pub cat: String,
    pub data: Value,
    pub created: chrono::NaiveDateTime,
    pub updated: chrono::NaiveDateTime,
}

enum QueryDataResp{
    Raw(GeneralData),
    Json(GeneralDataJson),
}

impl Serialize for QueryDataResp {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        match self{
            QueryDataResp::Raw(a) => a.serialize(serializer),
            QueryDataResp::Json(a) => a.serialize(serializer),
        }
    }
}


//
// #[debug_handler]
async fn query_data(s: S, Path(cat): Path<String>, Query(mut params): Query<HashMap<String, String>>) -> JSON<Vec<QueryDataResp>> {
    //options
    let select_fields = params.remove("_select").unwrap_or("*".to_string());
    let data_to_json = params.remove("_json").unwrap_or("false".to_string()).eq("true");

    let mut return_data = vec![];

    //support  one keyword field query only.
    if params.len() == 1 {
        for (k, v) in params {
            return_data = GeneralData::query_json(&select_fields, &cat, &k, &v, &s.db).await?;

            break;
        }
    } else if params.len() == 0 {
        return_data = GeneralData::query(&select_fields, &cat, &s.db).await?;
    }


    return if data_to_json {
        let data = return_data.iter().map(|d| QueryDataResp::Json(GeneralDataJson {
            id: d.id,
            cat: d.cat.to_string(),
            data: serde_json::from_str::<Value>(&d.data).unwrap_or(Value::String(d.data.to_string())),
            created: d.created,
            updated: d.updated,
        })).collect();
        Ok(Json(data))
    } else {
        let data = return_data.iter().map(|d| QueryDataResp::Raw(d.clone())).collect();
        Ok(Json(data))
    }

}

async fn query_data_global(s: S, Path(cat): Path<String>) -> JSON<Vec<GeneralData>> {
    let cat = format!("g-{}", cat);
    let data = GeneralData::query("*", &cat, &s.db).await?;
    return Ok(Json(data));
}
async fn get_data(s: S, Path(data_id): Path<u32>, Query(mut params): Query<HashMap<String, String>>) -> JSON<Vec<QueryDataResp>>  {
    let data_to_json = params.remove("_json").unwrap_or("false".to_string()).eq("true");

    let data = GeneralData::query_by_id(data_id, &s.db).await?;
    return if data_to_json {
        let data = data.iter().map(|d| QueryDataResp::Json(GeneralDataJson {
            id: d.id,
            cat: d.cat.to_string(),
            data: serde_json::from_str::<Value>(&d.data).unwrap_or(Value::String(d.data.to_string())),
            created: d.created,
            updated: d.updated,
        })).collect();
        Ok(Json(data))
    } else {
        let data = data.iter().map(|d| QueryDataResp::Raw(d.clone())).collect();
        Ok(Json(data))
    }
}


async fn update_data(s: S, Path(data_id): Path<u32>, body: String) -> JSON<MsgResp> {
    let data = GeneralData::update_text(data_id, &body, &s.db).await?;
    return Ok(Json(MsgResp { msg: "update ok".to_string(), id_or_count: data.rows_affected() as u32 }));
}

async fn update_field(s: S, Path(data_id): Path<u32>, Query(params): Query<HashMap<String, String>>) -> JSON<MsgResp> {
    ensure!(params.len()==1);
    for (k, v) in params {
        let data = GeneralData::update_json(data_id, &k, &v, &s.db).await?;
        return Ok(Json(MsgResp { msg: "update ok".to_string(), id_or_count: data.rows_affected() as u32 }));
    }

    return_error!("unknown error")
}


async fn delete_data(s: S, Path(data_id): Path<u32>) -> JSON<MsgResp> {
    let data = GeneralData::delete(data_id, &s.db).await?;
    return Ok(Json(MsgResp { msg: "delete ok".to_string(), id_or_count: data.rows_affected() as u32 }));
}


#[cfg(test)]
mod tests {
    use crate::mock_state;

    use super::*;

    #[tokio::test]
    async fn test_insert_data() -> anyhow::Result<()> {
        let result = insert_data(mock_state!(), Path("book".to_string()), r#"
        {"name":"zzp"}
        "#.to_string()).await;
        println!("resp : {:?}", result);
        assert!(result.is_ok());

        Ok(())
    }
}