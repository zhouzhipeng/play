use crate::controller::pages_controller::PageDto;
use crate::tables::general_data::GeneralData;
use crate::{
    files_dir, get_last_insert_id, hex_to_string, method_router, promise, return_error, AppError,
    JSON, R, S,
};
use anyhow::{anyhow, bail, ensure, Context, Result};
use axum::extract::{Path, Query};
use axum::response::IntoResponse;
use axum::Json;
use chrono::NaiveDateTime;
use dioxus::html::completions::CompleteWithBraces::param;
use http::Uri;
use play_shared::constants::LUA_DIR;
use regex::Regex;
use serde::{Deserialize, Serialize, Serializer};
use serde_json::{Map, Value};
use sqlx::Encode;
use std::collections::HashMap;
use tracing::info;

method_router!(
    get : "/api/v2/data/:category/:action"-> all_in_one_api,
    post : "/api/v2/data/:category/:action"-> all_in_one_api_post,
    get : "/api/v2/data/:category/:action/:hex"-> all_in_one_api_hex,
);

#[derive(Serialize, Deserialize, Debug)]
enum ActionEnum {
    #[serde(rename = "insert")]
    INSERT(HashMap<String, Value>),
    #[serde(rename = "update")]
    UPDATE(UpdateParam),
    #[serde(rename = "query")]
    QUERY(QueryParam),
    #[serde(rename = "delete")]
    DELETE(DeleteParam),
}

#[derive(Serialize, Deserialize, Debug)]
struct QueryParam {
    id: Option<u32>,
    select: Option<String>,
    #[serde(default = "default_limit")]
    limit: u32,
    #[serde(rename = "where")]
    _where: Option<String>,
    order_by: Option<String>,
    #[serde(default)]
    less: bool,
    #[serde(default)]
    count: bool,
}

fn default_limit() -> u32 {
    10
}

#[derive(Serialize, Deserialize, Debug)]
struct UpdateParam {
    id: u32,
    set: String,
}
#[derive(Serialize, Deserialize, Debug)]
struct DeleteParam {
    id: u32,
    #[serde(default)]
    hard_delete: bool,
}

fn parse_and_convert_to_json_extract(condition_str: &str) -> String {
    // 分割条件字符串，忽略 "AND" 的大小写
    let mut result = Vec::new();
    let mut current_part = String::new();
    let mut in_quotes = false;
    let mut chars = condition_str.chars().peekable();

    while let Some(c) = chars.next() {
        // 处理引号内的内容
        if c == '\'' || c == '"' {
            in_quotes = !in_quotes;
            current_part.push(c);
            continue;
        }

        // 在引号内，直接添加字符
        if in_quotes {
            current_part.push(c);
            continue;
        }

        // 检查是否遇到 "AND"（忽略大小写）
        if (c == 'a' || c == 'A') &&
            chars.peek() == Some(&'n') || chars.peek() == Some(&'N') {
            let mut lookahead = String::new();
            lookahead.push(c);

            // 尝试读取 "ND" 部分
            if let Some(n) = chars.next() {
                lookahead.push(n);
                if let Some(d) = chars.next() {
                    lookahead.push(d);

                    // 检查是否是独立的 "AND" 词
                    // 前后应该是空格或字符串的开始/结束
                    let is_and = lookahead.to_lowercase() == "and" &&
                        (current_part.is_empty() || current_part.ends_with(' ')) &&
                        (chars.peek().is_none() || chars.peek() == Some(&' '));

                    if is_and {
                        // 找到一个 "AND"，处理当前部分并重置
                        if !current_part.trim().is_empty() {
                            result.push(convert_condition(&current_part.trim()));
                            current_part = String::new();
                        }

                        // 跳过 "AND" 后的空格
                        while let Some(&next) = chars.peek() {
                            if next == ' ' {
                                chars.next();
                            } else {
                                break;
                            }
                        }
                    } else {
                        // 不是 "AND"，添加到当前部分
                        current_part.push_str(&lookahead);
                    }
                } else {
                    current_part.push_str(&lookahead);
                }
            } else {
                current_part.push(c);
            }
        } else {
            // 普通字符，直接添加
            current_part.push(c);
        }
    }

    // 处理最后一部分
    if !current_part.trim().is_empty() {
        result.push(convert_condition(&current_part.trim()));
    }

    result.join(" AND ")
}

