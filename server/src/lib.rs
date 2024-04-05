#![allow(non_camel_case_types)]

use std::env;
use std::net::SocketAddr;
use std::ops::Deref;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use async_channel::Receiver;

use axum::extract::{DefaultBodyLimit, State};
use axum::http::{Method, StatusCode};
use axum::Json;
use axum::response::{Html, IntoResponse, Response};
use axum::Router;
use axum_server::Handle;
use either::Either;
use hyper::HeaderMap;
use include_dir::{Dir, include_dir};
use lazy_static::lazy_static;
use serde::Serialize;
use serde_json::{json, Value};
use tokio::time::sleep;
use tower_http::compression::CompressionLayer;
use tower_http::cors::{Any, CorsLayer};
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::{DefaultMakeSpan, TraceLayer};
use tracing::{error, info};
use shared::constants::DATA_DIR;

use shared::{current_timestamp, timestamp_to_date_str};

use shared::redis_api::RedisAPI;
use shared::tpl_engine_api::{Template, TemplateData, TplEngineAPI};

use crate::config::Config;
use crate::config::init_config;
use crate::controller::app_routers;
use crate::service::openai_service::OpenAIService;
use crate::service::template_service;
use crate::service::template_service::{TemplateService};
use crate::tables::DBPool;
use crate::tables::email_inbox::EmailInbox;


pub mod controller;
pub mod tables;
pub mod service;
pub mod config;
pub mod layer;
pub mod extractor;






///
/// a replacement of `ensure!` in anyhow
#[macro_export]
macro_rules! ensure {
    ($($tt:tt)*) => {
        {
            use anyhow::ensure;
            (||{
                ensure!($($tt)*);
                Ok(())
            })()?
        }
    };
}
#[macro_export]
macro_rules! mock_state {
    ()=>{
       axum::extract::State(crate::init_app_state(&crate::config::init_config(true), true).await)
    };
}
#[macro_export]
macro_rules! mock_server {
    ()=>{
        axum_test::TestServer::new(play::routers(play::init_app_state(&play::config::init_config(true), true).await))?;
    };
}


///
/// a replacement for `bail!` in anyhow
#[macro_export]
macro_rules! return_error {
    ($msg:literal $(,)?) => {
        return Err(anyhow::anyhow!($msg).into());
    };
    ($err:expr $(,)?) => {
        return Err(anyhow::anyhow!($err).into())
    };
    ($fmt:expr, $($arg:tt)*) => {
        return Err(anyhow::anyhow!($fmt, $($arg)*).into())
    };
}
#[macro_export]
macro_rules! app_error {
    ($msg:literal $(,)?) => {
        anyhow::anyhow!($msg).into()
    };
    ($err:expr $(,)?) => {
        anyhow::anyhow!($err).into()
    };
    ($fmt:expr, $($arg:tt)*) => {
        anyhow::anyhow!($fmt, $($arg)*).into()
    };
}


pub struct AppState {
    pub template_service: TemplateService,
    pub openai_service: OpenAIService,
    pub db: DBPool,
    pub redis_service: Box<dyn RedisAPI + Send + Sync>,
    pub config: Config,
}


pub async fn init_app_state(config: &Config, use_test_pool: bool) -> Arc<AppState> {
    let final_test_pool = use_test_pool || config.use_test_pool;

    info!("use test pool : {}", final_test_pool);

    //create a group of channels to handle python code running
    let (req_sender, req_receiver) = async_channel::unbounded::<TemplateData>();

    // Create an instance of the shared state
    let app_state = Arc::new(AppState {
        template_service: TemplateService::new(req_sender),
        openai_service: OpenAIService::new(config.open_ai.api_key.to_string(), config.open_ai.assistant_id.to_string()).unwrap(),
        db: if final_test_pool { tables::init_test_pool().await } else { tables::init_pool(&config).await },
        #[cfg(feature = "redis")]
        redis_service: Box::new(redis::RedisService::new(config.redis_uri.clone(), final_test_pool).await.unwrap()),
        #[cfg(not(feature = "redis"))]
        redis_service: Box::new(crate::service::redis_fake_service::RedisFakeService::new(config.redis_uri.clone(), final_test_pool).await.unwrap()),
        config: config.clone(),
    });


    #[cfg(feature = "tpl")]
    start_template_backend_thread(Box::new(tpl::TplEngine {}), req_receiver);
    #[cfg(not(feature = "tpl"))]
    start_template_backend_thread(Box::new(crate::service::tpl_fake_engine::FakeTplEngine {}), req_receiver);


    app_state
}


