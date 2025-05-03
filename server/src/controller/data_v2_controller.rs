use crate::controller::pages_controller::PageDto;
use crate::tables::general_data::GeneralData;
use crate::{promise, files_dir, get_last_insert_id, method_router, return_error, AppError, JSON, R, S, hex_to_string};
use anyhow::{anyhow, bail, Context};
use axum::extract::{Path, Query};
use axum::response::IntoResponse;
use axum::Json;
use chrono::NaiveDateTime;
use play_shared::constants::LUA_DIR;
use regex::Regex;
use serde::{Deserialize, Serialize, Serializer};
use serde_json::{Map, Value};
use std::collections::HashMap;
use http::Uri;
use sqlx::Encode;
use tracing::info;

method_router!(
    get : "/api/v2/data/:cat/:action"-> all_in_one_api,
);

#[derive(Serialize, Deserialize, Debug)]
enum AllInOneActionEnum {
    #[serde(rename = "insert")]
    INSERT(HashMap<String, Value>),
    #[serde(rename = "update-full")]
    UPDATE_FULL,
    #[serde(rename = "update-field")]
    UPDATE_FIELD,
    #[serde(rename = "get-by-id")]
    GET_BY_ID(GetByIdParam),
    #[serde(rename = "get-all")]
    GET_ALL(GetAllParam),
}


#[derive(Serialize, Deserialize, Debug)]
struct GetAllParam {
    limit: u32,
    #[serde(rename = "where")]
    _where: String,
    order_by: String,
}
#[derive(Serialize, Deserialize, Debug)]
struct GetByIdParam {
    id: u32,
}

const ACTION_NAME: &str = "_action";
const DATA_NAME: &str = "_data";



/// 将查询字符串解析为JSON，并自动转换类型
pub fn parse_query_with_types(str_params: HashMap<String, String> ) -> anyhow::Result<Value> {

    // 然后自动转换类型
    let mut json_map = Map::new();
    for (key, value) in str_params {
        // 尝试布尔值
        if value.eq_ignore_ascii_case("true") {
            json_map.insert(key, Value::Bool(true));
        } else if value.eq_ignore_ascii_case("false") {
            json_map.insert(key, Value::Bool(false));
        }
        // 尝试空值
        else if value.eq_ignore_ascii_case("null") || value.is_empty() {
            json_map.insert(key, Value::Null);
        }
        // 尝试整数
        else if let Ok(num) = value.parse::<i64>() {
            json_map.insert(key, Value::Number(num.into()));
        }
        // 尝试浮点数
        else if let Ok(num) = value.parse::<f64>() {
            // 检查浮点数是否有效（非NaN或无穷大）
            if num.is_finite() {
                if let Some(n) = serde_json::Number::from_f64(num) {
                    json_map.insert(key, Value::Number(n));
                } else {
                    json_map.insert(key, Value::String(value));
                }
            } else {
                json_map.insert(key, Value::String(value));
            }
        }
        // 默认为字符串
        else {
            json_map.insert(key, Value::String(value));
        }
    }

    Ok(Value::Object(json_map))
}


async fn all_in_one_api(
    s: S,
    Path((cat, action)): Path<(String,String)>,
    Query(mut params): Query<HashMap<String, String>>,
) -> R<String> {
    promise!(
        Regex::new(r"^[a-z0-9-]{2,10}$")?.is_match(&cat),
        "invalid `category` path : {} , not match with : {}",
        cat, "^[a-z0-9-]{2,10}$"
    );

    if params.len() == 1 && params.contains_key("hex") {
        let hex_params = params.remove("hex").unwrap_or_default();
        let bytes = hex::decode(&hex_params)?;
        let raw_query_str =  String::from_utf8(bytes)?;
        params = serde_qs::from_str(&raw_query_str)?;
    }


    // 获取查询字符串
    let params = parse_query_with_types(params)?;

    let mut new_params = HashMap::new();
    new_params.insert(action.to_string(), params);

    let action_enum: AllInOneActionEnum = serde_json::from_value(serde_json::to_value(new_params)?)?;
    info!("action: {:?}", action_enum);


    match &action_enum {
        AllInOneActionEnum::INSERT(val) => {
            promise!(val.len()!=0, "query params cant be empty!");


        }
        AllInOneActionEnum::UPDATE_FULL => {}
        AllInOneActionEnum::UPDATE_FIELD => {}
        AllInOneActionEnum::GET_BY_ID(_) => {}
        AllInOneActionEnum::GET_ALL(param) => {

        }
    }


    Ok(serde_json::to_string(&action_enum)?)
}

