


#[cfg(feature =  "single")]
mod redis_single_service;
#[cfg(feature =  "single")]
pub use redis_single_service::RedisService;


mod redis_mock_service;



#[cfg(feature =  "cluster")]
mod redis_cluster_service;
#[cfg(feature =  "cluster")]
pub use redis_cluster_service::RedisService;

