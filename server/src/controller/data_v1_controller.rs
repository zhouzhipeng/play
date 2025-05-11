use std::collections::HashMap;

use axum::extract::{Path, Query};
use axum::Json;
use axum::response::IntoResponse;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize, Serializer};
use serde_json::Value;
use tracing::info;
use play_shared::constants::LUA_DIR;
use crate::{promise, JSON, method_router, return_error, S, get_last_insert_id, AppError, files_dir};
use crate::controller::pages_controller::PageDto;
use crate::tables::general_data::GeneralData;

method_router!(
    get : "/data/id-:data_id"-> get_data,
    put : "/data/id-:data_id"-> update_data,
    patch : "/data/id-:data_id"-> update_field,
    delete : "/data/id-:data_id"-> delete_data,
    put : "/data/cat-:cat"-> override_data,  // global data. insert or update.
    post : "/data/cat-:cat"-> insert_data,
    get : "/data/cat-:cat"-> query_data, // cat-pages?title=xxx&_select=title,url&_data_json=true

    //new mapping
    get : "/data/id/:data_id"-> get_data,
    get : "/data/cat/:cat/id/:data_id"-> get_data_under_cat,
    put : "/data/id/:data_id"-> update_data,
    patch : "/data/id/:data_id"-> update_field,
    delete : "/data/id/:data_id"-> delete_data,
    delete : "/data/cat/:cat"-> delete_category,
    put : "/data/cat/:cat"-> override_data,  // global data. insert or update.
    post : "/data/cat/:cat"-> insert_data,
    get : "/data/cat/:cat"-> query_data, // cat-pages?title=xxx&_select=title,url&_data_json=true
    get : "/data/cat/:cat/count"-> query_data_count,

);




#[derive(Deserialize)]
struct InsertDataReq {
    msg: String,
}

#[derive(Deserialize)]
struct UpdateDataTextReq {
    data: String,
}

async fn insert_data(s: S, Path(cat): Path<String>, body: String) -> JSON<Vec<QueryDataResp>> {
    //validation
    // ensure!(!vec!["id", "data","get","update","delete","list", "query"].contains(&cat.as_str()), "please use another category name ! ");
    // check!(serde_json::from_str::<Value>(&body).is_ok());

    let ret = GeneralData::insert(&cat,&body.trim(), &s.db).await?;
    let id = ret.rows_affected();
    promise!(id==1, "insert failed!");

    let data = GeneralData::query_by_id(get_last_insert_id!(ret) as u32, &s.db).await?;
    promise!(data.len()==1, "data error! query_by_id not found.");

    let data = data[0].clone();
    after_update_data(&data).await?;
    
    Ok(Json(vec![QueryDataResp::Raw(data)]))
}

async fn override_data(s: S, Path(cat): Path<String>, body: String) -> JSON<Vec<QueryDataResp>> {


    let list_data = GeneralData::query_by_cat("*", &cat,10, &s.db).await?;
    promise!(list_data.len()<=1, "A global category should have only one item!");

    if list_data.len() == 0 {
        //insert
        let ret = GeneralData::insert(&cat, &body.trim(), &s.db).await?;
        promise!(ret.rows_affected()==1, "GeneralData::insert error!");
        let data = GeneralData::query_by_id(get_last_insert_id!(ret) as u32, &s.db).await?;
        promise!(data.len()==1, "data error! query_by_id not found.");

        Ok(Json(vec![QueryDataResp::Raw(data[0].clone())]))
    } else {
        //update
        let r = GeneralData::update_data_by_cat(&cat, &body, &s.db).await?;
        promise!(r.rows_affected()==1, "update_data_by_cat error!");
        let data = GeneralData::query_by_cat_simple(&cat,1, &s.db).await?;
        promise!(data.len()==1, "data error! query_by_id not found.");
        Ok(Json(vec![QueryDataResp::Raw(data[0].clone())]))
    }
}

async fn delete_category(s: S, Path(cat): Path<String>) -> JSON<Vec<QueryDataResp>> {
    let r = GeneralData::delete_by_cat(&cat, &s.db).await?;
    Ok(Json(vec![]))
}

#[derive(Debug, Serialize, Default)]
pub struct GeneralDataJson {
    pub id: u32,
    pub cat: String,
    pub data: Value,
    #[serde(serialize_with = "serialize_as_timestamp")]
    pub created: chrono::NaiveDateTime,
    #[serde(serialize_with = "serialize_as_timestamp")]
    pub updated: chrono::NaiveDateTime,
}

