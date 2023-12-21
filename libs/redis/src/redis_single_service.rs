use std::time::Duration;
use bb8_redis::{
    bb8,
    redis::AsyncCommands,
    RedisConnectionManager,
};


use crate::redis_mock_service;
use futures_util::StreamExt;
use tracing::{error, info};

pub type RedisPool = bb8::Pool<RedisConnectionManager>;

pub struct RedisService {
    pool: Option<RedisPool>,
    test_pool: Option<redis_mock_service::RedisService>,
    is_test: bool,
}

impl RedisService {

    pub async fn new(redis_uri: Vec<String>, is_test: bool) -> anyhow::Result<Self> {
        if is_test {
            let test_redis_service = redis_mock_service::RedisService::new(redis_uri).await.unwrap();
            Ok(Self {
                pool: None,
                test_pool: Some(test_redis_service),
                is_test,
            })
        } else {
            let manager = RedisConnectionManager::new(redis_uri.get(0).unwrap().as_str())?;
            let pool = bb8::Pool::builder()
                .connection_timeout(Duration::from_secs(1))
                .max_size(100)
                .build(manager).await?;


            let r = Self {
                pool: Some(pool),
                test_pool: None,
                is_test,
            };



            //start a subscribe task.
            // r.initialise_subscriptions();

            Ok(r)
        }
    }


    pub async fn set(&self, key: &str, val: &str) -> anyhow::Result<()> {
        if self.is_test {
            self.test_pool.as_ref().unwrap().set(key, val).await
        } else {
            let mut conn = self.pool.as_ref().unwrap().get().await?;

            conn.set(key, val).await?;
            Ok(())
        }
    }

    pub async fn get(&self, key: &str) -> anyhow::Result<String> {
        if self.is_test {
            self.test_pool.as_ref().unwrap().get(key).await
        } else {
            let mut conn = self.pool.as_ref().unwrap().get().await?;

            Ok(conn.get(key).await?)
        }
    }
    pub async fn publish(&self, channel: &str, message: &str) -> anyhow::Result<()> {
        if self.is_test {
            error!("not implemented 'publish' for test pool.");
            Ok(())
        } else {
            let mut conn = self.pool.as_ref().unwrap().get().await?;
            conn.publish(channel, message).await?;
            Ok(())
        }
    }

    // #[allow(dead_code)]
    // // This is the main bit here
    // fn initialise_subscriptions(&self) {
    //     let pool = self.pool.as_ref().unwrap().clone();
    //     tokio::task::spawn(async move {
    //         info!("Started task");
    //         let conn = bb8::Pool::dedicated_connection(&pool).await.unwrap();
    //         let mut pubsub = conn.into_pubsub();
    //         let subscribed = pubsub.subscribe("a").await;
    //         info!("Subscribed Response: {:?}", subscribed);
    //         while let Some(result) = pubsub.on_message().next().await {
    //             let payload = result.get_payload::<String>().unwrap();
    //
    //             info!("<MSG>: {}", payload);
    //
    //         }
    //     });
    // }
}


