use std::env::temp_dir;
use std::fs::File;
use std::io::Cursor;
use std::process::Command;
use std::time::Duration;

use axum::extract::Query;
use axum::response::Html;
use serde::Deserialize;

use crate::{HTML, method_router, S, shutdown};

method_router!(
    get : "/admin/upgrade" -> upgrade,
);

#[derive(Deserialize)]
struct UpgradeRequest {
    url: String,
}

async fn upgrade(s: S, Query(upgrade): Query<UpgradeRequest>) -> HTML {
    // Create a file inside of `std::env::temp_dir()`.

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


    Ok(Html("upgrade ok. pls restart the app manually!".to_string()))
}
