use axum::extract::Path;
use crate::{method_router, S};
use crate::R;

method_router!(
    get : "/vpn"-> vpn,
    get : "/s/:link"-> vpn,
);
async fn vpn() -> R<String> {
    let res = reqwest::get("https://bit.ly/cmwowork").await?.text().await?;
    Ok(res)
}
async fn link(s: S, name : Path<(String,)>) -> R<String> {
    // s.config.finance
    let res = reqwest::get("https://bit.ly/cmwowork").await?.text().await?;
    Ok(res)
}

