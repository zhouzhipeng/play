use std::collections::HashMap;

use axum::extract::{Path, Query};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{check_if, JSON, method_router, R, return_error, S};
use crate::tables::general_data::GeneralData;

method_router!(
    post : "/data/:cat/insert"-> insert_data,
    get : "/data/:cat/list"-> list_data,
    get : "/data/:cat/query"-> query_data,
    get : "/data/get/:data_id"-> get_data,
    put : "/data/update/:data_id"-> update_data,
    delete : "/data/delete/:data_id"-> delete_data,
);

#[derive(Serialize, Debug)]
struct MsgResp {
    msg: String,
}

#[derive(Deserialize)]
struct InsertDataReq {
    msg: String,
}

async fn insert_data(s: S, Path(cat): Path<String>, body: String) -> JSON<MsgResp> {
    //validation
    check_if!(!vec!["data","get","update","delete","list", "query"].contains(&cat.as_str()));
    check_if!(serde_json::from_str::<Value>(&body).is_ok());

    let data = GeneralData {
        cat,
        data: body.trim().to_string(),
        ..GeneralData::default()
    };
    let id = GeneralData::insert(&data, &s.db).await?.rows_affected();
    check_if!(id==1);

    Ok(Json(MsgResp { msg: "ok".to_string() }))
}

async fn list_data(s: S, Path(cat): Path<String>) -> R<String> {
    let q = GeneralData {
        cat,
        ..GeneralData::default()
    };
    let data = GeneralData::query(&q, &s.db).await?;
    let mut final_data = data.iter().map(|d| d.data.to_string())
        .collect::<Vec<_>>()
        .join(",");

    final_data = format!("[{}]", final_data);


    Ok(final_data)
}

async fn query_data(s: S, Path(cat): Path<String>, Query(params): Query<HashMap<String, String>>) -> R<String> {
    check_if!(params.len()==1);
    for (k, v) in params {
        let data = GeneralData::query_json(&cat, &k, &v, &s.db).await?;
        let mut final_data = data.iter().map(|d| d.data.to_string())
            .collect::<Vec<_>>()
            .join(",");

        final_data = format!("[{}]", final_data);

        return Ok(final_data);
    }

    return_error!("unknown error")
}

async fn get_data(s: S, Path(data_id): Path<u32>) -> R<String> {
    let data = GeneralData::get_one(data_id, &s.db).await?;
    match data {
        None => {
            Ok("".to_string())
        }
        Some(s) => {
            Ok(s.data)
        }
    }
}

async fn update_data(s: S, Path(data_id): Path<u32>, Query(params): Query<HashMap<String, String>>) -> JSON<MsgResp> {
    check_if!(params.len()==1);
    for (k, v) in params {
        let data = GeneralData::update_json(data_id, &k, &v, &s.db).await?;
        return Ok(Json(MsgResp { msg: "update ok".to_string() }));
    }

    return_error!("unknown error")
}

async fn delete_data(s: S, Path(data_id): Path<u32>) -> JSON<MsgResp> {
    let data = GeneralData::delete(data_id, &s.db).await?;
    return Ok(Json(MsgResp { msg: "delete ok".to_string() }));
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