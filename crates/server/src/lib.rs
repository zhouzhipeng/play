#![allow(non_camel_case_types)]

use std::backtrace::Backtrace;
use std::env;
use std::fmt::format;
use std::net::SocketAddr;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, UNIX_EPOCH};
use anyhow::{anyhow, bail};
use anyhow::__private::kind::TraitKind;
use async_channel::Receiver;
use async_trait::async_trait;

use axum::extract::{DefaultBodyLimit, State};
use axum::http::{Method, StatusCode};
use axum::Json;
use axum::response::{Html, IntoResponse, Response};
use axum::Router;
use axum_server::Handle;
use http_body::Body;
use hyper::HeaderMap;
use include_dir::{Dir, include_dir};
use lazy_static::lazy_static;
use serde::Serialize;
use serde_json::{json, Value};
use tokio::fs;
use tokio::time::sleep;
use tower_http::compression::{CompressionLayer, DefaultPredicate, Predicate};
use tower_http::compression::predicate::{NotForContentType, SizeAbove};
use tower_http::cors::{Any, CorsLayer};
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::{DefaultMakeSpan, TraceLayer};
use tracing::{error, info};
use shared::constants::{CAT_FINGERPRINT, CAT_MAIL, DATA_DIR};

use shared::{current_timestamp, timestamp_to_date_str};

use shared::redis_api::RedisAPI;
use shared::tpl_engine_api::{Template, TemplateData, TplEngineAPI};

use crate::config::Config;
use crate::config::init_config;
use crate::controller::{app_routers, shortlink_controller};
use crate::layer::http_log::{HttpLogLayer};
use crate::service::elevenlabs_service::ElevenlabsService;
use crate::service::openai_service::OpenAIService;
use crate::service::template_service;
use crate::service::template_service::{TemplateService};
use crate::tables::DBPool;
use crate::tables::general_data::GeneralData;


pub mod controller;
pub mod tables;
pub mod service;
pub mod config;
pub mod layer;
pub mod extractor;
pub mod types;






///
/// a replacement of `ensure!` in anyhow
#[macro_export]
macro_rules! ensure {
    ($($tt:tt)*) => {
        {
            (||{
                anyhow::ensure!($($tt)*);
                Ok(())
            })()?
        }
    };
}



#[async_trait]
trait CheckResponse{
    async fn check(self)->anyhow::Result<Self>
        where Self:Sized;
}


#[async_trait]
impl CheckResponse for reqwest::Response {

    ///
    /// auto check http status code make sure it's 2xx
    async fn check(self) -> anyhow::Result<Self> {
        let url = self.url();
        let status = self.status();
        let msg = format!("request url : {},  status : {}",url , status);

        if !self.status().is_success(){
            let resp_text = self.text().await?;
            error!("{} , error http response >> {}",msg, resp_text);
            bail!(resp_text)
        }else{
            info!("{}", msg);
            return Ok(self)
        }
    }
}



#[macro_export]
macro_rules! mock_state {
    ()=>{
        {


           axum::extract::State(crate::init_app_state(&crate::config::init_config(true), true).await)

        }
    };
}
#[macro_export]
macro_rules! init_log {
    ()=>{
        {
             use tracing_subscriber::util::SubscriberInitExt;
              tracing_subscriber::fmt()
            .with_file(true)
            .with_line_number(true)
            .with_thread_names(true)
            .pretty()
            .with_writer(std::io::stdout)
            .finish()
            .init();



        }
    };
}
#[macro_export]
macro_rules! mock_server {
    ()=>{
        axum_test::TestServer::new(play::routers(play::init_app_state(&play::config::init_config(true), true).await))?;
    };
}
#[macro_export]
macro_rules! mock_server_state {
    ()=>{
        {
           let s = play::init_app_state(&play::config::init_config(true), true).await;
            let server = axum_test::TestServer::new(play::routers(s.clone()))?;
            (server, axum::extract::State(s))
        }

    };
}


