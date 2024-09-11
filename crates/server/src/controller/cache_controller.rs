use axum::body::HttpBody;
use axum::response::Html;
use sqlx::Row;

use crate::{method_router, HTML};

method_router!(
    get : "/cache/test" -> list_files,
);


async fn list_files() -> HTML {
    #[cfg(feature = "play_cache")]
    {
        let html  = play_cache::render_html_in_browser("http://example.com/index.html").await?;
        Ok(Html(html))
    }
    #[cfg(not(feature = "play_cache"))]
    {
        Ok(Html("play_cache is disabled.".to_owned()))
    }

}
