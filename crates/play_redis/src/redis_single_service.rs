use std::time::Duration;

use async_trait::async_trait;
use bb8_redis::{
    bb8,
    redis::AsyncCommands,
    RedisConnectionManager,
};

use shared::redis_api::RedisAPI;

pub type RedisPool = bb8::Pool<RedisConnectionManager>;

pub struct RedisService {
    pool: RedisPool,
}


#[async_trait]
impl RedisAPI for RedisService {
    async fn new(redis_uri: Vec<String>, is_test: bool) -> anyhow::Result<Self> {
        let manager = RedisConnectionManager::new(redis_uri.get(0).unwrap().as_str())?;
        let pool = bb8::Pool::builder()
            .connection_timeout(Duration::from_secs(1))
            .max_size(100)
            .build(manager).await?;


        let r = Self {
            pool,

        };


        //start a subscribe task.
        // r.initialise_subscriptions();

        Ok(r)
    }
    async fn set(&self, key: &str, val: &str) -> anyhow::Result<()> {
        let mut conn = self.pool.get().await?;

        conn.set(key, val).await?;
        Ok(())
    }

    async fn get(&self, key: &str) -> anyhow::Result<String> {
        let mut conn = self.pool.get().await?;

        Ok(conn.get(key).await?)
    }
    async fn publish(&self, channel: &str, message: &str) -> anyhow::Result<()> {
        let mut conn = self.pool.get().await?;
        conn.publish(channel, message).await?;
        Ok(())
    }
}


