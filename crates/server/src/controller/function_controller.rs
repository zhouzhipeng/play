use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;

use anyhow::anyhow;
use axum::body::{HttpBody, StreamBody};
use axum::http::HeaderMap;
use axum::response::{Html, IntoResponse};
use axum::{Form, Json};
use bytes::Bytes;
use either::Either;
use futures_core::Stream;
use futures_util::{StreamExt, TryStreamExt};
use hex::ToHex;
use http::{HeaderName, HeaderValue};
use reqwest::ClientBuilder;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[cfg(feature = "use_mysql")]
use sqlx::mysql::{MySqlPoolOptions, MySqlQueryResult, MySqlRow};
use sqlx::Executor;
use sqlx::{Column, Row};

use crate::{ensure, method_router, template};
use crate::{render_fragment, Template, HTML, JSON, S};

method_router!(
    post : "/functions/str-joiner" -> str_joiner,
    post : "/functions/render-template" -> render_template,
    post : "/functions/py-runner" -> py_runner,
    post : "/functions/run-sql" -> run_sql,
    post : "/functions/run-http-request" -> run_http_request,
    post : "/functions/text-compare" -> text_compare,
);

#[derive(Deserialize)]
struct Data {
    s: String,
}
#[derive(Deserialize)]
struct RenderTemplateReq {
    raw_content: String,
    data: String,
}

async fn str_joiner(s: S, Form(data): Form<Data>) -> HTML {
    render_fragment(&s, Template::DynamicTemplate {
        name: "<string>".to_string(),
        content: data.s,
    }, json!({
        "params":{}
    })).await
}


async fn render_template(s: S, Form(data): Form<RenderTemplateReq>) -> HTML {
    render_fragment(&s, Template::DynamicTemplate {
        name: "<string>".to_string(),
        content: data.raw_content ,
    }, serde_json::from_str(&data.data)?).await
}

async fn py_runner(s: S, Form(data): Form<Data>) -> HTML {
    render_fragment(&s, Template::PythonCode {
        name: "<tmp_code>".to_string(),
        content: data.s,
    }, json!({})).await
}

#[derive(Deserialize)]
struct HttpRequestData {
    url: String,
    method: String,
    body: String,
    headers: String,
}

#[allow(non_snake_case)]
#[derive(Serialize)]
struct HttpResponseData {
    Body: String,
    Headers: HashMap<String, String>,
    Status: u16,
    StatusMsg: String,
}

async fn run_http_request(s: S, Form(data): Form<HttpRequestData>) -> JSON<HttpResponseData> {
    let client = ClientBuilder::new().timeout(Duration::from_secs(3)).build()?;

    let mut headers = HeaderMap::new();
    let json_data: Value = serde_json::from_str(&data.headers)?;
    if let Value::Object(map) = json_data {
        for (k, v) in map {
            if let Value::String(v) = v {
                let k =k.to_string();
                let header_value = HeaderValue::from_str(&v)?;
                headers.insert(HeaderName::from_str(&k)?, header_value);
            } else {
                // Skip non-string values or add error handling as needed
            }
        }
    }

    let response = match data.method.as_str() {
        "GET" => {
            client.get(&data.url).headers(headers).send().await?
        }
        "DELETE" => {
            client.delete(&data.url).headers(headers).send().await?
        }
        "POST" => {
            client.post(&data.url).headers(headers).body(data.body).send().await?
        }
        "PUT" => {
            client.put(&data.url).headers(headers).body(data.body).send().await?
        }
        "PATCH" => {
            client.patch(&data.url).headers(headers).body(data.body).send().await?
        }
        _ => return Err(anyhow!("Err >> method : {} not supported", data.method).into())
    };

    let mut headers = HashMap::new();
    for (k, v) in response.headers() {
        headers.insert(k.to_string(), v.to_str().unwrap_or("").to_string());
    }
    let status_code = response.status();
    let resp_body = response.text().await?;
    Ok(Json(HttpResponseData {
        Body: resp_body,
        Headers: headers,
        Status: status_code.as_u16(),
        StatusMsg: status_code.to_string(),
    }))
}


