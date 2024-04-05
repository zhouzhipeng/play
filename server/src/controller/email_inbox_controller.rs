use axum::response::Html;
use serde_json::json;
use crate::{ensure, HTML, method_router, S, template};
use crate::tables::email_inbox::EmailInbox;

method_router!(
    get : "/email"-> list,
    get : "/email-inbox/delete-all"-> delete_all,
);

async fn list(s: S) ->HTML{
    let items = EmailInbox::query_all(&s.db).await?;
    let count = EmailInbox::count(&s.db).await?;
    template!(s, "email_inbox/list.html", json!({
        "items": items,
        "count": count
    }))
}
async fn delete_all(s: S) ->HTML{
    let r = EmailInbox::delete_all(&s.db).await?;

    Ok(Html(format!("delete count : {}", r.rows_affected())))
}