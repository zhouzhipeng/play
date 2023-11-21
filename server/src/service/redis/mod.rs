use axum::async_trait;
use bb8_redis::redis::AsyncCommands;
use crate::config::Config;

#[cfg(ENV =  "dev")]
mod redis_single_service;


#[cfg(ENV =  "prod")]
mod redis_cluster_service;


#[cfg(ENV =  "dev")]
pub type RedisPool = bb8_redis::bb8::Pool<bb8_redis::RedisConnectionManager>;


pub struct RedisService{
    pool : RedisPool,
}

impl RedisService{
    pub async fn new(redis_uri: Vec<String>)->Self{
        Self{
            pool: init_redis_pool(redis_uri).await.unwrap(),
        }
    }
}


#[async_trait]
pub trait RedisOperation{
    async fn set(&self, key: &str, val: &str) -> anyhow::Result<()>;
    async fn get(&self, key: &str) -> anyhow::Result<String>;

}


async  fn init_redis_pool(redis_uri: Vec<String>)->anyhow::Result<RedisPool>{
    #[cfg(ENV =  "dev")]
    redis_single_service::init_pool(redis_uri.get(0).unwrap().as_str()).await
}