// 将单个条件转换为 json_extract 格式
fn convert_condition(condition: &str) -> String {
    // 支持常见操作符：=, !=, >, <, >=, <=
    let operators = ["=", "!=", ">", "<", ">=", "<="];

    for op in operators {
        if let Some(pos) = condition.find(op) {
            let key = condition[..pos].trim();
            let value = condition[pos + op.len()..].trim();

            return format!("json_extract(data, '$.{}') {} {}", key, op, value);
        }
    }

    // 如果没有找到操作符，返回原始条件
    condition.to_string()
}

fn check_action_valid(action: &str) -> Result<()> {
    let valid_actions = vec!["insert", "update", "query", "delete"];
    ensure!(
        valid_actions.contains(&action),
        "invalid action : {}",
        action
    );
    Ok(())
}
fn check_category_valid(category: &str) -> Result<()> {
    ensure!(
        Regex::new(r"^[a-z0-9-]{2,10}$")?.is_match(&category),
        "invalid `category` path : {} , not match with : {}",
        category,
        "^[a-z0-9-]{2,10}$"
    );
    Ok(())
}
fn check_set_param_valid(set_param: &str) -> Result<()> {
    let re = Regex::new(
        r"^([a-zA-Z_][a-zA-Z0-9_]*\s*=\s*[^,]+)(\s*,\s*[a-zA-Z_][a-zA-Z0-9_]*\s*=\s*[^,]+)*$",
    )?;
    ensure!(
        re.is_match(&set_param),
        "invalid `set` parameter : {} , shoule be eg. : {}",
        set_param,
        "set=|a=1,b=2|"
    );
    Ok(())
}

