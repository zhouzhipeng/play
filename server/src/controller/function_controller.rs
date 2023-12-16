use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use anyhow::anyhow;

use axum::{Form, Json, Router};
use axum::http::HeaderMap;
use axum::routing::post;
use reqwest::ClientBuilder;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::mysql::MySqlPoolOptions;
use sqlx::{Executor, MySqlPool};

use crate::{AppState, template};
use crate::controller::{HTML, JSON, render_fragment, S, Template};

pub fn init() -> Router<Arc<AppState>> {
    Router::new()
        .route("/functions/str-joiner", post(str_joiner))
        .route("/functions/py-runner", post(py_runner))
        .route("/functions/run-sql", post(run_sql))
        .route("/functions/run-http-request", post(run_http_request))
}

#[derive(Deserialize)]
struct Data {
    s: String,
}

async fn str_joiner(s: S, Form(data): Form<Data>) -> HTML {
    render_fragment(&s, Template::DynamicTemplate {
        name: "<string>".to_string(),
        content: data.s,
    }, json!({})).await
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

#[derive(Serialize)]
struct HttpResponseData {
    Body: String,
    Headers: HashMap<String, String>,
    Status: u16,
    StatusMsg: String,
}

async fn run_http_request(s: S, Form(data): Form<HttpRequestData>) -> JSON<HttpResponseData> {
    let client = ClientBuilder::new().timeout(Duration::from_secs(3)).build()?;

    let response = match data.method.as_str() {
        "GET" => {
            client.get(&data.url).send().await?
        }
        "DELETE" => {
            client.delete(&data.url).send().await?
        }
        "POST" => {
            let mut headers = HeaderMap::new();
            let json_data :Value =  serde_json::from_str(&data.headers)?;
            headers.insert("Content-Type", json_data["Content-Type"].as_str().unwrap_or("").parse()?);
            client.post(&data.url).headers(headers).body(data.body).send().await?
        }
        "PUT" => {
            client.put(&data.url).body(data.body).send().await?
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

async fn run_sql(s: S, Form(data): Form<RunSqlRequest>) -> HTML {
    let sql = data.sql.trim();
    let data = query_mysql(&data.url.trim(), sql).await?;
    println!("results >> {:?}", data);

    template!(s, "fragments/data-table.html", json!({
        "sql": sql,
        "items" : data
    }))
}

use sqlx::{Column, Row};
async  fn query_mysql(url: &str, sql: &str)->anyhow::Result<Vec<Vec<String>>>{

    let db = MySqlPoolOptions::new()
        .max_connections(1)
        .connect(url).await?;

    let mut conn = db.acquire().await?;
    let rows = conn.fetch_all(sql).await?;
    let mut  data = vec![];
    if !rows.is_empty(){
        let column_data = (&rows[0].columns()).iter().map(|column|column.name().to_string()).collect::<Vec<String>>();
        data.push(column_data);
    }

    for row in &rows {
        let mut row_data = vec![];

        for i in 0..row.columns().len() {
            let column = row.column(i);
            let column_name = column.name();
            // println!("name >> {}", column_name);
            let val:String = row.try_get_unchecked(i).unwrap_or("NULL".to_string());
            // println!("value >> {:?}", val);
            row_data.push(val);
        }

        data.push(row_data);
    }
    Ok(data)
}

#[cfg(test)]
mod tests {

    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[ignore]
    #[tokio::test]
    async fn test_all() -> anyhow::Result<()> {
        let url = "mysql://root:@localhost:3306/mysql";
        let sql = "select * from article";
        let data = query_mysql(url, sql).await?;
        println!("data >> {:?}", data);
        Ok(())
    }
}