fn start_template_backend_thread(tpl_engine: Box<dyn TplEngineAPI + Send + Sync>, req_receiver: Receiver<TemplateData>) {
    info!("ready to spawn py_runner");
    tokio::spawn(async move { tpl_engine.run_loop(req_receiver).await; });
}

pub async fn start_server(router: Router, app_state: Arc<AppState>) -> anyhow::Result<()> {
    let server_port = app_state.config.server_port;


    println!("server started at  : http://127.0.0.1:{}", server_port);
    info!("server started at  : http://127.0.0.1:{}", server_port);

    let addr = SocketAddr::from(([0, 0, 0, 0], server_port as u16));

    #[cfg(not(feature = "https"))]
    // run it with hyper on localhost:3000
    axum_server::bind(addr)
        .serve(router.into_make_service())
        .await?;


    let certs_path = Path::new(env::var(DATA_DIR)?.as_str()).join("certs");


    #[cfg(feature = "https")]
    https::start_https_server(&https::HttpsConfig {
        domains: app_state.config.https_cert.domains.clone(),
        email: app_state.config.https_cert.emails.clone(),
        cache_dir: certs_path.to_str().unwrap().to_string(),
        prod: true,
        http_port: server_port as u16,
        https_port: app_state.config.https_cert.https_port,
        auto_redirect:app_state.config.https_cert.auto_redirect,
    }, router).await;


    Ok(())
}

pub async fn shutdown_another_instance(local_url: &String) {
//check if port is already in using. if it is , call /shutdown firstly.
    let shutdown_result = reqwest::get(&format!("{}/admin/shutdown", local_url)).await;
    info!("shutdown_result >> {} , can be ignored.", shutdown_result.is_ok());
    sleep(Duration::from_micros(200)).await;
}


type R<T> = Result<T, AppError>;
type S = State<Arc<AppState>>;

type HTML = Result<Html<String>, AppError>;
type JSON<T> = Result<Json<T>, AppError>;



#[derive(Serialize)]
pub struct Success {}


#[macro_export]
macro_rules! register_routers {
    ($($c: ident),*$(,)?) => {
            use axum::Router;
            pub fn app_routers() -> Router<std::sync::Arc<crate::AppState>> {

            let mut router = Router::new();
            $(

                #[allow(unused_assignments)]
                {
                    router= router.merge($c::init());
                }
            )*

            router

    }
    };
}

#[macro_export]
macro_rules! method_router {
    ($($m: ident : $u :literal-> $f: ident),*$(,)?) => {
        pub fn init() -> axum::Router<std::sync::Arc<crate::AppState>> {
            let mut router = axum::Router::new();
            $(
                router = router.route($u, axum::routing::$m($f));
            )*
            router
        }
    };
}
#[macro_export]
macro_rules! data_dir {
    () => {
        std::path::Path::new(std::env::var("DATA_DIR").unwrap().as_str());
    };
}
#[macro_export]
macro_rules! files_dir {
    () => {
        std::path::Path::new(std::env::var("DATA_DIR").unwrap().as_str()).join("files");
    };
}



pub fn routers(app_state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        // allow `GET` and `POST` when accessing the resource
        .allow_methods([Method::GET, Method::POST])
        // allow requests from any origin
        .allow_origin(Any);

    // Define the maximum body size (in bytes).
    let max_body_size = 10 * 1024 * 1024; // For example, 10 MB



    Router::new()
        .merge(app_routers())
        .with_state(app_state)
        // logging so we can see whats going on
        .layer(TraceLayer::new_for_http().make_span_with(DefaultMakeSpan::default().include_headers(true)))
        .layer(TimeoutLayer::new(Duration::from_secs(60)))
        .layer(DefaultBodyLimit::disable())
        .layer(CompressionLayer::new())
        .layer(cors)
}

// Make our own error that wraps `anyhow::Error`.
#[derive(Debug)]
pub struct AppError(anyhow::Error);

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        // #[cfg(feature = "debug")]
        error!("server error: {}", self.0.to_string());


        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Server Error: {}", self.0),
        )
            .into_response()
    }
}


impl Deref for AppError {
    type Target = anyhow::Error;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// This enables using `?` on functions that return `Result<_, anyhow::Error>` to turn them into
// `Result<_, AppError>`. That way you don't need to do that manually.
impl<E> From<E> for AppError
    where
        E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}






