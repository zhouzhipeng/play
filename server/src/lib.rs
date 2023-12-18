use std::net::{SocketAddr, TcpListener};
use std::ops::Deref;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use axum::Json;
use axum::extract::State;
use axum::http::{Method, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::Router;
use hyper::HeaderMap;
use lazy_static::lazy_static;
use serde::Serialize;
use serde_json::{json, Value};
use tower_http::cors::{Any, CorsLayer};
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::{DefaultMakeSpan, TraceLayer};
use tracing::info;

use crate::config::Config;
use crate::config::init_config;
use crate::controller::app_routers;
use crate::service::redis::RedisService;
use crate::service::template_service::{TemplateData, TemplateService};
use crate::tables::DBPool;
use crate::threads::py_runner;

pub mod controller;
pub mod threads;
pub mod tables;
pub mod service;
pub mod config;



#[macro_export]
macro_rules! file_path {
    ($s:expr) => {
        concat!(env!("CARGO_MANIFEST_DIR"),$s)
    };
}

#[macro_export]
macro_rules! check {
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


lazy_static! {
    pub static ref CONFIG: Config = init_config();
}

pub struct AppState {
    pub template_service: TemplateService,
    pub db: DBPool,
    pub redis_service: RedisService,
    pub shutdown_handle: Handle,
}


pub async fn init_app_state(config: &Config, use_test_pool: bool) -> Arc<AppState> {
    let final_test_pool = use_test_pool || config.use_test_pool;

    info!("use test pool : {}", final_test_pool);

    //create a group of channels to handle python code running
    let (req_sender, req_receiver) = async_channel::unbounded::<TemplateData>();

    // Create an instance of the shared state
    let app_state = Arc::new(AppState {
        template_service: TemplateService::new(req_sender),
        db: if final_test_pool { tables::init_test_pool().await } else { tables::init_pool(&config).await },
        redis_service: RedisService::new(config.redis_uri.clone(), final_test_pool).await.unwrap(),
        shutdown_handle:  Handle::new()
    });


    //run a thread to run python code.
    info!("ready to spawn py_runner");
    tokio::spawn(async move { py_runner::run(req_receiver).await; });
    // let copy_receiver = req_receiver.clone();
    // let copy_receiver2 = req_receiver.clone();
    // thread::Builder::new().name("py_runner_1".to_string()).spawn(move || { py_runner::run(req_receiver); });
    // thread::Builder::new().name("py_runner_2".to_string()).spawn(move || { py_runner::run(copy_receiver2); });
    // thread::Builder::new().name("py_runner_3".to_string()).spawn(move || { py_runner::run(req_receiver); });
    app_state
}

use axum_server::Handle;
use tokio::time::sleep;

pub async fn start_server(router: Router, app_state: Arc<AppState>) {
    let server_port = CONFIG.server_port;
    info!("server start at  : http://127.0.0.1:{}", server_port);

    let addr = SocketAddr::from(([127, 0, 0, 1], server_port as u16));

    // run it with hyper on localhost:3000
    axum_server::bind(addr)
        .handle(app_state.shutdown_handle.clone())
        .serve(router.into_make_service())
        .await
        .unwrap();


    //run after `handle` shutdown() being called.
    // tokio::time::sleep(Duration::from_secs(1)).await;
    info!("shutdown self , and ready to pull a new app.");
    let mut cmd = Command::new(std::env::args().next().unwrap());
    cmd.args(std::env::args().skip(1));
    std::process::exit(cmd.status().unwrap().code().unwrap());

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



pub fn routers(app_state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        // allow `GET` and `POST` when accessing the resource
        .allow_methods([Method::GET, Method::POST])
        // allow requests from any origin
        .allow_origin(Any);

    Router::new()
        .merge(app_routers())
        .with_state(app_state)
        // logging so we can see whats going on
        .layer(TraceLayer::new_for_http().make_span_with(DefaultMakeSpan::default().include_headers(true)))
        .layer(TimeoutLayer::new(Duration::from_secs(3)))
        .layer(cors)
}

// Make our own error that wraps `anyhow::Error`.
pub struct AppError(anyhow::Error);

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
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


#[allow(dead_code)]
fn should_return_json(header_map: &HeaderMap) -> bool {
    let mut return_json = false;
    if let Some(v) = header_map.get("accept") {
        if v.to_str().unwrap().contains("json") {
            return_json = true;
        }
    }
    return_json
}


pub enum Template {
    StaticTemplate {
        name: &'static str,
        content: &'static str,
    },
    DynamicTemplate {
        name: String,
        content: String,
    },
    PythonCode {
        name: String,
        content: String,
    },
}


#[cfg(not(feature = "debug"))]
#[macro_export]
macro_rules! init_template {
    ($fragment: expr) => {
        {
            use crate::py_runner::TEMPLATES_DIR;

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
            use crate::py_runner::TEMPLATES_DIR;

            //for compiling time check file existed or not.
            include_str!(crate::file_path!(concat!("/templates/",  $fragment)));

            crate::Template::DynamicTemplate { name: $fragment.to_string(), content: fs::read_to_string(crate::file_path!(concat!("/templates/",  $fragment))).unwrap() }

        }

    };
}

#[macro_export]
macro_rules! template {
    ($s: ident, $fragment: expr, $json: expr) => {
        {
            let t = crate::init_template!($fragment);
            let content: axum::response::Html<String> = crate::render_fragment(&$s,t,  $json).await?;
            Ok(content)
        }

    };
    ($s: ident, $page: expr, $fragment: expr, $json:expr) => {
        {
            let page = crate::init_template!($page);
            let frag = crate::init_template!($fragment);
            let content: axum::response::Html<String> = crate::render_page(&$s,page,frag, $json).await?;
            Ok(content)
        }

    };
}





async fn render_page(s: &S, page: Template, fragment: Template, data: Value) -> R<Html<String>> {
    let content = s.template_service.render_template(fragment, data).await?;


    let final_data = json!({
        "content": content
    });

    let final_html = s.template_service.render_template(page, final_data).await?;

    Ok(Html(final_html))
}

async fn render_fragment(s: &S, fragment: Template, data: Value) -> R<Html<String>> {
    let content = s.template_service.render_template(fragment, data).await?;
    Ok(Html(content))
}

