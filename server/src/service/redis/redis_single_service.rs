use axum::async_trait;
use bb8_redis::{
    bb8,
    redis::AsyncCommands,
    RedisConnectionManager,
};

use crate::service::redis::{RedisOperation, RedisPool, RedisService};

pub async fn init_pool(redis_uri: &str) -> anyhow::Result<RedisPool> {
    let manager = RedisConnectionManager::new(redis_uri)?;
    let pool = bb8::Pool::builder().build(manager).await?;

    Ok(pool)
}


impl  RedisService{
    pub async fn set(&self, key: &str, val: &str) -> anyhow::Result<()> {
        let mut conn = self.pool.get().await.unwrap();

        conn.set(key, val).await?;
        Ok(())
    }

    pub async fn get(&self, key: &str) -> anyhow::Result<String> {
        let mut conn = self.pool.get().await.unwrap();
        Ok(conn.get(key).await?)
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_all() -> anyhow::Result<()> {
        let redis_service = RedisService::new(vec!{"redis://127.0.0.1".to_string()}).await;
        redis_service.set("testkey", "testval").await?;

        let val = redis_service.get( "testkey").await?;
        assert_eq!(val, "testval");

        Ok(())
    }
}