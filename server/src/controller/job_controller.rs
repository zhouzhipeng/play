use axum::response::Html;

use crate::{HTML, S};
use crate::method_router;

method_router!(
    get : "/job/test"-> root,
);

// #[axum::debug_handler]
async fn root(s: S) -> HTML {
    Ok(Html("job done".to_string()))
}



