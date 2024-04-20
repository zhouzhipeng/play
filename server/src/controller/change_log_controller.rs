use serde_json::json;
use crate::{HTML, method_router, S, template};

method_router!(
    get : "/change-log/list"-> list,
);

async fn list(s: S) ->HTML{
    template!(s, "frame.html" + "change_log/list.html", json!({}))
}