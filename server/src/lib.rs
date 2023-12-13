use std::sync::Arc;


use axum::Router;
use lazy_static::lazy_static;
use tracing::info;



use crate::config::Config;
use crate::config::init_config;
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



lazy_static! {
    pub static ref CONFIG: Config = init_config();
}

pub struct AppState {
    pub template_service: TemplateService,
    pub db: DBPool,
    pub redis_service: RedisService,
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


pub async fn start_server(router: Router) {
    let server_port = CONFIG.server_port;
    info!("server start at  : http://127.0.0.1:{}", server_port);
    // run it with hyper on localhost:3000
    axum::Server::bind(&format!("0.0.0.0:{}", server_port).parse().unwrap())
        .serve(router.into_make_service())
        .await
        .unwrap();
}