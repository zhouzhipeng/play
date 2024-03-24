use axum::extract::Path;
use axum::response::Html;
use chrono::{TimeZone, Utc};
use serde::Deserialize;
use serde_json::json;
use tracing::{error, info};

use shared::timestamp_to_date_str;
use shared::tpl_engine_api::Template;

use crate::{check, hex_to_string, method_router, render_fragment, template};
use crate::{HTML, R, S};
use crate::tables::general_data::GeneralData;

method_router!(
    get : "/pages/*url"-> dynamic_pages,
);

#[derive(Deserialize)]
struct PageDto{
    title: String,
    url: String,
    content: String
}



async fn dynamic_pages(s: S, Path(url): Path<String>) -> HTML {
    info!("dynamic_pages >> url is : {}", url);

    let data = GeneralData::query_json("*", "pages", "url", &format!("/{}", url), &s.db).await?;
    check!(data.len()==1, "url not found.");
    // Your hex string


    let page_dto = serde_json::from_str::<PageDto>(&data[0].data)?;
    let raw_html =  String::from_utf8(hex::decode(&page_dto.content)?)?;

    if url.ends_with(".html"){
        Ok(Html(raw_html))
    }else{
        render_fragment(&s, Template::DynamicTemplate {
            name: "<dynamic_pages>".to_string(),
            content: raw_html,
        }, json!({})).await
    }

}

#[cfg(test)]
mod tests {
    use crate::mock_state;

    use super::*;

    #[tokio::test]
    async fn test_dynamic_pages() -> anyhow::Result<()> {
        dynamic_pages(mock_state!(), Path("/a/b".to_string())).await;

        Ok(())
    }
}