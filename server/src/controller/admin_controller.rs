use std::env::temp_dir;
use std::fs::File;
use std::io::Cursor;

use axum::extract::Query;
use axum::response::Html;
use reqwest::Url;
use serde::Deserialize;
use serde_json::json;

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

async fn upgrade(s: S, Query(upgrade): Query<UpgradeRequest>) -> HTML {
    Url::parse(&upgrade.url)?;

    println!("begin to download from url  : {}", &upgrade.url);

    // download file
    let new_binary = temp_dir().join("new_play_bin");
    let mut file = File::create(&new_binary)?;
    let response = reqwest::get(&upgrade.url).await?;
    let mut content = Cursor::new(response.bytes().await?);
    std::io::copy(&mut content, &mut file)?;

    println!("downloaded and saved at : {:?}", new_binary);

    self_replace::self_replace(&new_binary)?;
    std::fs::remove_file(&new_binary)?;

    println!("replaced ok. and ready to reboot self...");


    s.shutdown_handle.shutdown();


    Ok(Html("upgrade ok.".to_string()))
}

async fn reboot(s: S) -> HTML {


    println!("ready to reboot...");

    s.shutdown_handle.shutdown();

    Ok(Html("reboot ok.".to_string()))
}
