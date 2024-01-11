use axum::http::header;
use chrono::TimeZone;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{method_router, template};
use crate::{HTML, S};

method_router!(
    get : "/finance/dashboard"-> dashboard,
);

// #[axum::debug_handler]
async fn dashboard(s: S) -> HTML {
    // return_error!("test");
    let mut r1 = query_currency_rate("USD", "CNY").await?;
    let r2 = query_currency_rate("HKD", "CNY").await?;
    let r3 = query_currency_rate("USD", "HKD").await?;
    r1.extend(r2);
    r1.extend(r3);

    template!(s, "finance/dashboard.html", json!({
        "items":r1
    }))

}

#[derive(Serialize, Deserialize, Debug)]
struct RateInfo{
    rate: f64,
    source : String,
    target : String,
    time : String,
}

async fn query_currency_rate(source: &str, target: &str) ->anyhow::Result<Vec<RateInfo>>{
    let mut headers = header::HeaderMap::new();
    headers.insert("authority", "api.wise.com".parse().unwrap());
    headers.insert("accept", "*/*".parse().unwrap());
    headers.insert("accept-language", "zh-CN,zh;q=0.9,en;q=0.8".parse().unwrap());
    headers.insert("authorization", "Basic OGNhN2FlMjUtOTNjNS00MmFlLThhYjQtMzlkZTFlOTQzZDEwOjliN2UzNmZkLWRjYjgtNDEwZS1hYzc3LTQ5NGRmYmEyZGJjZA==".parse().unwrap());
    headers.insert("cache-control", "no-cache".parse().unwrap());
    headers.insert("content-type", "application/json".parse().unwrap());
    headers.insert("origin", "https://wise.com".parse().unwrap());
    headers.insert("pragma", "no-cache".parse().unwrap());
    headers.insert("referer", "https://wise.com/".parse().unwrap());
    headers.insert("sec-ch-ua", "\"Not_A Brand\";v=\"8\", \"Chromium\";v=\"120\", \"Google Chrome\";v=\"120\"".parse().unwrap());
    headers.insert("sec-ch-ua-mobile", "?0".parse().unwrap());
    headers.insert("sec-ch-ua-platform", "\"macOS\"".parse().unwrap());
    headers.insert("sec-fetch-dest", "empty".parse().unwrap());
    headers.insert("sec-fetch-mode", "cors".parse().unwrap());
    headers.insert("sec-fetch-site", "same-site".parse().unwrap());
    headers.insert("user-agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".parse().unwrap());

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let res = client.get(format!("https://api.wise.com/v1/rates?source={}&target={}", source, target))
        .headers(headers)
        .send().await?
        .json().await?;
    Ok(res)
}



#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
   async fn test_query_rate()->anyhow::Result<()> {

        let res = query_currency_rate("USD", "CNY").await?;
        println!("{:?}", res);

        Ok(())
    }
}