fn serialize_as_timestamp<S>(date: &NaiveDateTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
{
    serializer.serialize_i64(date.timestamp_millis())
}


#[derive(Debug)]
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
    let limit = params.remove("_limit").unwrap_or("10".to_string()).parse::<i32>()?;


    let mut return_data = vec![];

    //support  one keyword field query only.
    if params.len() == 1 {
        for (k, v) in params {
            return_data = GeneralData::query_by_json_field(&select_fields, &cat, &k, &v, limit, &s.db).await?;

            break;
        }
    } else if params.len() == 0 {
        return_data = GeneralData::query_by_cat(&select_fields, &cat,limit, &s.db).await?;
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
async fn query_data_count(s: S, Path(cat): Path<String>) -> Result<String,AppError>{
    Ok(GeneralData::query_count(&cat ,&s.db).await?.to_string())
}

async fn get_data(s: S, Path(data_id): Path<u32>, Query(mut params): Query<HashMap<String, String>>) -> JSON<Vec<QueryDataResp>>  {
    let select_fields = params.remove("_select").unwrap_or("*".to_string());

    let data_to_json = params.remove("_json").unwrap_or("false".to_string()).eq("true");
    let data = GeneralData::query_by_id_with_select(&select_fields,data_id, &s.db).await?;
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
async fn get_data_under_cat(s: S, Path((cat,data_id)): Path<(String,u32)>, Query(mut params): Query<HashMap<String, String>>) -> JSON<Vec<QueryDataResp>>  {
    let select_fields = params.remove("_select").unwrap_or("*".to_string());

    let data_to_json = params.remove("_json").unwrap_or("false".to_string()).eq("true");
    let data = GeneralData::query_by_id_with_cat_select(&select_fields,data_id, &cat,&s.db).await?;
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


async fn update_data(s: S, Path(data_id): Path<u32>, body: String) -> JSON<Vec<QueryDataResp>> {
    let r = GeneralData::update_data_by_id(data_id, &body, &s.db).await?;
    promise!(r.rows_affected()==1, "update_data failed!");
    

    
    let data = GeneralData::query_by_id(data_id as u32, &s.db).await?;
    let data = data[0].clone();
  
    after_update_data(&data).await?;

    Ok(Json(vec![QueryDataResp::Raw(data)]))
}

async fn after_update_data(data: &GeneralData) -> Result<(), AppError> {
    //save *.lua page
    if data.cat.eq_ignore_ascii_case("pages") {
        let page_dto = serde_json::from_str::<PageDto>(&data.data)?;
        if page_dto.title.ends_with(".lua") {
            let save_path = std::path::Path::new(std::env::var(LUA_DIR).unwrap().as_str()).join(&page_dto.title);

            info!("ready to save lua file to save_path : {save_path:?}");
            let raw_content = String::from_utf8(hex::decode(&page_dto.content)?)?;

            //save to local file system
            tokio::fs::write(save_path, raw_content).await?;
        }
    }
    Ok(())
}

async fn update_field(s: S, Path(data_id): Path<u32>, Query(params): Query<HashMap<String, String>>) -> JSON<Vec<QueryDataResp>> {
    promise!(params.len()==1, "must specify only one pair param !");
    for (k, v) in params {
        let r = GeneralData::update_json_field_by_id(data_id, &k, &v, &s.db).await?;
        promise!(r.rows_affected()==1, "update_json_field_by_id failed!");
        let data = GeneralData::query_by_id(data_id as u32, &s.db).await?;
        return Ok(Json(vec![QueryDataResp::Raw(data[0].clone())]))
    }

    return_error!("unknown error")
}


async fn delete_data(s: S, Path(data_id): Path<u32>) -> JSON<Vec<QueryDataResp>> {
    let data = GeneralData::query_by_id(data_id as u32, &s.db).await?;
    promise!(data.len()==1, "query_by_id failed! length is not 1");
    let r = GeneralData::delete(data_id, &s.db).await?;
    promise!(r.rows_affected()==1, "delete failed!");
    Ok(Json(vec![QueryDataResp::Raw(data[0].clone())]))
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

    #[tokio::test]
    async fn test_query() -> anyhow::Result<()> {
        let state = mock_state!();
        insert_data(state.clone(), Path("book".to_string()), r#"
        {"name":"zzp"}
        "#.to_string()).await.unwrap();
        let result = get_data_under_cat(state, Path(("book".to_string()
                                        ,1)), Query(HashMap::new())).await;
        println!("resp : {:?}", result);
        assert!(result.is_ok());

        Ok(())
    }

    #[tokio::test]
    async fn test_query_count() -> anyhow::Result<()> {
        let state = mock_state!();
        insert_data(state.clone(), Path("book".to_string()), r#"
        {"name":"zzp"}
        "#.to_string()).await.unwrap();
        let result = query_data_count(state, Path("book".to_string())).await;
        println!("resp : {:?}", result);
        assert!(result.is_ok());

        Ok(())
    }
}