use std::path::PathBuf;
use axum::body::Body;
use axum::extract::Query;
use axum::response::{Html, IntoResponse, Response};
// removed dioxus usage; render pure HTML
use serde::Deserialize;
// serde_json available elsewhere if needed; not used here
use tokio::fs::File;
use tokio_util::codec::{BytesCodec, FramedRead};
use tracing::{info, warn};

use play_shared::constants::CAT_FINGERPRINT;

use crate::{method_router, return_error};
use crate::{HTML, R, S};
use crate::config::get_config_path;
use crate::controller::admin_controller::shutdown;
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

fn escape_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
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

    // Build the inner content HTML (modern layout)
    let mut content = String::new();

    // Quick Actions Section with compact card grid
    content.push_str(r#"
        <section class="section">
            <div class="section-header">
                <h2>
                    <span class="section-icon">⚡</span>
                    Quick Actions
                </h2>
            </div>
            <div class="cards-grid">
                <a class="card" href="/static/page-editor.html">
                    <div class="card-icon">📝</div>
                    <div class="card-content">
                        <div class="card-title">Page Editor</div>
                        <div class="card-description">Create and edit pages</div>
                    </div>
                </a>
                <a class="card" href="/static/file-explorer.html">
                    <div class="card-icon">📁</div>
                    <div class="card-content">
                        <div class="card-title">File Browser</div>
                        <div class="card-description">Browse and manage files</div>
                    </div>
                </a>
                <a class="card" href="/static/fileupload.html">
                    <div class="card-icon">⬆️</div>
                    <div class="card-content">
                        <div class="card-title">Upload Files</div>
                        <div class="card-description">Quick file uploads</div>
                    </div>
                </a>
                <a class="card" href="/static/plugin-manager.html">
                    <div class="card-icon">🔌</div>
                    <div class="card-content">
                        <div class="card-title">Plugin Manager</div>
                        <div class="card-description">Configure plugins</div>
                    </div>
                </a>
                <a class="card" href="/web-terminal">
                    <div class="card-icon">⌨️</div>
                    <div class="card-content">
                        <div class="card-title">Web Terminal</div>
                        <div class="card-description">Run commands</div>
                    </div>
                </a>
                <a class="card" href="/static/crontab-manager.html">
                    <div class="card-icon">⏱️</div>
                    <div class="card-content">
                        <div class="card-title">Crontab</div>
                        <div class="card-description">Schedule tasks</div>
                    </div>
                </a>
                <a class="card" href="/admin/translator">
                    <div class="card-icon">🌐</div>
                    <div class="card-content">
                        <div class="card-title">Translator</div>
                        <div class="card-description">Translate text</div>
                    </div>
                </a>
                <a class="card" href="/static/shortlink-manager.html">
                    <div class="card-icon">🔗</div>
                    <div class="card-content">
                        <div class="card-title">Shortlinks</div>
                        <div class="card-description">Manage URLs</div>
                    </div>
                </a>
            </div>
        </section>
    "#);

    // Short links section with modern chips
    if !s.config.shortlinks.is_empty() {
        content.push_str(r#"
            <section class="section">
                <div class="section-header">
                    <h2>
                        <span class="section-icon">🔖</span>
                        Quick Links
                    </h2>
                </div>
                <div class="chips-container">
        "#);
        for item in &s.config.shortlinks {
            let href = escape_html(&item.from);
            let text = escape_html(item.from.trim_start_matches('/'));
            content.push_str(&format!(r#"<a class="chip" target="_blank" href="{}">🔗 {}</a>"#, href, text));
        }
        content.push_str("</div></section>");
    }

    // Business pages with modern list design
    if !pages.is_empty() {
        content.push_str(r#"
            <section class="section">
                <div class="section-header">
                    <h2>
                        <span class="section-icon">📚</span>
                        Business Pages
                    </h2>
                </div>
                <div class="pages-list">
        "#);
        for item in pages {
            let href = escape_html(&format!("/pages{}", item.url));
            let title = escape_html(&item.title);
            content.push_str(&format!(r#"
                <a class="page-item" href="{}">
                    <div class="page-icon">📄</div>
                    <div class="page-title">{}</div>
                </a>
            "#, href, title));
        }
        content.push_str("</div></section>");
    }

    let html = INDEX_NEW_HTML.replace("{{content}}", &content)
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
