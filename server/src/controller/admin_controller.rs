use std::env::temp_dir;
use std::fs::File;
use std::io::Cursor;
use std::process::Command;

use axum::extract::Query;
use axum::response::Html;
use serde::Deserialize;

use crate::{HTML, method_router};

method_router!(
    get : "/admin/upgrade" -> upgrade,
);

#[derive(Deserialize)]
struct UpgradeRequest {
    url: String,
}

async fn upgrade(Query(upgrade): Query<UpgradeRequest>) -> HTML {
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
    //
    // let mut cmd = Command::new(std::env::args().next().unwrap());
    // cmd.args(std::env::args().skip(1));
    // std::process::exit(cmd.status().unwrap().code().unwrap());


    Ok(Html("upgrade ok. pls restart the app manually!".to_string()))
}
