use std::env::temp_dir;
use std::fs::File;
use std::io::Cursor;
use std::time::Duration;

use axum::extract::Query;
use axum::response::Html;
use reqwest::{ClientBuilder, Url};
use serde::Deserialize;
use serde_json::json;
use tracing::info;

use crate::{check, CONFIG, HTML, method_router, S, template};

method_router!(
    get : "/admin/upgrade" -> upgrade,
    get : "/admin/reboot" -> reboot,
    get : "/admin/index" -> enter_admin_page,
);

#[derive(Deserialize)]
struct UpgradeRequest {
    url: String,
}

async fn enter_admin_page(s: S) -> HTML {
    let config = &CONFIG;

    template!(s, "frame.html", "fragments/admin.html", json!({
        "upgrade_url" : &config.upgrade_url
    }))
}

async fn upgrade_in_background(s: S, url: Url) ->anyhow::Result<()>{
    info!("begin to download from url in background  : {}", url);

    // download file
    let new_binary = temp_dir().join("new_play_bin");
    let mut file = File::create(&new_binary)?;
    let client = ClientBuilder::new().timeout(Duration::from_secs(30)).build()?;
    let response = client.get(url).send().await?;
    let mut content = Cursor::new(response.bytes().await?);
    std::io::copy(&mut content, &mut file)?;

    info!("downloaded and saved at : {:?}", new_binary);

    self_replace::self_replace(&new_binary)?;
    std::fs::remove_file(&new_binary)?;

    info!("replaced ok. and ready to reboot self...");


    #[cfg(not(feature = "debug"))]
    s.shutdown_handle.shutdown();

    Ok(())
}

async fn upgrade(s: S, Query(upgrade): Query<UpgradeRequest>) -> HTML {
    let url = Url::parse(&upgrade.url)?;


    tokio::spawn(async move{
        let r = upgrade_in_background(s, url).await;
        info!("upgrade_in_background result >> {:?}", r);
    });


    Ok(Html("upgrade in background, pls wait and check for console logs.".to_string()))
}

async fn reboot(s: S) -> HTML {


    info!("ready to reboot...");

    #[cfg(not(feature = "debug"))]
    s.shutdown_handle.shutdown();

    Ok(Html("reboot ok.".to_string()))
}
