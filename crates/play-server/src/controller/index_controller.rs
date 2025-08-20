use std::path::PathBuf;
use axum::body::Body;
use axum::extract::Query;
use axum::response::{Html, IntoResponse, Response};
use chrono::{TimeZone, Utc};
use dioxus::prelude::*;
use http::Request;
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::fs::File;
use tokio_util::codec::{BytesCodec, FramedRead};
use tracing::{info, warn};

use play_shared::constants::CAT_FINGERPRINT;
use play_shared::timestamp_to_date_str;

use crate::{method_router, return_error, template};
use crate::{HTML, R, S};
use crate::config::get_config_path;
use crate::controller::admin_controller::shutdown;
use crate::controller::cache_controller::generate_cache_key;
use crate::controller::pages_controller::PageDto;
use crate::tables::general_data::GeneralData;

method_router!(
    get : "/"-> root,
    get : "/robots.txt"-> robots,
    get : "/ping"-> ping,
    get : "/save-fingerprint"-> save_fingerprint,
    get : "/download-db"-> serve_db_file,
    get : "/download-config"-> serve_config_file,
);


static INDEX_NEW_HTML : &str = include_str!("templates/index-new.html");

static ROBOTS_TXT : &str = include_str!("templates/robots.txt");

async fn robots() -> R<String> {
    Ok(ROBOTS_TXT.to_string())
}


fn has_extension(url: &str)->bool{
    let p = PathBuf::from(&url);
    let extension =p
        .extension()
        .and_then(|ext| ext.to_str());
    extension.is_some()
}

async fn root(s: S) -> HTML {
    let built_time = env!("BUILT_TIME").parse::<i64>()?;
    // return_error!("test");
    let data = GeneralData::query_by_cat("title,url", "pages",1000, &s.db).await?;
    let pages = data.iter()
        .map(|p|serde_json::from_str::<PageDto>(&p.data).unwrap())
        .filter(|p|
            p.url.ends_with(".html")  || !has_extension(p.url.as_str())
        )
        .collect::<Vec<PageDto>>();

    let html = dioxus_ssr::render_element(rsx!{
        div { class: "row",
            div { class: "col",
                h2 { "System Tools" }
                ul {
                    li {
                        a { href: "/static/page-editor.html", "Page Editor" }
                    }
                    li {
                        a { href: "/static/plugin-manager.html", "Plugin Manager" }
                    }
                    li {
                        a { href: "/static/shortlink-manager.html", "Shortlinks Manager" }
                    }
                    li {
                        a { href: "/static/file-explorer.html", "File Browser" }
                    }
                    li {
                        a { href: "/static/fileupload.html", "Upload File" }
                    }
                    li {
                        a { href: "/web-terminal", "Web Terminal" }
                    }
                    li {
                        a { href: "/static/crontab-manager.html", "Crontab Manager" }
                    }
                }
                h2 { "Short Links" }
                ul {
                    for item in &s.config.shortlinks{
                    li {
                        a { target: "_blank", href: "{item.from}", "{&item.from[1..]}" }
                    }
                    }
                }
            }
            div { class: "col",
                h2 { "Business Pages" }
                ul {
                    for item in pages{
                    li {
                        a { href: "/pages{item.url}","{item.title}" }
                    }
                    }
                }
            }
        }
    });

    let html = INDEX_NEW_HTML.replace("{{content}}", &html)
        .replace("{{built_time}}", built_time.to_string().as_str());


    Ok(Html(html))

}



async fn ping() -> R<String> {
    info!("ping");
    Ok("pong".to_string())
}

#[derive(Deserialize, Debug)]
struct SaveFingerPrintReq{
    fingerprint: String,
    passcode: String
}

async fn save_fingerprint(s: S, Query(req): Query<SaveFingerPrintReq>) -> R<String> {
    //check passcode
    if &s.config.auth_config.passcode == &req.passcode{
        //save fingerprint
        let r = GeneralData::insert(CAT_FINGERPRINT, &req.fingerprint, &s.db).await?;
        info!("save fingerprint result  : {:?}", r);
    }else{
        warn!("passcode not matched. req : {:?}", req);
        return_error!("passcode not matched.")
    }

    tokio::spawn(async {
       shutdown();
    });
    Ok("save ok,will reboot in a sec.".to_string())
}



async fn serve_db_file(s: S) -> impl IntoResponse {
    let raw = s.config.database.url.to_string();
    let path = &raw["sqlite://".len()..raw.len()];
    let file = File::open(path).await.expect("Cannot open file");
    let stream = FramedRead::new(file, BytesCodec::new());
    let body = Body::from_stream(stream);
    Response::new(body)
}
async fn serve_config_file(s: S) -> impl IntoResponse {
    let path = get_config_path().unwrap();
    let file = File::open(&path).await.expect("Cannot open file");
    let stream = FramedRead::new(file, BytesCodec::new());
    let body = Body::from_stream(stream);
    Response::new(body)
}

