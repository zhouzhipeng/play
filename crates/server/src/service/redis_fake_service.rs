use anyhow::bail;
use async_trait::async_trait;

use shared::redis_api::RedisAPI;

pub struct RedisFakeService {}


#[async_trait]
impl RedisAPI for RedisFakeService {
    async fn new(redis_uri: Vec<String>, is_test: bool) -> anyhow::Result<Self>{
        Ok(Self {})
    }

    async fn set(&self, key: &str, val: &str) -> anyhow::Result<()> {
        bail!("not implemented!")
    }

    async fn get(&self, key: &str) -> anyhow::Result<String> {
        bail!("not implemented!")
    }
}