async fn insert_data(s: S, Path(cat): Path<String>, body: String) -> JSON<Vec<QueryDataResp>> {
    //validation
    // ensure!(!vec!["id", "data","get","update","delete","list", "query"].contains(&cat.as_str()), "please use another category name ! ");
    // check!(serde_json::from_str::<Value>(&body).is_ok());

    let ret = GeneralData::insert(&cat, &body.trim(), &s.db).await?;
    let id = ret.rows_affected();
    promise!(id == 1, "insert failed!");

    let data = GeneralData::query_by_id(get_last_insert_id!(ret) as u32, &s.db).await?;
    promise!(data.len() == 1, "data error! query_by_id not found.");

    let data = data[0].clone();
    after_update_data(&data).await?;

    Ok(Json(vec![QueryDataResp::Raw(data)]))
}

async fn override_data(s: S, Path(cat): Path<String>, body: String) -> JSON<Vec<QueryDataResp>> {
    let list_data = GeneralData::query_by_cat("*", &cat, 10, &s.db).await?;
    promise!(
        list_data.len() <= 1,
        "A global category should have only one item!"
    );

    if list_data.len() == 0 {
        //insert
        let ret = GeneralData::insert(&cat, &body.trim(), &s.db).await?;
        promise!(ret.rows_affected() == 1, "GeneralData::insert error!");
        let data = GeneralData::query_by_id(get_last_insert_id!(ret) as u32, &s.db).await?;
        promise!(data.len() == 1, "data error! query_by_id not found.");

        Ok(Json(vec![QueryDataResp::Raw(data[0].clone())]))
    } else {
        //update
        let r = GeneralData::update_data_by_cat(&cat, &body, &s.db).await?;
        promise!(r.rows_affected() == 1, "update_data_by_cat error!");
        let data = GeneralData::query_by_cat_simple(&cat, 1, &s.db).await?;
        promise!(data.len() == 1, "data error! query_by_id not found.");
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
enum QueryDataResp {
    Raw(GeneralData),
    Json(GeneralDataJson),
}

impl Serialize for QueryDataResp {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            QueryDataResp::Raw(a) => a.serialize(serializer),
            QueryDataResp::Json(a) => a.serialize(serializer),
        }
    }
}

//
// #[debug_handler]
async fn query_data(
    s: S,
    Path(cat): Path<String>,
    Query(mut params): Query<HashMap<String, String>>,
) -> JSON<Vec<QueryDataResp>> {
    //options
    let select_fields = params.remove("_select").unwrap_or("*".to_string());
    let data_to_json = params
        .remove("_json")
        .unwrap_or("false".to_string())
        .eq("true");
    let limit = params
        .remove("_limit")
        .unwrap_or("10".to_string())
        .parse::<i32>()?;
    let limit = params
        .remove("_limit")
        .unwrap_or("10".to_string())
        .parse::<i32>()?;

    let mut return_data = vec![];

    //support  one keyword field query only.
    if params.len() == 1 {
        for (k, v) in params {
            return_data =
                GeneralData::query_by_json_field(&select_fields, &cat, &k, &v, limit, &s.db)
                    .await?;

            break;
        }
    } else if params.len() == 0 {
        return_data = GeneralData::query_by_cat(&select_fields, &cat, limit, &s.db).await?;
    }

    return if data_to_json {
        let data = return_data
            .iter()
            .map(|d| {
                QueryDataResp::Json(GeneralDataJson {
                    id: d.id,
                    cat: d.cat.to_string(),
                    data: serde_json::from_str::<Value>(&d.data)
                        .unwrap_or(Value::String(d.data.to_string())),
                    created: d.created,
                    updated: d.updated,
                })
            })
            .collect();
        Ok(Json(data))
    } else {
        let data = return_data
            .iter()
            .map(|d| QueryDataResp::Raw(d.clone()))
            .collect();
        Ok(Json(data))
    };
}
async fn query_data_count(s: S, Path(cat): Path<String>) -> Result<String, AppError> {
    Ok(GeneralData::query_count(&cat, &s.db).await?.to_string())
}

