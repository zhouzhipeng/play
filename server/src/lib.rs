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

use axum::extract::{DefaultBodyLimit, State};
use axum::http::{Method, StatusCode};
use axum::{middleware, Json};
use axum::response::{Html, IntoResponse, Response};
use axum::Router;
use axum_server::Handle;
use http_body::Body;
use hyper::HeaderMap;
use include_dir::{include_dir, Dir};
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
use play_shared::constants::{CAT_FINGERPRINT, CAT_MAIL, DATA_DIR};

use play_shared::{current_timestamp, timestamp_to_date_str};

use play_shared::tpl_engine_api::{Template, TemplateData};

use crate::config::{Config, PluginConfig, ShortLink};
use crate::config::init_config;
use crate::controller::{app_routers, plugin_controller, shortlink_controller};
use crate::layer::custom_http_layer::http_middleware;
use crate::service::template_service;
use crate::service::template_service::TemplateService;
use crate::tables::DBPool;
use crate::tables::general_data::GeneralData;


pub mod controller;
pub mod tables;
pub mod service;
pub mod config;
pub mod layer;
pub mod extractor;






///
/// a replacement of `ensure!` in anyhow
#[macro_export]
macro_rules! promise {
    ($($tt:tt)*) => {
        {
            (||{
                anyhow::ensure!($($tt)*);
                Ok(())
            })()?
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
    pub db: DBPool,
    pub config: Config,
}


pub async fn init_app_state(config: &Config, use_test_pool: bool) -> anyhow::Result<Arc<AppState>> {
    let final_test_pool = use_test_pool || config.use_test_pool;

    info!("use test pool : {}", final_test_pool);

    //create a group of channels to handle python code running
    let (req_sender, req_receiver) = async_channel::unbounded::<TemplateData>();


    let mut inner_app_state  = AppState {
        template_service: TemplateService::new(req_sender),
        db: if final_test_pool { tables::init_test_pool().await } else { tables::init_pool(&config).await },
        config: config.clone(),
    };

    let mut auth_config = &mut inner_app_state.config.auth_config;


    let mut fingerprints = GeneralData::query_by_cat_simple(CAT_FINGERPRINT,1000,&inner_app_state.db).await?.iter().map(|f|f.data.to_string()).collect::<Vec<String>>();
    auth_config.fingerprints.append(&mut fingerprints);

    //query plugin data
    let mut plugin_config_list = &mut inner_app_state.config.plugin_config;

    //remove disabled plugins
    let mut new_plugin_config_list = vec![];
    for plugin in plugin_config_list.iter_mut() {
        if !plugin.disable {
            new_plugin_config_list.push(plugin.clone());
        }
    }
    plugin_config_list.clear();
    plugin_config_list.append(&mut new_plugin_config_list);

    let plugin_list = GeneralData::query_by_cat_simple("plugins",1000,&inner_app_state.db).await?;;
    for data in &plugin_list {
        let plugin_config = serde_json::from_value::<PluginConfig>(serde_json::from_str(&data.data)?)?;
        if plugin_config.disable{continue}

        plugin_config_list.push(plugin_config);

    }
    info!("active plugin_config_list: {:?}", plugin_config_list);


    //query shortlinks data from db
    let mut shortlinks = &mut inner_app_state.config.shortlinks;
    let db_shortlinks = GeneralData::query_by_cat_simple("shortlinks",1000,&inner_app_state.db).await?;;
    for data in &db_shortlinks {
        let shortlink = serde_json::from_value::<ShortLink>(serde_json::from_str(&data.data)?)?;
        shortlinks.push(shortlink);

    }
    info!("active shortlinks : {:?}", shortlinks);
    let mut shortlinks:Vec<String> = inner_app_state.config.shortlinks.clone().iter()
        .filter(|p|!p.auth)
        .map(|p|p.from.to_string()).collect();
    auth_config.whitelist.append(&mut shortlinks);

    info!("whitelist : {:?}", auth_config.whitelist);

    // Create an instance of the shared state
    let app_state = Arc::new(inner_app_state);


    Ok(app_state)
}

pub async fn start_server(router: Router, app_state: Arc<AppState>) -> anyhow::Result<()> {
    let server_port = app_state.config.server_port;


    println!("server started at  : http://127.0.0.1:{}", server_port);
    info!("server started at  : http://127.0.0.1:{}", server_port);

    let addr = SocketAddr::from(([0, 0, 0, 0], server_port as u16));

    #[cfg(not(feature = "play-https"))]
    // run it with hyper on localhost:3000
    axum_server::bind(addr)
        .serve(router.into_make_service_with_connect_info::<SocketAddr>())
        .await?;


    let certs_path = Path::new(env::var(DATA_DIR)?.as_str()).join("certs");


    #[cfg(feature = "play-https")]
    play_https::start_https_server(&play_https::HttpsConfig {
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
    fn should_compress<B>(&self, response: &axum::response::Response<B>) -> bool where B: http_body::Body {
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


    // info!("fingerprints : {:?}", auth_config.fingerprints);



    let mut router = Router::new()
        .merge(shortlink_controller::init(app_state.clone()))
        .merge(plugin_controller::init(app_state.clone()))
        .merge(app_routers())
        .with_state(app_state.clone())
        // logging so we can see whats going on
        .layer(TraceLayer::new_for_http().make_span_with(DefaultMakeSpan::default().include_headers(true)))
        .layer(TimeoutLayer::new(Duration::from_secs(600)))
        .layer(DefaultBodyLimit::disable())
        // .layer(HttpLogLayer{ auth_config })
        .layer(middleware::from_fn_with_state(app_state.clone(), http_middleware))
        .layer(cors)
        .layer(CompressionLayer::new().compress_when(CustomCompressPredict{}))
        .fallback(handle_404)
        ;

    //
    // #[cfg(not(feature = "debug"))]
    // {
        router = router;
    // }


    Ok(router)
}

// 创建自定义404处理函数
async fn handle_404(uri: axum::http::Uri) -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        format!("Server Error: url not found: {}", uri)
    )
}


// Make our own error that wraps `anyhow::Error`.
#[derive(Debug)]
pub struct AppError(anyhow::Error);

// Implement Display for AppError
impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let error = self.0;
        let error_msg = format!("Server Error: {:?}", error);
        error!("server error: {}", error_msg);

        (
            StatusCode::INTERNAL_SERVER_ERROR,
            error_msg,
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




#[macro_export]
macro_rules! template {
    ($s: ident, $fragment: expr, $json: expr) => {
        {

            let t = crate::Template::DynamicTemplate { name: "<string>".to_string(), content:$fragment.to_string() };
            let content: axum::response::Html<String> = crate::render_fragment(&$s,t,  $json).await?;
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

#[cfg(feature = "play-lua")]
pub async fn render_template_new(text: &str, data: Value)->anyhow::Result<String>{
    Ok(play_lua::lua_render(text, data).await?)
}

#[cfg(not(feature = "play-lua"))]
pub async fn render_template_new(text: &str, data: Value)->anyhow::Result<String> {
    bail!("feature `play-lua` not enabled")
}