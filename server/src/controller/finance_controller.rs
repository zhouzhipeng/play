use anyhow::Context;
use axum::http::header;
use chrono::TimeZone;
use rand::Rng;
use regex::Regex;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::info;

use crate::{method_router, template};
use crate::{HTML, S};
use crate::config::{PortfolioItemPosition, PortfolioMarket};
//
method_router!(
    get : "/finance/dashboard"-> dashboard,
);

fn get_random(start: usize, end: usize)->usize{
    let mut rng = rand::thread_rng();
    let rand_range = rng.gen_range(start..end); // random number between 0 and 9
    rand_range
}

fn get_random_api_key(keys: &Vec<String>)->String{
    let rand_range =get_random(0, keys.len());
    keys[rand_range].to_string()
}

// #[axum::debug_handler]
async fn dashboard(s: S) -> HTML {
    // return_error!("test");

    //query currency rate in parallel
    let mut handles = vec![];
    for rate in &s.config.finance.rate{
        let copy_rate = rate.clone();
        handles.push(tokio::spawn(async move {
            query_currency_rate(&copy_rate.source, &copy_rate.target).await
        }));
    }


    let mut total_rates:Vec<RateInfo> = vec![];
    for handle in handles{
        total_rates.extend(handle.await??);
    }

    //query stock market status
    let market_status = query_market_status( &get_random_api_key(&s.config.finance.alphavantage_apikeys)).await.unwrap_or_default();
    let default_market = MarketStatus::default();
    let us_stock_status = &market_status.iter().filter(|m|m.region=="United States").next().unwrap_or(&default_market).current_status;
    let hk_stock_status = &market_status.iter().filter(|m|m.region=="Hong Kong").next().unwrap_or(&default_market).current_status;


    let us_stock_symbols: Vec<String> = s.config.finance.portfolio.iter().filter(|p|p.market==PortfolioMarket::US_STOCK).map(|p|p.symbol.to_string()).collect();

    //query stock prices
    // use  a random apikey
    let mut handles = vec![];
    for symbol in us_stock_symbols{
        // let copy_keys = s.config.finance.alphavantage_apikeys.clone();
        handles.push(tokio::spawn(async move{
            // let rand_range =get_random(0, copy_keys.len());
            // info!("random >> {}", rand_range);
            let price = query_us_stock_price(&symbol, "").await.unwrap_or(GlobalQuote::default());
            StockItem{
                symbol: symbol.to_string(),
                market: PortfolioMarket::US_STOCK,
                price,
            }
        }));
    }

    //query hk stocks
    let hk_stock_symbols: Vec<String> = s.config.finance.portfolio.iter().filter(|p|p.market==PortfolioMarket::HK_STOCK).map(|p|p.symbol.to_string()).collect();
    let token = query_hk_page_token().await?;
    for symbol in hk_stock_symbols{
        let cp_token = token.clone();
        handles.push(tokio::spawn(async move{
            let price =     query_hk_stock(&symbol, &cp_token).await.map(|a|GlobalQuote{
                change_percent: format!("{}%", a.data.quote.pc),
                price: a.data.quote.bd.to_string(),
                previous_close: a.data.quote.hc.to_string(),
                last_trading_day: "".to_string(),
            }).unwrap_or(GlobalQuote::default());
            StockItem{
                symbol: symbol.to_string(),
                market: PortfolioMarket::HK_STOCK,
                price,
            }
        }));
    }

    //query crypto
    let crypto_symbols: Vec<String> = s.config.finance.portfolio.iter().filter(|p|p.market==PortfolioMarket::CRYPTO).map(|p|p.symbol.to_string()).collect();
    for symbol in crypto_symbols{
        handles.push(tokio::spawn(async move{
            let price = query_crypto_price(&symbol).await.map(|a|GlobalQuote{
                change_percent: format!("{}%", a.priceChangePercent),
                price: a.lastPrice.to_string(),
                previous_close: a.prevClosePrice.to_string(),
                last_trading_day: "".to_string(),
            }).unwrap_or(GlobalQuote::default());
            StockItem{
                symbol: symbol.to_string(),
                market: PortfolioMarket::CRYPTO,
                price,
            }
        }));
    }


    let mut stock_items = vec![];
    for handle in handles{
        stock_items.push(handle.await?);
    }


    let mut portfolio_items = s.config.finance.portfolio.clone();




    for item in portfolio_items.iter_mut() {

        if item.orders.len()>0{
            let mut qty = 0.;
            let mut avg_price = 0.;
            let mut total_value =0. ;

            let mut buy_list = vec![];
            let mut sell_list = vec![];
            let mut total_invest_in_orders = 0.;
            for (quantity, price , date ) in &item.orders {
                total_invest_in_orders = total_invest_in_orders + quantity * price;
                if *quantity >0.{
                    buy_list.push(PortfolioItemPosition{
                        quantity: *quantity,
                        price: *price,
                    })
                }else{
                    sell_list.push(PortfolioItemPosition{
                        quantity: -*quantity,
                        price: *price,
                    })
                }
            }

            item.total_invest=  total_invest_in_orders.max(0.);




            //calculate positions
            buy_list.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap());
            for sell_item in sell_list.iter_mut() {
                let mut i =0;
                let mut to_remove = vec![];
                for buy_item in buy_list.iter_mut() {
                    if sell_item.quantity < buy_item.quantity{
                        buy_item.quantity = buy_item.quantity - sell_item.quantity;
                        break;

                    }else if sell_item.quantity == buy_item.quantity{
                        //remove
                        to_remove.push(i);
                        break;
                    }else{
                        //remove
                        to_remove.push(i);
                        sell_item.quantity= sell_item.quantity  - buy_item.quantity;

                    }

                    i=i+1;
                }

                //remove items in buy list
                for i in to_remove {
                    buy_list.remove(i);
                }
            }

            item.positions = buy_list;

            //re-calculate avg price
            for buy_item in &item.positions {
                total_value  = total_value + buy_item.quantity * buy_item.price;
                qty = qty + buy_item.quantity;
            }
            avg_price = total_value / qty;

            item.quantity = qty;
            item.price = avg_price;
        }
    }

    template!(s, "finance/dashboard.html", json!({
        "items":total_rates,
        "stock_items": stock_items,
        "portfolio_items": portfolio_items,
        "us_stock_status": us_stock_status,
        "hk_stock_status": hk_stock_status,
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
struct YahooChartResp{
    chart : YahooChartRespChart
}
#[derive(Serialize, Deserialize, Debug)]
struct YahooChartRespChart{
    result : Vec<YahooChartRespChartResult>
}
#[derive(Serialize, Deserialize, Debug)]
struct YahooChartRespChartResult{
    meta : YahooChartRespChartResultMeta
}
#[derive(Serialize, Deserialize, Debug)]
struct YahooChartRespChartResultMeta{
    regularMarketPrice : f64,
    previousClose : f64,
}
#[derive(Serialize, Deserialize, Debug)]
struct StockItem{
    symbol: String,
    market: PortfolioMarket,
    price : GlobalQuote,
}
#[derive(Serialize, Deserialize, Debug)]
struct GlobalQuote{
    change_percent: String,
    price : String,
    previous_close : String,
    last_trading_day : String,
}
#[derive(Serialize, Deserialize, Debug, Default)]
struct MarketStatus{
    market_type: String,
    region : String,
    primary_exchanges : String,
    current_status : String,
}
#[derive(Serialize, Deserialize, Debug)]
struct FullMarketStatus{
    endpoint: String,
    markets : Vec<MarketStatus>,
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
async fn query_us_stock_price(symbol: &str, apikey: &str)->anyhow::Result<GlobalQuote>{
    let mut headers = header::HeaderMap::new();
    headers.insert("authority", "query1.finance.yahoo.com".parse().unwrap());
    headers.insert("accept", "*/*".parse().unwrap());
    headers.insert("accept-language", "zh-CN,zh;q=0.9,en;q=0.8".parse().unwrap());
    headers.insert("cache-control", "no-cache".parse().unwrap());
    headers.insert("origin", "https://finance.yahoo.com".parse().unwrap());
    headers.insert("pragma", "no-cache".parse().unwrap());
    headers.insert("referer", "https://finance.yahoo.com/quote/YANG?p=YANG&.tsrc=fin-srch".parse().unwrap());
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
    let res:YahooChartResp = client.get(format!("https://query1.finance.yahoo.com/v8/finance/chart/{}?region=US&lang=en-US&includePrePost=false&interval=2m&useYfid=true&range=1d&corsDomain=finance.yahoo.com&.tsrc=finance", symbol))
        .headers(headers)
        .send().await?
        .json().await?;
    println!("{:?}", res);
    let data = &res.chart.result[0].meta;
    Ok(GlobalQuote{
        change_percent: format!("{:.2}%", (data.regularMarketPrice/data.previousClose-1.0)*100.0),
        price: data.regularMarketPrice.to_string(),
        previous_close: data.previousClose.to_string(),
        last_trading_day: "".to_string(),
    })
}

async fn query_market_status(apikey: &str)->anyhow::Result<Vec<MarketStatus>>{

    let url = format!("https://www.alphavantage.co/query?function=MARKET_STATUS&apikey={}", apikey).to_string();

    let data:FullMarketStatus =reqwest::get(url).await?
        .json().await?;
    Ok(data.markets)
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


#[derive(Serialize, Deserialize, Debug)]
struct HKStockQuote{
    data: HKStockQuoteData,
}
#[derive(Serialize, Deserialize, Debug)]
struct HKStockQuoteData{
    quote: HKStockQuoteDataInner,
}
#[derive(Serialize, Deserialize, Debug)]
struct HKStockQuoteDataInner{
    //price
    bd: String,
    //previous close
    hc: String,
    //change percent
    pc: String,
    //trading status
    trdstatus: String,
}

async fn query_hk_page_token() -> anyhow::Result<String>{

    //get token from page source
    let page_html = reqwest::get("https://www.hkex.com.hk/Market-Data/Securities-Prices/Equities/Equities-Quote?sym=2477&sc_lang=en").await?.text().await?;
    // info!("page_html >> {}", page_html);
    // let document = Html::parse_document(&page_html);
    // let selector = Selector::parse("h1.foo").unwrap();
    let re = Regex::new(r#""ev.{62,100}""#).unwrap();
    // let token = re.captures(&page_html).unwrap().get(0).unwrap().as_str();
    let token = re.find(&page_html).unwrap().as_str().replace(r#"""#, "");
    Ok(token)
}
async fn query_hk_stock(symbol: &str, token: &str) -> anyhow::Result<HKStockQuote>{
    // 使用 reqwest 发送 HTTP GET 请求
    let resp = reqwest::get(format!("https://www1.hkex.com.hk/hkexwidget/data/getequityquote?sym={}&token={}&lang=eng&qid=1705105507584&callback=_&_=1705105505592", symbol,token)).await?.text().await?;
    let json_str = &resp[2..resp.len()-1];
    info!("query_hk_stock  resp >> {:?}", json_str);

    Ok(serde_json::from_str::<HKStockQuote>(json_str)?)
}


#[derive(Serialize, Deserialize, Debug)]
struct CryptoQuote{
    symbol: String,
    lastPrice: String,
    priceChange: String,
    priceChangePercent: String,
    prevClosePrice: String,
}
async fn query_crypto_price(symbol: &str) -> anyhow::Result<CryptoQuote>{
    // 使用 reqwest 发送 HTTP GET 请求
    let ret = reqwest::get(format!("https://api.binance.com/api/v3/ticker/24hr?symbol={}", symbol)).await?.json().await?;

    Ok(ret)
}




#[cfg(test)]
mod test {
    use super::*;

    #[ignore]
    #[tokio::test]
   async fn test_query_crypto_price()->anyhow::Result<()> {

        let res = query_crypto_price("BTCUSDT").await?;
        println!("{:?}", res);

        Ok(())
    }

    #[ignore]
    #[tokio::test]
   async fn test_query_rate()->anyhow::Result<()> {

        let res = query_currency_rate("USD", "CNY").await?;
        println!("{:?}", res);

        Ok(())
    }

    #[ignore]
    #[tokio::test]
   async fn test_random()->anyhow::Result<()> {
        // use  a random apikey
        let mut rng = rand::thread_rng();
        let rand_range = rng.gen_range(0..10); // random number between 0 and 9
        println!("random > > {}", rand_range);
        Ok(())
    }


    #[ignore]
    #[tokio::test]
   async fn test_query_stock_price()->anyhow::Result<()> {

        let res = query_stock_price("AAPL", "demo").await?;
        println!("{:?}", res);



        Ok(())
    }


    #[ignore]
    #[tokio::test]
   async fn test_query_hk_stock()->anyhow::Result<()> {

        let token = query_hk_page_token().await?;
        let res = query_hk_stock("2477",&token).await?;
        println!("{:?}", res);

        Ok(())
    }
    #[ignore]
    #[tokio::test]
   async fn test_regex()->anyhow::Result<()> {
        let ss = r#"
                 return "evLtsLsBNAUVTPxtGqVeG2H85NZFkD5sb7jQnB3RkH6AxuYpvbi%2bnu7vN1y0XAln";
        "#;

        let re = Regex::new(r#""ev.{62,64}""#).unwrap();
        let token = re.find(&ss).unwrap().as_str().replace(r#"""#, "");
        println!("token  = {}", token);

        Ok(())
    }
    #[ignore]
    #[tokio::test]
   async fn test_query_us_stock()->anyhow::Result<()> {
        let data = query_us_stock_price("AAPL","").await?;
        println!("{:?}", data);
        Ok(())
    }
}