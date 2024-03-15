use std::collections::HashMap;
use axum::extract::{Path, Query};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{check_if, HTML, JSON, method_router, R, return_error, S, template};
use crate::tables::general_data::GeneralData;

method_router!(
    get : "/general-data/list"-> list,
    post : "/general-data/:meta_id/insert"-> insert_data,
    get : "/general-data/:meta_id/list"-> list_data,
    get : "/general-data/:meta_id/query"-> query_data,
);

async fn list(s: S) -> HTML {
    template!(s, "frame.html" + "general_data/list.html", json!({}))
}

#[derive(Serialize)]
struct MsgResp {
    msg: String,
}

#[derive(Deserialize)]
struct InsertDataReq {
    msg: String,
}

async fn insert_data(s: S, Path(meta_id): Path<u32>, body: String) -> JSON<MsgResp> {
    let data = GeneralData {
        meta_id,
        data: body.trim().to_string(),
        ..GeneralData::default()
    };
    let id = GeneralData::insert(&data, &s.db).await?.rows_affected();
    check_if!(id==1);

    Ok(Json(MsgResp { msg: "ok".to_string() }))
}

async fn list_data(s: S, Path(meta_id): Path<u32>) -> JSON<Vec<GeneralData>> {
    let q = GeneralData {
        meta_id,
        ..GeneralData::default()
    };
    let data = GeneralData::query(&q, &s.db).await?;

    Ok(Json(data))
}
async fn query_data(s: S, Path(meta_id): Path<u32>,Query(params): Query<HashMap<String, String>> ) -> R<String> {
    check_if!(params.len()==1);
    for (k, v) in params {
        let data = GeneralData::query_json(meta_id,&k, &v, &s.db).await?;
        // data.iter().map(|d|d.data)
        // return Ok(Json(data));
        return Ok("".to_string());
    }

    return_error!("unknown error")

}


#[cfg(test)]
mod tests {
    use crate::mock_state;

    use super::*;

    #[tokio::test]
    async fn test_insert_data() -> anyhow::Result<()> {
        let result = insert_data(mock_state!(), Path(1), "abc".to_string()).await;
        assert!(result.is_ok());

        Ok(())
    }
}