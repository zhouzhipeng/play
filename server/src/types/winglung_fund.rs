use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct WingLungFundRes {
    pub msg: String,
    pub data: Data,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Data {
    pub cumulative_perfor: CumulativePerfor,
    pub offer_period_end_date: String,
    pub namecn: String,
    pub manage_org_name_en: String,
    pub manage_org_name_cn: String,
    pub fund_cat: String,
    #[serde(rename = "sharperatio1yr")]
    pub sharperatio1_yr: String,
    pub view_profit: String,
    pub assets: String,
    pub price_value_list: Vec<PriceValueList>,
    pub inception_date: String,
    #[serde(rename = "maxdrawdown1yr")]
    pub maxdrawdown1_yr: String,
    #[serde(rename = "FMPFlag")]
    pub fmp_flag: String,
    pub rtn_value_trip_list: Vec<RtnValueTripList>,
    pub currency: String,
    pub id: String,
    pub day_end_date: String,
    pub day_end_nav: String,
    pub expiration_date: String,
    pub price_his_value_list: Vec<PriceHisValueList>,
    pub risk_attributes: String,
    pub maximum_discount_rate: String,
    pub subscribe_now: String,
    pub day_trip: String,
    #[serde(rename = "stddev1yr")]
    pub stddev1_yr: String,
    pub saving_plan: String,
    #[serde(rename = "return1y")]
    pub return1_y: String,
    pub category_code: String,
    pub closed_end_fund: String,
    pub wlb_fund_code1: String,
    pub wlb_fund_code2: String,
    pub is_financial: String,
    pub redemption_rate: String,
    pub dividend_way2: String,
    pub document_list: Vec<DocumentList>,
    pub rtn_value_list: Vec<RtnValueList>,
    pub dividend_way1: String,
    pub name: String,
    pub subscribe_rate: String,
    pub manager_list: Vec<ManagerList>,
    pub namefon: String,
    pub offer_period_start_date: String,
    pub position: Vec<Position>,
    pub fund_status: String,
    pub switching_fee: String,
    pub class_type: String,
    pub confirmed_share: String,
}



#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CumulativePerfor {
    pub return6_mth_rank: String,
    pub return1_week: String,
    pub return1_mth: String,
    pub return1_week_rank: String,
    pub return_ytd_rank: String,
    pub return1_yr: String,
    pub return3_mth: String,
    pub return_ytd: String,
    pub return1_mth_rank: String,
    pub return3_mth_rank: String,
    pub return6_mth: String,
    pub return1_yr_rank: String,
}


#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentList {
    pub document_date: String,
    #[serde(rename = "type")]
    pub document_list_type: String,
    pub inner_url: String,
    pub public_url: String,
    pub language: String,
    pub market: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagerList {
    pub start_date: String,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Position {
    pub detail_name: String,
    pub weighting: String,
    pub market_value: String,
    pub portfolio_date: String,
    pub exchange_id: String,
    pub ticker: String,
    pub fund_holding_type_class: String,
}



#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PriceHisValueList {
    pub date: String,
    pub cumulative_value: String,
    pub day_trip: String,
    pub unit_value: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PriceValueList {
    pub date: String,
    pub unit_value: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RtnValueList {
    pub rtn_his_date: String,
    pub rtn_his_value: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RtnValueTripList {
    pub date: String,
    pub is_dividend_point: String,
    pub fund_value: String,
    pub is_buy_point: Option<String>,
    pub is_sale_point: String,
    pub other_value: String,
    pub cat_return_value: String,
}

