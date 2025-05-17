use crate::tables::general_data::GeneralData;
use crate::{
    get_last_insert_id, method_router, promise
    , R, S,
};
use anyhow::{ensure, Context, Result};
use axum::extract::{Path, Query};
use axum::Json;

use regex::Regex;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::num::NonZeroU32;

method_router!(
    get : "/api/v4/data/:category/get"-> handle_get,
    get : "/api/v4/data/:category/query"-> handle_query,
    get : "/api/v4/data/:category/count"-> handle_count,
    post : "/api/v4/data/:category/delete"-> handle_delete,
    post : "/api/v4/data/:category/insert"-> handle_insert,
    post : "/api/v4/data/:category/update"-> handle_update,
);

#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
struct GetParam {

    id: Option<u32>,
    select: Option<String>,
    #[serde(default)]
    slim: bool,
}
#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
struct QueryParam {
    select: Option<String>,
    #[serde(default = "default_limit", deserialize_with = "deserialize_limit")]
    limit: LimitParam,
    #[serde(rename = "where")]
    _where: Option<String>,
    order_by: Option<String>,
    #[serde(default)]
    slim: bool,
    #[serde(default)]
    include_deleted: bool,
}
#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
struct InsertOptionParam {
    #[serde(default)]
    unique: bool,
}
#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
struct CountParam {
    #[serde(rename = "where")]
    _where: Option<String>,
    #[serde(default)]
    include_deleted: bool,
}

#[derive(Serialize, Debug)]
struct LimitParam((u32, NonZeroU32));

impl LimitParam {
    pub fn to_string(&self) -> String {
        format!("{},{}", self.0 .0, self.0 .1)
    }
}

fn deserialize_limit<'de, D>(deserializer: D) -> Result<LimitParam, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;

    match value {
        serde_json::Value::String(s) => {
            // return Err(D::Error::custom("invalid `limit` parameter,should have only two numbers. "))
            let values: Vec<&str> = s.split(',').collect();
            if values.len() != 2 {
                return Err(D::Error::custom(
                    "invalid `limit` parameter,should have only two numbers. ",
                ));
            }

            let num1 = values[0].parse::<u32>().map_err(|e| {
                D::Error::custom("invalid `limit` parameter, should be positive integers ")
            })?;
            let num2 = values[1].parse::<NonZeroU32>().map_err(|e| {
                D::Error::custom("invalid `limit` parameter, should be positive integers ")
            })?;

            Ok(LimitParam((num1, num2)))
        }
        serde_json::Value::Number(n) => {
            if let Some(num) = n.as_u64() {
                if let Some(num) = NonZeroU32::new(num as u32) {
                    Ok(LimitParam((0, num)))
                } else {
                    return Err(D::Error::custom(
                        "invalid `limit` parameter,should be positive integers. ",
                    ));
                }
            } else {
                return Err(D::Error::custom(
                    "invalid `limit` parameter,should be positive integers. ",
                ));
            }
        }
        _ => return Err(D::Error::custom("invalid `limit` parameter. ")),
    }
}

fn default_limit() -> LimitParam {
    LimitParam((0, NonZeroU32::new(10).unwrap()))
}

const SYSTEM_FIELDS: [&str; 6] = ["id", "cat", "data", "is_deleted", "created", "updated"];

#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
struct UpdateOptionParam {
    id: u32,

    /// override full data , not just a field.
    #[serde(default)]
    override_data : bool,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
struct DeleteParam {
    id: Option<u32>,

    #[serde(default)]
    delete_all: bool,

