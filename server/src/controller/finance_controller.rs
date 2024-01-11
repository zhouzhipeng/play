use anyhow::Context;
use axum::http::header;
use chrono::TimeZone;
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::info;

use crate::{method_router, template};
use crate::{HTML, S};
//
method_router!(
    get : "/finance/dashboard"-> dashboard,
);

fn get_random(start: usize, end: usize)->usize{
    let mut rng = rand::thread_rng();
    let rand_range = rng.gen_range(start..end); // random number between 0 and 9
    rand_range
}



// #[axum::debug_handler]
async fn dashboard(s: S) -> HTML {
    // return_error!("test");

    //query currency rate in parallel
    let mut handles = vec![];
    for (source, target) in [("USD", "CNY"),("HKD", "CNY"),("USD", "HKD")]{
        handles.push(tokio::spawn(async {
            query_currency_rate(source, target).await
        }));
    }


    let mut total_rates:Vec<RateInfo> = vec![];
    for handle in handles{
        total_rates.extend(handle.await??);
    }


    //query stock prices
    // use  a random apikey
    let mut handles = vec![];
    for symbol in ["AAPL","NVDA"]{
        let copy_keys = s.config.finance.alphavantage_apikeys.clone();
        handles.push(tokio::spawn(async move{
            let rand_range =get_random(0, copy_keys.len());
            info!("random >> {}", rand_range);
            let price = query_stock_price(symbol, &copy_keys[rand_range]).await.unwrap_or(GlobalQuote::default());
            StockItem{
                symbol: symbol.to_string(),
                price,
            }
        }));
    }

    let mut stock_items = vec![];
    for handle in handles{
        stock_items.push(handle.await?);
    }



    let portfolio_items = &s.config.finance.portfolio_holdings;

    template!(s, "finance/dashboard.html", json!({
        "items":total_rates,
        "stock_items": stock_items,
        "portfolio_items": portfolio_items
    }))

}

#[derive(Serialize, Deserialize, Debug)]
struct RateInfo{
    rate: f64,
    source : String,
    target : String,
    time : String,
}
#[derive(Serialize, Deserialize, Debug)]
struct StockItem{
    symbol: String,
    price : GlobalQuote,
}
#[derive(Serialize, Deserialize, Debug)]
struct GlobalQuote{
    change_percent: String,
    price : String,
    previous_close : String,
    last_trading_day : String,
}

impl Default for GlobalQuote{
    fn default() -> Self {
        GlobalQuote{
            change_percent: "0%".to_string(),
            price: "0".to_string(),
            previous_close: "".to_string(),
            last_trading_day: "".to_string(),
        }
    }
}

async fn query_stock_price(symbol: &str, apikey: &str)->anyhow::Result<GlobalQuote>{
    #[cfg(feature = "debug")]
    let url = "https://www.alphavantage.co/query?function=GLOBAL_QUOTE&symbol=300135.SHZ&apikey=demo";
    #[cfg(not(feature = "debug"))]
    let url = format!("https://www.alphavantage.co/query?function=GLOBAL_QUOTE&symbol={}&apikey={}", symbol, apikey).to_string();

    let data: Value =reqwest::get(url).await?
        .json().await?;
    info!("data >> {:?}", data);
    if data.get("Information").is_some(){
        return Ok(GlobalQuote::default())
    }
    let price = data.get("Global Quote").context("key error")?.get("05. price").context("key error")?.as_str().context("key error")?.to_string();
    let change_percent = data.get("Global Quote").context("key error")?.get("10. change percent").context("key error")?.as_str().context("key error")?.to_string();
    let last_trading_day = data.get("Global Quote").context("key error")?.get("07. latest trading day").context("key error")?.as_str().context("key error")?.to_string();
    let previous_close = data.get("Global Quote").context("key error")?.get("08. previous close").context("key error")?.as_str().context("key error")?.to_string();

    Ok(GlobalQuote{
        change_percent,
        price,
        previous_close,
        last_trading_day,
    })
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
    #[tokio::test]
   async fn test_random()->anyhow::Result<()> {
        // use  a random apikey
        let mut rng = rand::thread_rng();
        let rand_range = rng.gen_range(0..10); // random number between 0 and 9
        println!("random > > {}", rand_range);
        Ok(())
    }

    #[tokio::test]
   async fn test_query_stock_price()->anyhow::Result<()> {

        let res = query_stock_price("AAPL", "demo").await?;
        println!("{:?}", res);

        Ok(())
    }
}