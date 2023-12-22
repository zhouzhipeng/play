use async_trait::async_trait;

#[async_trait]
pub trait RedisAPI {

    async fn set(&self, key: &str, val: &str) -> anyhow::Result<()>;

    async fn get(&self, key: &str) -> anyhow::Result<String>;

    async fn publish(&self, channel: &str, message: &str) -> anyhow::Result<()>{
        todo!()
    }
}
