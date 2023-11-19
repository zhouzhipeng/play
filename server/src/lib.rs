use std::sync::Arc;
use std::thread;

use crossbeam_channel::bounded;
use tokio::spawn;
use tracing::info;

use crate::config::Config;
use crate::config::init_config;
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
    pub config: Config,
}



pub async fn init_app_state(use_test_pool: bool) -> Arc<AppState> {
// init config
    let config = init_config();

    //create a group of channels to handle python code running
    let (req_sender, req_receiver) = bounded::<TemplateData>(0);
    let (res_sender, res_receiver) = bounded::<String>(1);

    // Create an instance of the shared state
    let app_state = Arc::new(AppState {
        template_service: TemplateService::new(req_sender, res_receiver),
        db: if use_test_pool {tables::init_test_pool().await}else{tables::init_pool(&config).await},
        config,
    });


    //run a thread to run python code.
    info!("ready to spawn py_runner");
    // spawn(async move { py_runner::run(req_receiver, res_sender).await; });
    thread::spawn(move ||{ py_runner::run(req_receiver, res_sender); });
    app_state
}


#[macro_export]
macro_rules! include_html {
    ($name1: ident, $name2: ident,$fragment: expr) => {
        pub const $name1: &str = $fragment;
        pub const $name2: &str =include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/templates/",  $fragment));
    };
}

