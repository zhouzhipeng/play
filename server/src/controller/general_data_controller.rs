use std::collections::HashMap;

use axum::{Form, Json};
use axum::extract::{Path, Query};
use serde::{Deserialize, Serialize};

use crate::{check, JSON, method_router, return_error, S};
use crate::tables::general_data::GeneralData;

method_router!(
    get : "/data/id-:data_id"-> get_data,
    put : "/data/id-:data_id"-> update_data,
    patch : "/data/id-:data_id"-> update_field,
    delete : "/data/id-:data_id"-> delete_data,
    post : "/data/cat-g-:cat"-> override_data,  // g-xxx  means a global category , should have only one item. so when call insert_data twice it will override data.
    get : "/data/cat-g-:cat"-> query_data_global,
    post : "/data/cat-:cat"-> insert_data,
    get : "/data/cat-:cat"-> query_data,

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
    check!(!vec!["id", "data","get","update","delete","list", "query"].contains(&cat.as_str()));
    // check!(serde_json::from_str::<Value>(&body).is_ok());

    let data = GeneralData {
        cat,
        data: body.trim().to_string(),
        ..GeneralData::default()
    };

    let ret = GeneralData::insert(&data, &s.db).await?;
    let id = ret.rows_affected();
    check!(id==1);

    Ok(Json(MsgResp { msg: "ok".to_string(), id_or_count: ret.last_insert_rowid() as u32 }))
}

async fn override_data(s: S, Path(cat): Path<String>, body: String) -> JSON<MsgResp> {
    let cat = format!("g-{}", cat);

    let list_data = GeneralData::query(&cat, &s.db).await?;
    check!(list_data.len()<=1, "A global category should have only one item!");

    if list_data.len() == 0 {
        //insert
        let data = GeneralData {
            cat,
            data: body.trim().to_string(),
            ..GeneralData::default()
        };

        let ret = GeneralData::insert(&data, &s.db).await?;
        let id = ret.rows_affected();
        check!(id==1);
        Ok(Json(MsgResp { msg: "ok".to_string(), id_or_count: ret.last_insert_rowid() as u32 }))
    } else {
        //update
        let data = GeneralData::update_text_global(&cat, &body, &s.db).await?;
        return Ok(Json(MsgResp { msg: "update ok".to_string(), id_or_count: data.rows_affected() as u32 }));
    }
}


async fn query_data(s: S, Path(cat): Path<String>, Query(params): Query<HashMap<String, String>>) -> JSON<Vec<GeneralData>> {
    if params.len() == 1 {
        for (k, v) in params {
            let data = GeneralData::query_json(&cat, &k, &v, &s.db).await?;
            return Ok(Json(data));
        }
    } else if params.len() == 0 {
        let data = GeneralData::query(&cat, &s.db).await?;
        return Ok(Json(data));
    }


    return_error!("unknown error")
}

async fn query_data_global(s: S, Path(cat): Path<String>) -> JSON<Vec<GeneralData>> {
    let cat = format!("g-{}", cat);
    let data = GeneralData::query(&cat, &s.db).await?;
    return Ok(Json(data));
}
async fn get_data(s: S, Path(data_id): Path<u32>) -> JSON<Vec<GeneralData>> {
    let data = GeneralData::query_by_id(data_id, &s.db).await?;
    Ok(Json(data))
}


async fn update_data(s: S, Path(data_id): Path<u32>, body: String) -> JSON<MsgResp> {
    let data = GeneralData::update_text(data_id, &body, &s.db).await?;
    return Ok(Json(MsgResp { msg: "update ok".to_string(), id_or_count: data.rows_affected() as u32 }));
}

async fn update_field(s: S, Path(data_id): Path<u32>, Query(params): Query<HashMap<String, String>>) -> JSON<MsgResp> {
    check!(params.len()==1);
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