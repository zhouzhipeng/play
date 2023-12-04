use std::sync::Arc;
use std::thread;
use axum::Router;


use tracing::info;
use shared::constants::HOST;

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


pub struct AppState {
    pub template_service: TemplateService,
    pub db: DBPool,
    pub redis_service: RedisService,
    pub config: Config,
}



pub async fn init_app_state(config: Config, use_test_pool: bool) -> Arc<AppState> {


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


#[cfg(ENV="dev")]
pub fn start_window() -> wry::Result<()> {
    use wry::{
        application::{
            event::{Event, StartCause, WindowEvent},
            event_loop::{ControlFlow, EventLoop},
            window::WindowBuilder,
            dpi::{LogicalSize, Size},
            window::Icon,
        },
        webview::WebViewBuilder,

    };

    use image::ImageFormat;

    //icon
    let bytes: Vec<u8> = include_bytes!(file_path!("/static/icon.png")).to_vec();
    let imagebuffer = image::load_from_memory_with_format(&bytes, ImageFormat::Png).unwrap().into_rgba8();
    let (icon_width, icon_height) = imagebuffer.dimensions();
    let icon_rgba = imagebuffer.into_raw();

    let icon = Icon::from_rgba(icon_rgba, icon_width, icon_height).unwrap();

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Play")
        .with_inner_size(LogicalSize::new(1000, 600))
        .with_window_icon(Some(icon.clone()))
        .build(&event_loop)?;
    let _webview = WebViewBuilder::new(window)?
        .with_url(&format!("{}/static/wasm/index.html", HOST))?
        .build()?;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::NewEvents(StartCause::Init) => println!("Wry has started!"),
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            _ => (),
        }
    });
}

pub async fn start_server(server_port: u32, router : Router){
    info!("server start at port : {} ...", server_port);
    // run it with hyper on localhost:3000
    axum::Server::bind(&format!("0.0.0.0:{}", server_port).parse().unwrap())
        .serve(router.into_make_service())
        .await
        .unwrap();
}