/// 将查询字符串解析为JSON，并自动转换类型
pub fn parse_query_with_types(str_params: HashMap<String, String>) -> Result<Value> {
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

async fn all_in_one_api_hex(
    s: S,
    Path((category, action, hex)): Path<(String, String, String)>,
) -> R<String> {
    check_category_valid(&category)?;
    check_action_valid(&action)?;
    promise!(
        Regex::new(r"^[a-f0-9A-F]+$")?.is_match(&hex),
        "invalid `hex` path : {} , no match with : {}",
        hex,
        "^[a-f0-9A-F]+$"
    );

    let bytes = hex::decode(&hex)?;
    let raw_query_str = String::from_utf8(bytes)?;
    let params = serde_json::from_str(&raw_query_str)?;

    Ok(handle_request(s, category, action, params).await?)
}
async fn all_in_one_api_post(
    s: S,
    Path((category, action)): Path<(String, String)>,
    body: String,
) -> R<String> {
    check_category_valid(&category)?;
    check_action_valid(&action)?;

    // 获取查询字符串
    let params = serde_json::from_str(&body)?;

    Ok(handle_request(s, category, action, params).await?)
}
async fn all_in_one_api(
    s: S,
    Path((category, action)): Path<(String, String)>,
    Query(mut params): Query<HashMap<String, String>>,
) -> R<String> {
    check_category_valid(&category)?;
    check_action_valid(&action)?;

    // 获取查询字符串
    let params = parse_query_with_types(params)?;

    Ok(handle_request(s, category, action, params).await?)
}

fn parse_set_string_to_hashmap(input: &str) -> HashMap<String, String> {
    let mut result = HashMap::new();

    // Trim the outer pipe characters and any whitespace
    let trimmed = input
        .trim()
        .trim_start_matches('|')
        .trim_end_matches('|')
        .trim();

    // Split by comma and process each key-value pair
    for pair in trimmed.split(',') {
        // Find the equals sign position
        if let Some(pos) = pair.find('=') {
            // Extract and trim the key and value
            let key = pair[..pos].trim().to_string();
            let value = pair[pos + 1..].trim().to_string();

            // Add to HashMap if key is not empty
            if !key.is_empty() {
                result.insert(key, value);
            }
        }
    }

    result
}

async fn handle_request(s: S, category: String, action: String, params: Value) -> Result<String> {
    let mut new_params = HashMap::new();
    new_params.insert(action.to_string(), params);

    let action_enum: ActionEnum = serde_json::from_value(serde_json::to_value(new_params)?)?;
    info!("action: {:?}", action_enum);

    match &action_enum {
        ActionEnum::INSERT(val) => {
            ensure!(val.len() != 0, "query params cant be empty!");
            ensure!(!val.contains_key("id"), "cant use system field `id` when insert data.");
            ensure!(!val.contains_key("cat"), "cant use system field `cat` when insert data.");
            ensure!(!val.contains_key("is_deleted"), "cant use system field `is_deleted` when insert data.");
            ensure!(!val.contains_key("created"), "cant use system field `created` when insert data.");
            ensure!(!val.contains_key("updated"), "cant use system field `updated` when insert data.");

            let obj = insert_data(s, Path(category), serde_json::to_string(val)?).await?;
            return Ok(serde_json::to_string(&obj.to_flat_map()?)?);
        }
        ActionEnum::UPDATE(UpdateParam { set, id: id }) => {
            check_set_param_valid(&set)?;

            let kv = parse_set_string_to_hashmap(set);
            let r = GeneralData::update_with_json_patch(&s.db, *id, parse_query_with_types(kv)?).await?;
            return Ok(r.rows_affected().to_string().to_string());
        }
        ActionEnum::QUERY(query_param) => {
            let select_fields =  if let Some(select)= &query_param.select{
                select
            }else {
                "*"
            };
            if let Some(id) = query_param.id {
                let r = GeneralData::query_by_id_with_cat_select(select_fields,id,&category, &s.db).await?;
                ensure!(r.len() == 1, "data not found for id : {}", id);

                if !query_param.less {
                    let new_map = r[0].to_flat_map()?;
                    return Ok(serde_json::to_string(&new_map)?);
                } else {
                    return Ok(r[0].data.to_string());
                }
            } else {
                //query list 
                
                let order_by =  if let Some(val)= &query_param.order_by{
                    val
                }else {
                    "id desc"
                };
                
                let _where =  if let Some(val)= &query_param._where{
                    &parse_and_convert_to_json_extract(val)
                }else {
                    "1=1"
                };
                
                if query_param.count{
                    let count = GeneralData::query_count_composite(&category,query_param.limit,_where,order_by, &s.db).await?;
                    return Ok(count.to_string());

                }else{
                    let list = GeneralData::query_composite(select_fields,&category,query_param.limit,_where,order_by, &s.db).await?;
                    if !query_param.less {
                        let mut new_arr = vec![];
                        for data in &list{
                            new_arr.push(data.to_flat_map()?);
                        }
                        return Ok(serde_json::to_string(&new_arr)?);
                    } else {
                        let mut new_arr = vec![];
                        for data in &list{
                            new_arr.push(data.extract_data()?);
                        }
                        return Ok(serde_json::to_string(&new_arr)?);
                    }
                }

            }
        }
        ActionEnum::DELETE(DeleteParam { id, hard_delete }) => {
            return if *hard_delete {
                let r = GeneralData::delete(*id, &s.db).await?;
                Ok(r.rows_affected().to_string().to_string())
            } else {
                let r = GeneralData::soft_delete(*id, &s.db).await?;
                Ok(r.rows_affected().to_string().to_string())
            }
        }
    }

    Ok(serde_json::to_string(&action_enum)?)
}

async fn insert_data(s: S, Path(cat): Path<String>, body: String) -> Result<GeneralData> {
    //validation
    // ensure!(!vec!["id", "data","get","update","delete","list", "query"].contains(&cat.as_str()), "please use another category name ! ");
    // check!(serde_json::from_str::<Value>(&body).is_ok());

    let ret = GeneralData::insert(&cat, &body.trim(), &s.db).await?;
    let id = ret.rows_affected();
    ensure!(id == 1, "insert failed!");

    let data = GeneralData::query_by_id(get_last_insert_id!(ret) as u32, &s.db).await?;
    ensure!(data.len() == 1, "data error! query_by_id not found.");

    let data = data[0].clone();
    after_update_data(&data).await?;

    Ok(data)
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
    pub is_deleted: bool,
    #[serde(serialize_with = "serialize_as_timestamp")]
    pub created: chrono::NaiveDateTime,
    #[serde(serialize_with = "serialize_as_timestamp")]
    pub updated: chrono::NaiveDateTime,
}
impl GeneralDataJson {
    pub fn from_data(data: &GeneralData) -> Result<Self> {
        Ok(Self {
            id: data.id,
            cat: data.cat.to_string(),
            data: serde_json::from_str(&data.data)?,
            is_deleted: data.is_deleted,
            created: data.created,
            updated: data.updated,
        })
    }
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
                    is_deleted: false,
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
) -> Result<Vec<QueryDataResp>> {
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
                    is_deleted: false,
                    created: d.created,
                    updated: d.updated,
                })
            })
            .collect();
        Ok(data)
    } else {
        let data = data.iter().map(|d| QueryDataResp::Raw(d.clone())).collect();
        Ok(data)
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

async fn after_update_data(data: &GeneralData) -> Result<()> {
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
    async fn test_insert_data() -> Result<()> {
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
    async fn test_query() -> Result<()> {
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
    async fn test_query_count() -> Result<()> {
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