///
/// a replacement for `bail!` in anyhow
#[macro_export]
macro_rules! return_error {
    ($msg:literal $(,)?) => {
        // tracing::info!("Error: {} (file: {}, line: {})", error, file!(), line!());
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
    pub elevenlabs_service: ElevenlabsService,
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
        openai_service: OpenAIService::new(config.open_ai.api_key.to_string()).unwrap(),
        elevenlabs_service: ElevenlabsService::new(config.elevenlabs.api_key.to_string(), config.elevenlabs.voice_id.to_string()).unwrap(),
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
        .serve(router.into_make_service_with_connect_info::<SocketAddr>())
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

//
//
// lazy_static! {
//     pub static ref CONFIG: Config = init_config(false);
// }



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

#[derive(Clone, Copy, Debug)]
struct CustomCompressPredict{}



impl Predicate for CustomCompressPredict {
    fn should_compress<B>(&self, response: &http::Response<B>) -> bool where B: Body {
        response.headers().contains_key("x-compress")
    }
}


pub async fn routers(app_state: Arc<AppState>) -> anyhow::Result<Router> {
    let cors = CorsLayer::new()
        // allow `GET` and `POST` when accessing the resource
        .allow_methods(Any)
        .allow_headers(Any)
        // allow requests from any origin
        .allow_origin(Any);

    // Define the maximum body size (in bytes).
    let max_body_size = 10 * 1024 * 1024; // For example, 10 MB



    let mut auth_config = app_state.config.auth_config.clone();
    let mut shortlinks:Vec<String> = app_state.config.shortlinks.iter().map(|p|p.from.to_string()).collect();
    auth_config.whitelist.append(&mut shortlinks);

    info!("whitelist : {:?}", auth_config.whitelist);

    let mut fingerprints = GeneralData::query_by_cat_simple(CAT_FINGERPRINT,1000,&app_state.db).await?.iter().map(|f|f.data.to_string()).collect::<Vec<String>>();
    auth_config.fingerprints.append(&mut fingerprints);

    info!("fingerprints : {:?}", auth_config.fingerprints);





    let mut router = Router::new()
        .merge(shortlink_controller::init(app_state.clone()))
        .merge(app_routers())
        .with_state(app_state)
        // logging so we can see whats going on
        .layer(TraceLayer::new_for_http().make_span_with(DefaultMakeSpan::default().include_headers(true)))
        .layer(TimeoutLayer::new(Duration::from_secs(600)))
        .layer(DefaultBodyLimit::disable())
        .layer(HttpLogLayer{ auth_config })
        .layer(cors);

    //
    // #[cfg(not(feature = "debug"))]
    // {
        router = router.layer(CompressionLayer::new().compress_when(CustomCompressPredict{}));
    // }


    Ok(router)
}

// Make our own error that wraps `anyhow::Error`.
#[derive(Debug)]
pub struct AppError(anyhow::Error);

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {


        let error = self.0;

        error!("server error: {}", error);


        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Server Error: {}", error),
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
    Ok(Html(content.trim().to_string()))
}


#[cfg(feature = "mail_server")]
pub async fn handle_email_message(copy_appstate: &Arc<AppState>, msg: &mail_server::models::message::Message) {
    //double write
    if GeneralData::query_count(CAT_MAIL, &copy_appstate.db).await.unwrap_or_default()>=50{
        info!("delete emails");
        GeneralData::delete_by_cat(CAT_MAIL, &copy_appstate.db).await;
    }
    let r2 = GeneralData::insert(CAT_MAIL,
        &json!({
            "from_mail": msg.sender.to_string(),
            "to_mail": msg.recipients.join(","),
            "send_date": msg.created_at.as_ref().unwrap_or(&String::from("")).to_string(),
            "subject": msg.subject.to_string(),
            "plain_content": msg.plain.as_ref().unwrap_or(&String::from("")).to_string(),
            "html_content": msg.html.as_ref().unwrap_or(&String::from("")).to_string(),
            "full_body": "<TODO>".to_string(),
            "attachments": "<TODO>".to_string(),
            "create_time": current_timestamp!(),
        }).to_string()
    ,  &copy_appstate.db).await;
    info!("email insert result2 : {:?}", r2);


    //send push
    let sender= urlencoding::encode(&msg.sender).into_owned();
    let title= urlencoding::encode(&msg.subject).into_owned();
    reqwest::get(format!("{}/{}/{}", &copy_appstate.config.misc_config.mail_notify_url, sender, title)).await;

}



// Define a macro to convert a hex string literal to a Rust string
#[macro_export]
macro_rules! hex_to_string {
    ($hex_literal:expr) => {{
         // Convert the hex string to a vector of bytes
        let bytes = hex::decode($hex_literal).expect("Invalid hex string");

        // Convert the byte vector to a UTF-8 string
        String::from_utf8(bytes).unwrap()

    }};
}
#[macro_export]
macro_rules! string_to_hex {
    ($raw:expr) => {{
        $raw.as_bytes()
        .iter()
        .map(|&b| format!("{:02x}", b))
        .collect::<String>()
    }};
}


// Include the generated-file as a seperate module
#[cfg(test)]
#[cfg(feature = "mail_server")]
mod test {

    use super::*;


    #[ignore]
    #[tokio::test]
    #[cfg(feature = "mail_server")]
    async fn test_save_email() ->anyhow::Result<()>{
        use mail_server::models::message::Message;
        let s = mock_state!();
        handle_email_message(&s, &Message{
            id: Some(1),
            sender: "test".to_string(),
            recipients: vec!["test@test.com".to_string()],
            subject: "testsub".to_string(),
            created_at: Some("1111".to_string()),
            attachments: vec![],
            source: vec![],
            formats: vec![],
            html: Some("html".to_string()),
            plain: Some("plain".to_string()),
        }).await;

        let r = GeneralData::query_by_cat_simple(CAT_MAIL, &s.db).await?;
        println!("{:?}" , r);

        Ok(())
    }
}


pub async fn get_file_modify_time(path: &PathBuf)->i64{
    if let Ok(metadata) = fs::metadata(&path).await {
        if let Ok(modify_time) = metadata.modified() {
            // 使用 chrono 来格式化时间
            let modify_time = modify_time
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_millis() as i64;  // 转换为毫秒
            return modify_time
        }
    }

    0
}