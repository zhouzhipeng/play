
use dashmap::DashMap;

pub type RedisPool = DashMap<String, String>;

pub struct RedisService {
    pub pool: RedisPool,
}

impl RedisService {
    pub async fn new(_redis_uri: Vec<String>) -> anyhow::Result<Self> {

        Ok(Self {
            pool: DashMap::new(),
        })
    }


    pub async fn set(&self, key: &str, val: &str) -> anyhow::Result<()> {
        self.pool.insert(key.to_string(), val.to_string());
        println!("redis mock set");
        Ok(())
    }

    pub async fn get(&self, key: &str) -> anyhow::Result<String> {
        println!("redis mock get");
        Ok(self.pool.get(key).unwrap().value().to_string())
    }
}

