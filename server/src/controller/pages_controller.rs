use std::collections::HashMap;
use axum::extract::{Path, Query};
use axum::response::Html;
use chrono::{TimeZone, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use tracing::{error, info};

use shared::timestamp_to_date_str;
use shared::tpl_engine_api::Template;

use crate::{ensure, hex_to_string, method_router, render_fragment, render_page, template};
use crate::{HTML, R, S};
use crate::tables::change_log::ChangeLog;
use crate::tables::general_data::GeneralData;

method_router!(
    get : "/pages/*url"-> dynamic_pages,
    get : "/page-versions"-> page_versions,
);

#[derive(Deserialize)]
struct PageDto{
    title: String,
    url: String,
    content: String
}
#[derive(Deserialize)]
struct QueryPageVersion{
    data_id: u32,
}



async fn page_versions(s: S, Query(q): Query<QueryPageVersion>) -> HTML {
    let rows = GeneralData::query_by_id(q.data_id, &s.db).await?;

    ensure!( rows.len()==1 && rows[0].cat=="pages", "invalid data id , not pages");

    let logs = ChangeLog::query(q.data_id, &s.db).await?;

    let data = logs.iter().map(|log| {
        let before = hex_to_string!(serde_json::from_str::<PageDto>(&log.data_before).unwrap().content);
        let after = hex_to_string!(serde_json::from_str::<PageDto>(&log.data_after).unwrap().content);
        json!({
            "id": log.id,
            "before": before,
            "after": after,
            "updated": log.created
        })
    }).collect::<Vec<Value>>();

    template!(s, "change_log/page-versions.html", json!({ "items": data}))


}

async fn dynamic_pages(s: S, Path(url): Path<String>,Query(params): Query<HashMap<String, String>>) -> HTML {
    info!("dynamic_pages >> url is : {}", url);

    let data = GeneralData::query_by_json_field("*", "pages", "url", &format!("/{}", url), &s.db).await?;
    ensure!(data.len()==1, "url not found.");
    // Your hex string


    let page_dto = serde_json::from_str::<PageDto>(&data[0].data)?;
    let raw_html =  String::from_utf8(hex::decode(&page_dto.content)?)?;

    if url.ends_with(".html"){
        Ok(Html(raw_html))
    }else{
        //pass through query params.

        render_fragment(&s, Template::DynamicTemplate {
            name: page_dto.title.to_string(),
            content: raw_html,
        }, json!({
            "params": params
        })).await
    }

}

#[cfg(test)]
mod tests {
    use crate::mock_state;

    use super::*;

    #[tokio::test]
    async fn test_dynamic_pages() -> anyhow::Result<()> {
        // dynamic_pages(mock_state!(), Path("/a/b".to_string())).await;

        Ok(())
    }
}