async fn get_data_under_cat(
    s: S,
    Path((cat, data_id)): Path<(String, u32)>,
    Query(mut params): Query<HashMap<String, String>>,
) -> JSON<Vec<QueryDataResp>> {
    let select_fields = params.remove("_select").unwrap_or("*".to_string());

    let data_to_json = params
        .remove("_json")
        .unwrap_or("false".to_string())
        .eq("true");
    let data =
        GeneralData::query_by_id_with_cat_select(&select_fields, data_id, &cat, &s.db).await?;
    return if data_to_json {
        let data = data
            .iter()
            .map(|d| {
                QueryDataResp::Json(GeneralDataJson {
                    id: d.id,
                    cat: d.cat.to_string(),
                    data: serde_json::from_str::<Value>(&d.data)
                        .unwrap_or(Value::String(d.data.to_string())),
                    created: d.created,
                    updated: d.updated,
                })
            })
            .collect();
        Ok(Json(data))
    } else {
        let data = data.iter().map(|d| QueryDataResp::Raw(d.clone())).collect();
        Ok(Json(data))
    };
}

async fn update_data(s: S, Path(data_id): Path<u32>, body: String) -> JSON<Vec<QueryDataResp>> {
    let r = GeneralData::update_data_by_id(data_id, &body, &s.db).await?;
    promise!(r.rows_affected() == 1, "update_data failed!");

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
            let save_path = std::path::Path::new(std::env::var(LUA_DIR).unwrap().as_str())
                .join(&page_dto.title);

            info!("ready to save lua file to save_path : {save_path:?}");
            let raw_content = String::from_utf8(hex::decode(&page_dto.content)?)?;

            //save to local file system
            tokio::fs::write(save_path, raw_content).await?;
        }
    }
    Ok(())
}

async fn update_field(
    s: S,
    Path(data_id): Path<u32>,
    Query(params): Query<HashMap<String, String>>,
) -> JSON<Vec<QueryDataResp>> {
    promise!(params.len() == 1, "must specify only one pair param !");
    for (k, v) in params {
        let r = GeneralData::update_json_field_by_id(data_id, &k, &v, &s.db).await?;
        promise!(r.rows_affected() == 1, "update_json_field_by_id failed!");
        let data = GeneralData::query_by_id(data_id as u32, &s.db).await?;
        return Ok(Json(vec![QueryDataResp::Raw(data[0].clone())]));
    }

    return_error!("unknown error")
}

async fn delete_data(s: S, Path(data_id): Path<u32>) -> JSON<Vec<QueryDataResp>> {
    let data = GeneralData::query_by_id(data_id as u32, &s.db).await?;
    promise!(data.len() == 1, "query_by_id failed! length is not 1");
    let r = GeneralData::delete(data_id, &s.db).await?;
    promise!(r.rows_affected() == 1, "delete failed!");
    Ok(Json(vec![QueryDataResp::Raw(data[0].clone())]))
}

#[cfg(test)]
mod tests {
    use crate::mock_state;

    use super::*;

    #[tokio::test]
    async fn test_insert_data() -> anyhow::Result<()> {
        let result = insert_data(
            mock_state!(),
            Path("book".to_string()),
            r#"
        {"name":"zzp"}
        "#
            .to_string(),
        )
        .await;
        println!("resp : {:?}", result);
        assert!(result.is_ok());

        Ok(())
    }

    #[tokio::test]
    async fn test_query() -> anyhow::Result<()> {
        let state = mock_state!();
        insert_data(
            state.clone(),
            Path("book".to_string()),
            r#"
        {"name":"zzp"}
        "#
            .to_string(),
        )
        .await
        .unwrap();
        let result =
            get_data_under_cat(state, Path(("book".to_string(), 1)), Query(HashMap::new())).await;
        println!("resp : {:?}", result);
        assert!(result.is_ok());

        Ok(())
    }

    #[tokio::test]
    async fn test_query_count() -> anyhow::Result<()> {
        let state = mock_state!();
        insert_data(
            state.clone(),
            Path("book".to_string()),
            r#"
        {"name":"zzp"}
        "#
            .to_string(),
        )
        .await
        .unwrap();
        let result = query_data_count(state, Path("book".to_string())).await;
        println!("resp : {:?}", result);
        assert!(result.is_ok());

        Ok(())
    }
}
