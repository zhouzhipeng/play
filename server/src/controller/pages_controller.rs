use std::collections::HashMap;

use axum::extract::{Path, Query};
use axum::Form;
use axum::response::Html;
use chrono::{TimeZone, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::{error, info};

use shared::timestamp_to_date_str;
use shared::tpl_engine_api::Template;

use crate::{ensure, hex_to_string, method_router, render_fragment, render_page, template};
use crate::{HTML, R, S};
use crate::controller::function_controller::{text_compare, TextCompareReq};
use crate::tables::change_log::ChangeLog;
use crate::tables::general_data::GeneralData;

method_router!(
    get : "/pages/*url"-> dynamic_pages,
    get : "/page-versions"-> page_versions,
);

#[derive(Deserialize)]
struct PageDto {
    title: String,
    url: String,
    content: String,
}

#[derive(Deserialize)]
struct QueryPageVersion {
    data_id: u32,
}

#[derive(Serialize)]
struct PageVersion {
    id: i64,
    data_before: String,
    data_after: String,
    output_html: String,
    updated: chrono::NaiveDateTime,
}


async fn page_versions(s: S, Query(q): Query<QueryPageVersion>) -> HTML {
    let rows = GeneralData::query_by_id(q.data_id, &s.db).await?;

    ensure!( rows.len()==1 && rows[0].cat=="pages", "invalid data id , not pages");

    let logs = ChangeLog::query(q.data_id, &s.db).await?;


    let mut handlers = vec![];

    for log in logs {
        let s_clone = s.clone();
        handlers.push(tokio::spawn(async move {
            let before =  if !log.data_before.is_empty(){hex_to_string!(serde_json::from_str::<PageDto>(&log.data_before).unwrap().content)} else{"".to_string()};
            let after =  if !log.data_after.is_empty(){hex_to_string!(serde_json::from_str::<PageDto>(&log.data_after).unwrap().content) }else{"".to_string()};
            let output_html = text_compare(s_clone, Form(TextCompareReq {
                text1: before.to_string(),
                text2: after.to_string(),
                with_ajax: 1,
                use_str_joiner: false,
                format_json: false,
            })).await.unwrap_or(Html("server error".to_string())).0;
            PageVersion {
                id: log.id,
                data_before: before,
                data_after: after,
                output_html,
                updated: log.created,
            }
        }));
    }
    let mut data = vec![];
    for h in handlers {
        data.push(h.await?);
    }


    template!(s, "change_log/page-versions.html", json!({ "items": data}))
}

async fn dynamic_pages(s: S, Path(url): Path<String>, Query(params): Query<HashMap<String, String>>) -> HTML {
    info!("dynamic_pages >> url is : {}", url);

    let data = GeneralData::query_by_json_field("*", "pages", "url", &format!("/{}", url), 1,&s.db).await?;
    ensure!(data.len()==1, "url not found.");
    // Your hex string


    let page_dto = serde_json::from_str::<PageDto>(&data[0].data)?;
    let raw_html = String::from_utf8(hex::decode(&page_dto.content)?)?;

    if url.ends_with(".html") {
        Ok(Html(raw_html))
    } else {
        //pass through query params.

        render_fragment(&s, Template::DynamicTemplate {
            name: page_dto.title.to_string(),
            content: raw_html,
        }, json!({
            "params": params
        })).await
    }
}
fn escape_html(input: &str) -> String {
    input.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")
}
#[cfg(test)]
mod tests {
    use std::fs;
    use difference::{Changeset, Difference};
    use crate::controller::pages_controller::escape_html;

    #[tokio::test]
    async fn test_text_compare() -> anyhow::Result<()> {
        // dynamic_pages(mock_state!(), Path("/a/b".to_string())).await;

        let text1 = fs::read_to_string("file1.txt").expect("Error reading file1.txt");
        let text2 = fs::read_to_string("file2.txt").expect("Error reading file2.txt");

        let changeset = Changeset::new(&text1, &text2, "\n");

        let mut html_output = String::new();
        html_output.push_str("<html><head><style>");
        html_output.push_str("body { font-family: monospace; }");
        html_output.push_str("table { width: 100%; table-layout: fixed; border-collapse: collapse; }");
        html_output.push_str("td { vertical-align: top; white-space: pre-wrap; word-wrap: break-word; padding: 5px; border: 1px solid #ddd; }");
        html_output.push_str(".lineno { color: #999; }");
        html_output.push_str(".add { background-color: #e6ffed; }");
        html_output.push_str(".rem { background-color: #ffeef0; }");
        html_output.push_str("</style></head><body>");
        html_output.push_str("<table><tr><th>Old</th><th>New</th></tr>");

        let mut line_no_old = 1;
        let mut line_no_new = 1;

        for diff in changeset.diffs {
            match diff {
                Difference::Same(ref x) => {
                    // Split lines and advance both line numbers correctly.
                    let count = x.split('\n').count();
                    line_no_old += count;
                    line_no_new += count;
                },
                Difference::Add(ref x) => {
                    for line in x.split('\n') {
                        html_output.push_str(&format!("<tr><td></td><td class='add'><span class='lineno'>{}</span> {}</td></tr>", line_no_new, escape_html(line)));
                        line_no_new += 1;
                    }
                },
                Difference::Rem(ref x) => {
                    for line in x.split('\n') {
                        html_output.push_str(&format!("<tr><td class='rem'><span class='lineno'>{}</span> {}</td><td></td></tr>", line_no_old, escape_html(line)));
                        line_no_old += 1;
                    }
                }
            }
        }

        html_output.push_str("</table></body></html>");

        fs::write("diff_output.html", html_output).expect("Unable to write file");
        Ok(())
    }
}