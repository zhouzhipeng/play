use std::sync::Arc;
use std::thread;


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



pub struct AppState {
    pub template_service: TemplateService,
    pub db: DBPool,
    pub redis_service: RedisService,
    pub config: Config,
}



pub async fn init_app_state(use_test_pool: bool) -> Arc<AppState> {
// init config
    let config = init_config();

    let final_test_pool = use_test_pool || config.use_test_pool;

    info!("use test pool : {}", final_test_pool);

    //create a group of channels to handle python code running
    let (req_sender, req_receiver) = async_channel::bounded::<TemplateData>(1);
    let (res_sender, res_receiver) = async_channel::bounded::<String>(1);

    // Create an instance of the shared state
    let app_state = Arc::new(AppState {
        template_service: TemplateService::new(req_sender, res_receiver),
        db: if final_test_pool {tables::init_test_pool().await}else{tables::init_pool(&config).await},
        redis_service: RedisService::new(config.redis_uri.clone(), final_test_pool).await.unwrap(),
        config,
    });


    //run a thread to run python code.
    info!("ready to spawn py_runner");
    // tokio::spawn(async move { py_runner::run(req_receiver, res_sender).await; });
    thread::spawn(move ||{ py_runner::run(req_receiver, res_sender); });
    app_state
}


