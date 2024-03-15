use serde_json::json;
use crate::{HTML, method_router, S, template};

method_router!(
    get : "/general-data-meta/list"-> list,
);

async fn list(s: S) ->HTML{
    template!(s, "frame.html" + "general_data_meta/list.html", json!({}))
}