#[cfg(not(feature = "debug"))]
#[macro_export]
macro_rules! init_template {
    ($fragment: expr) => {
        {
            #[cfg(feature = "tpl")]
            use tpl::TEMPLATES_DIR;
            #[cfg(not(feature = "tpl"))]
            use crate::service::tpl_fake_engine::TEMPLATES_DIR;

            let content = TEMPLATES_DIR.get_file($fragment).unwrap().contents_utf8().unwrap();
            crate::Template::StaticTemplate { name: $fragment, content: content }

        }

    };
}

#[cfg(feature = "debug")]
#[macro_export]
macro_rules! init_template {
    ($fragment: expr) => {
        {
            use std::fs;

            //for compiling time check file existed or not.
            include_str!(shared::file_path!(concat!("/templates/",  $fragment)));

            crate::Template::DynamicTemplate { name: $fragment.to_string(), content: fs::read_to_string(shared::file_path!(concat!("/templates/",  $fragment))).unwrap() }

        }

    };
}

#[macro_export]
macro_rules! template {
    ($s: ident, $fragment: literal, $json: expr) => {
        {
            let t = crate::init_template!($fragment);
            let content: axum::response::Html<String> = crate::render_fragment(&$s,t,  $json).await?;
            Ok(content)
        }

    };
    ($s: ident, $page: literal + $fragment: literal, $json:expr) => {
        {
            let page = crate::init_template!($page);
            let frag = crate::init_template!($fragment);
            let content: axum::response::Html<String> = crate::render_page(&$s,page,frag, $json).await?;
            Ok(content)
        }

    };
}





async fn render_page(s: &S, page: Template, fragment: Template, data: Value) -> R<Html<String>> {
    let title = if let Some(title) = data["title"].as_str() { title } else { "<no title>" };
    let title = title.to_string();

    let content = s.template_service.render_template(fragment, data).await?;

    let built_time = timestamp_to_date_str!(env!("BUILT_TIME").parse::<i64>()?);

    let final_data = json!({
        "built_time": built_time,
        "title": title,
        "content": content
    });

    let final_html = s.template_service.render_template(page, final_data).await?;

    Ok(Html(final_html))
}

async fn render_fragment(s: &S, fragment: Template, data: Value) -> R<Html<String>> {
    let content = s.template_service.render_template(fragment, data).await?;
    Ok(Html(content))
}


#[cfg(feature = "mail_server")]
pub async fn handle_email_message(copy_appstate: &Arc<AppState>, msg: &mail_server::models::message::Message) {
    //delete all if email count > 100
    if EmailInbox::count(&copy_appstate.db).await.unwrap_or_default() >= 50{
        info!("delete emails");
        EmailInbox::delete_all(&copy_appstate.db).await;
    }

    let r = EmailInbox::insert(&EmailInbox {
        from_mail: msg.sender.to_string(),
        to_mail: msg.recipients.join(","),
        send_date: msg.created_at.as_ref().unwrap_or(&String::from("")).to_string(),
        subject: msg.subject.to_string(),
        plain_content: msg.plain.as_ref().unwrap_or(&String::from("")).to_string(),
        html_content: msg.html.as_ref().unwrap_or(&String::from("")).to_string(),
        full_body: "<TODO>".to_string(),
        attachments: "<TODO>".to_string(),
        create_time: current_timestamp!(),
        ..Default::default()
    }, &copy_appstate.db).await;
    info!("email insert result : {:?}", r);

    //send push
    let sender= urlencoding::encode(&msg.sender).into_owned();
    let title= urlencoding::encode(&msg.subject).into_owned();
    reqwest::get(format!("https://api.day.app/pTyPrycAjp36tGRSAUgfiU/{}/{}", sender, title)).await;
}



// Define a macro to convert a hex string literal to a Rust string
#[macro_export]
macro_rules! hex_to_string {
    ($hex_literal:expr) => {{
        let mut chars = $hex_literal.chars();
        let mut string = String::with_capacity($hex_literal.len() / 2);

        while let (Some(high), Some(low)) = (chars.next(), chars.next()) {
            let high_val = high.to_digit(16).unwrap_or(0);
            let low_val = low.to_digit(16).unwrap_or(0);
            string.push(char::from((high_val << 4 | low_val) as u8));
        }

        string
    }};
}


// Include the generated-file as a seperate module
#[cfg(test)]
mod test {
    use super::*;

    #[ignore]
    #[tokio::test]
    async fn test_send_push() {
        let sender= urlencoding::encode("aa@bb.com").into_owned();
        let title= urlencoding::encode("sdf sdfs  👋 ").into_owned();
        reqwest::get(format!("https://api.day.app/pTyPrycAjp36tGRSAUgfiU/{}/{}", sender, title)).await;
    }
}


