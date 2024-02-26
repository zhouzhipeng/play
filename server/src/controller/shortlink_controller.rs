use crate::method_router;
use crate::R;

method_router!(
    get : "/vpn"-> vpn,
);
async fn vpn() -> R<String> {
    let res = reqwest::get("https://bit.ly/cmwowork").await?.text().await?;
    Ok(res)
}

