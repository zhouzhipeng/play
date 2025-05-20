use std::env;
use tracing::subscriber::set_global_default;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, EnvFilter, Registry};
use tracing_subscriber::layer::SubscriberExt;
use crate::constants::DATA_DIR;

pub fn init_logger(print_console: bool) {

    let data_dir = env::var(DATA_DIR).unwrap();
    let file_appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY) // rotate log files once every hour
        .filename_prefix("play") // log file names will be prefixed with `myapp.`
        .filename_suffix("log") // log file names will be suffixed with `.log`
        .max_log_files(10)
        // .max_file_size(100*1024*1024 /*100MB*/)
        .build(data_dir) // try to build an appender that stores log files in `/var/log`
        .expect("initializing rolling file appender failed");

    let (writer, _guard) = tracing_appender::non_blocking(file_appender);

    // 创建文件层 (Layer)
    let file_layer = tracing_subscriber::fmt::layer()
        .with_file(true)
        .with_line_number(true)
        .with_thread_names(true)
        .pretty()
        .with_writer(writer)
        .with_ansi(true); // 禁用颜色，以避免文件中出现不必要的控制符

    // 创建控制台层 (Layer)
    let console_layer = tracing_subscriber::fmt::layer().with_ansi(true); // 控制台中启用颜色

    // 设置环境过滤器
    let env_filter = EnvFilter::from_default_env()
        .add_directive("info".parse().unwrap());

    // 构建组合订阅器 (Subscriber)
    let mut subscriber = Registry::default()
        .with(env_filter)
        .with(file_layer)
        .with(console_layer);

    if print_console {
        // 将构建好的订阅器作为全局默认的订阅器
        set_global_default(subscriber)
            .expect("Failed to set subscriber");


    }else{
        set_global_default(subscriber)
            .expect("Failed to set subscriber");

    }


    //
    // tracing_subscriber::fmt()
    //     .with_env_filter(env_filter)
    //     .with_file(true)
    //     .with_line_number(true)
    //     .with_thread_names(true)
    //     .pretty()
    //     .with_writer(writer)
    //     .with_writer(std::io::stderr)
    //     .finish()
    //     .init();


}