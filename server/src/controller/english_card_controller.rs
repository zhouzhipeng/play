use serde_json::json;
use crate::{HTML, method_router, S, template};

method_router!(
    get : "/english_card/list"-> list,
);

async fn list(s: S) ->HTML{
    template!(s, "frame.html" + "english_card/list.html", json!({}))
}