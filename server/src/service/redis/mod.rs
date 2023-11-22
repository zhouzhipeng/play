use axum::async_trait;
use crate::config::Config;

#[cfg(ENV =  "dev")]
mod redis_single_service;
#[cfg(ENV =  "dev")]
pub use redis_single_service::RedisService;
#[cfg(ENV =  "dev")]
mod redis_mock_service;


#[cfg(ENV =  "prod")]
mod redis_cluster_service;


