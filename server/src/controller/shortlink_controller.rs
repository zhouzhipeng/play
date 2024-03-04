use anyhow::anyhow;
use axum::extract::Path;
use axum::response::{IntoResponse, Redirect};
use either::Either;
use crate::{app_error, AppError, method_router, return_error, S};
use crate::R;

method_router!(
    get : "/vpn"-> vpn,
    get : "/s/:link"-> vpn,
);
async fn vpn() -> R<String> {
    let res = reqwest::get("https://bit.ly/cmwowork").await?.text().await?;
    Ok(res)
}
async fn link(s: S, Path((name,)) : Path<(String,)>) -> R<Either<String, Redirect>> {
    let shortlink = s.config.shortlinks.iter().find(|c|c.from==name).ok_or::<AppError>(app_error!("404 , link not found."))?;
    // s.config.finance
    if !shortlink.jump{
        let res = reqwest::get("https://bit.ly/cmwowork").await?.text().await?;
        Ok(Either::Left(res))
    }else{
        Ok(Either::Right(Redirect::temporary(&shortlink.to)))
    }

}