#[derive(Deserialize)]
struct RunSqlRequest {
    url: String,
    sql: String,
}
#[cfg(not(feature = "use_mysql"))]
async fn run_sql(s: S, Form(data): Form<RunSqlRequest>) -> HTML {
    Ok(Html("use_mysql feature is disabled.".to_string()))
}
#[cfg(feature = "use_mysql")]
async fn run_sql(s: S, Form(data): Form<RunSqlRequest>) -> HTML {

    let sql = &data.sql;
    let is_query =  sql_util::is_query_sql(sql);
    let data = query_mysql(&data.url.trim(), sql, is_query).await?;
    // println!("results >> {:?}", data);

    template!(s, "fragments/data-table.html", json!({
        "sql": sql,
        "items" : data
    }))
}

#[derive(Deserialize, Serialize)]
pub struct TextCompareReq {
    pub text1: String,
    pub text2: String,
    #[serde(default = "default_with_ajax")]
    pub with_ajax: i32,
    #[serde(default)]
    pub use_str_joiner: bool,
    #[serde(default)]
    pub format_json: bool,
}

#[allow(non_snake_case)]
#[derive(Deserialize)]
struct TextCompareRes {
    comparison: Option<String>,
    messageForUser: Option<String>,
}

fn default_with_ajax() -> i32 {
    1
}

pub async fn text_compare(s: S, Form(mut data): Form<TextCompareReq>) -> HTML {
    let client = ClientBuilder::new().timeout(Duration::from_secs(3)).build()?;

    if data.use_str_joiner{
        data.text1 = str_joiner(s.clone(), Form(Data { s: data.text1.to_string() })).await?.0;
        data.text2 = str_joiner(s, Form(Data { s: data.text2.to_string() })).await?.0;

    }


    if data.format_json{
        //format json
        if let Ok(val) = serde_json::from_str::<Value>(&data.text1) {
            if let Ok(val) = serde_json::to_string_pretty(&val) {
                data.text1 = val;
            }
        }
        if let Ok(val) = serde_json::from_str::<Value>(&data.text2) {
            if let Ok(val) = serde_json::to_string_pretty(&val) {
                data.text2 = val;
            }
        }
    }


    let resp = client.post("https://text-compare.com/").form(&data).send().await?;
    ensure!(resp.status().is_success(), "call https://text-compare.com/ failed.");
    // info!("resp >> {}", resp.text().await?);
    let res_body = resp.json::<TextCompareRes>().await?;
    Ok(Html(res_body.comparison.unwrap_or("<h2>No Diff!</h2>".to_string())))
    // Ok(Html("sfd".to_string()))
}

const CAT_OPENAI_THREAD: &str="openai_thread";

enum MyBodyStream<S>
    where
S: Stream<Item = Result<Bytes,  reqwest::Error>>,
{
    MyStreamBody(StreamBody<S>),
    MyBytes(&'static [u8]),
}



#[cfg(feature = "use_mysql")]
async fn query_mysql(url: &str, sql: &str, is_query: bool) -> anyhow::Result<Vec<Vec<String>>> {
    let db = MySqlPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_secs(5))
        .connect(url).await?;

    let mut conn = db.acquire().await?;
    let mut data = vec![];
    let mut column_handled = false;
    // conn.fetch_all()
    use futures_util::FutureExt;
    let results: Vec<Either<MySqlQueryResult, MySqlRow>> = conn.fetch_many(sql).try_collect().boxed().await?;
    for v in results {
        match v {
            Either::Left(r) => {
                if !column_handled && !is_query {
                    data.push(vec!["Rows Affected".to_string()]);
                    data.push(vec![r.rows_affected().to_string()]);
                }
            }
            Either::Right(row) => {
                if !column_handled {
                    column_handled = true;
                    let column_data = (&row.columns()).iter().map(|column| column.name().to_string()).collect::<Vec<String>>();
                    data.push(column_data);
                }
                let mut row_data = vec![];
                for i in 0..row.columns().len() {
                    let column = row.column(i);
                    let column_name = column.name();
                    // println!("name >> {}", column_name);
                    let val: String = row.try_get_unchecked(i).unwrap_or("NULL".to_string());
                    // println!("value >> {:?}", val);
                    row_data.push(val);
                }

                data.push(row_data);
            }
        }
    }


    Ok(data)
}


#[cfg(test)]
mod test {
    use super::*;
    use crate::mock_state;

    #[cfg(feature = "play-py-tpl")]
    #[tokio::test]
    async fn test_render_template() -> anyhow::Result<()> {

        let resp = render_template(mock_state!(), Form(RenderTemplateReq{ raw_content: "1 {{a}} b".to_string(), data: json!({
            "a":"zz"
        })})).await.unwrap();

        println!("resp >> {:?}", resp);

        Ok(())

    }
}