    #[serde(default)]
    hard_delete: bool,
}
#[derive(Serialize, Deserialize, Debug)]
struct AffectedResp {
    affected_rows: u64,
}
#[derive(Serialize, Deserialize, Debug)]
struct CountResp {
    rows: u32,
}

fn parse_and_convert_to_json_extract(condition_str: &str) -> String {
    // 存储条件和连接操作符
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

        // 检查是否遇到 "AND" 或 "OR"（忽略大小写）
        if (c == 'a' || c == 'A' || c == 'o' || c == 'O') &&
            ((c == 'a' || c == 'A') && (chars.peek() == Some(&'n') || chars.peek() == Some(&'N')) ||
                (c == 'o' || c == 'O') && (chars.peek() == Some(&'r') || chars.peek() == Some(&'R'))) {

            let mut lookahead = String::new();
            lookahead.push(c);

            let is_operator = if c == 'a' || c == 'A' {
                // 尝试读取 "ND" 部分
                if let Some(n) = chars.next() {
                    lookahead.push(n);
                    if let Some(d) = chars.next() {
                        lookahead.push(d);

                        // 检查是否是独立的 "AND" 词
                        lookahead.to_lowercase() == "and"
                            && (current_part.is_empty() || current_part.ends_with(' '))
                            && (chars.peek().is_none() || chars.peek() == Some(&' '))
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else { // c == 'o' || c == 'O'
                // 尝试读取 "R" 部分
                if let Some(r) = chars.next() {
                    lookahead.push(r);

                    // 检查是否是独立的 "OR" 词
                    lookahead.to_lowercase() == "or"
                        && (current_part.is_empty() || current_part.ends_with(' '))
                        && (chars.peek().is_none() || chars.peek() == Some(&' '))
                } else {
                    false
                }
            };

            if is_operator {
                // 找到一个操作符，处理当前部分并记录操作符
                if !current_part.trim().is_empty() {
                    result.push((convert_condition(&current_part.trim()), lookahead.to_lowercase()));
                    current_part = String::new();
                }

                // 跳过操作符后的空格
                while let Some(&next) = chars.peek() {
                    if next == ' ' {
                        chars.next();
                    } else {
                        break;
                    }
                }
            } else {
                // 不是操作符，添加到当前部分
                current_part.push_str(&lookahead);
            }
        } else {
            // 普通字符，直接添加
            current_part.push(c);
        }
    }

    // 处理最后一部分
    if !current_part.trim().is_empty() {
        // 最后一个条件后面没有操作符，使用默认的空字符串
        result.push((convert_condition(&current_part.trim()), String::new()));
    }

    // 构建结果字符串，正确应用每个操作符
    if result.is_empty() {
        return String::new();
    }

    let mut final_result = result[0].0.clone();
    for i in 0..(result.len() - 1) {
        let operator = &result[i].1;
        let next_condition = &result[i+1].0;

        if operator == "and" {
            final_result = format!("{} AND {}", final_result, next_condition);
        } else if operator == "or" {
            final_result = format!("{} OR {}", final_result, next_condition);
        }
    }

    final_result
}

// 将单个条件转换为 json_extract 格式
fn convert_condition(condition: &str) -> String {
    // 支持常见操作符：=, !=, >, <, >=, <=
    let operators = ["=", "!=", ">", "<", ">=", "<=", "like"];

    for op in operators {
        if let Some(pos) = condition.to_lowercase().find(op) {
            let key = condition[..pos].trim();
            let value = condition[pos + op.len()..].trim();

            if SYSTEM_FIELDS.contains(&key) {
                return format!("{} {} {}", key, op, value);
            } else {
                return format!("json_extract(data, '$.{}') {} {}", key, op, value);
            }
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
        Regex::new(r"^[a-zA-Z0-9-_]{2,20}$")?.is_match(&category),
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
        "set=a=1,b=2"
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

async fn handle_insert(
    s: S,
    Path((category)): Path<(String)>,
    Query(option) : Query<InsertOptionParam>,
    Json(val): Json<HashMap<String, Value>>,
) -> R<Json<Map<String, Value>>> {
    check_category_valid(&category)?;

    promise!(val.len() != 0, "query params cant be empty!");

    for field in SYSTEM_FIELDS {
        promise!(
            !val.contains_key(field),
            format!("cant use system field `{field}` when insert data.")
        );
    }


    if option.unique{
        //should be only one record for this category.
        let count = GeneralData::query_count(&category, &s.db).await?;
        promise!(count <=1, "`unique`  is true , but there is more than one ({}) row under category :{}.", count, category);

        if count==0{
            //insert record
            let obj = insert_data(s, Path(category), serde_json::to_string(&val)?).await?;
            Ok(Json(obj.to_flat_map()?))
        }else{
            //update record
            GeneralData::update_data_by_cat(&category, &serde_json::to_string(&val)? ,&s.db).await?;
            let records = GeneralData::query_by_cat_simple(&category, 1,&s.db).await?;
            promise!(records.len() ==1 , "{category} not found , expected one record under it!");

            Ok(Json(records[0].to_flat_map()?))
        }

    }else{
        //just insert
        let obj = insert_data(s, Path(category), serde_json::to_string(&val)?).await?;
        Ok(Json(obj.to_flat_map()?))
    }



}
async fn handle_update(
    s: S,
    Path((category)): Path<(String)>,
    Query(option) : Query<UpdateOptionParam>,
    Json(val): Json<HashMap<String, Value>>,
) -> R<Json<AffectedResp>> {
    check_category_valid(&category)?;

    let r = if !option.override_data{
        GeneralData::update_with_json_patch(&s.db, option.id, serde_json::to_string(&val)?).await?
    }else{
        GeneralData::update_data_by_id(option.id, &serde_json::to_string(&val)?, &s.db).await?
    };

    Ok(Json(AffectedResp {
        affected_rows: r.rows_affected(),
    }))
}

async fn handle_delete(
    s: S,
    Path((category)): Path<(String)>,
    Query(DeleteParam {
        id,
        delete_all,
        hard_delete,
    }): Query<DeleteParam>,
) -> R<Json<AffectedResp>> {
    check_category_valid(&category)?;

    let affected_rows = if delete_all {
        if hard_delete {
            let r = GeneralData::delete_by_cat(&category, &s.db).await?;
            r.rows_affected()
        } else {
            let r = GeneralData::soft_delete_by_cat(&category, &s.db).await?;
            r.rows_affected()
        }
    } else {
        promise!(id.is_some(), "id/delete_all cant be empty!");
        let id = id.unwrap();
        if hard_delete {
            let r = GeneralData::delete(id, &s.db).await?;
            r.rows_affected()
        } else {
            let r = GeneralData::soft_delete(id, &s.db).await?;
            r.rows_affected()
        }
    };

    Ok(Json(AffectedResp { affected_rows }))
}
async fn handle_get(
    s: S,
    Path((category)): Path<(String)>,
    Query(query_param): Query<GetParam>,
) -> R<Json<Map<String, Value>>> {
    check_category_valid(&category)?;

    let select_fields = if let Some(select) = &query_param.select {
        select
    } else {
        "*"
    };


    let r = if let Some(id) = query_param.id{
        let r =
            GeneralData::query_by_id_with_cat_select(select_fields, id, &category, &s.db)
                .await?;
        promise!(r.len() == 1, "data not found for id : {}", id);
        r

    }else{
        let records = GeneralData::query_by_cat_simple(&category, 2,&s.db).await?;
        promise!(records.len() ==1 , "when `get` without id , there must have only one record under `{category}` , but found more or less!");
        records
    };

    if !query_param.slim {
        let new_map = r[0].to_flat_map()?;
        Ok(Json(new_map))
    } else {
        Ok(Json(
            serde_json::from_str::<Value>(&r[0].data)?
                .as_object()
                .context("not json obj")?
                .clone(),
        ))
    }
}
async fn handle_query(
    s: S,
    Path((category)): Path<(String)>,
    Query(query_param): Query<QueryParam>,
) -> R<Json<Vec<Value>>> {
    check_category_valid(&category)?;

    let select_fields = if let Some(select) = &query_param.select {
        select
    } else {
        "*"
    };

    //query list

    let order_by = if let Some(val) = &query_param.order_by {
        val
    } else {
        "id desc"
    };

    let _where = if let Some(val) = &query_param._where {
        &parse_and_convert_to_json_extract(val)
    } else {
        "1=1"
    };

    let list = GeneralData::query_composite(
        select_fields,
        &category,
        &query_param.limit.to_string(),
        _where,
        query_param.include_deleted,
        order_by,
        &s.db,
    )
    .await?;
    if !query_param.slim {
        let mut new_arr = vec![];
        for data in &list {
            new_arr.push(data.to_flat_map()?);
        }
        Ok(Json(
            serde_json::to_value(&new_arr)?
                .as_array()
                .context("not map")?
                .clone(),
        ))
    } else {
        let mut new_arr = vec![];
        for data in &list {
            new_arr.push(data.extract_data()?);
        }
        Ok(Json(
            serde_json::to_value(&new_arr)?
                .as_array()
                .context("not map")?
                .clone(),
        ))
    }
}
async fn handle_count(
    s: S,
    Path((category)): Path<(String)>,
    Query(query_param): Query<CountParam>,
) -> R<Json<CountResp>> {
    check_category_valid(&category)?;

    let _where = if let Some(val) = &query_param._where {
        &parse_and_convert_to_json_extract(val)
    } else {
        "1=1"
    };

    let count =
        GeneralData::query_count_composite(&category, _where, query_param.include_deleted, &s.db)
            .await?;
    return Ok(Json(CountResp{ rows: count }));
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

    Ok(data)
}

