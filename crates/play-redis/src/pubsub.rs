use crate::error::RedisError;
use redis::{Client, aio::PubSub};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc::{self, Receiver, Sender};
use futures_util::StreamExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub channel: String,
    pub payload: String,
}

pub struct PubSubClient {
    client: Client,
}

pub struct Subscriber {
    receiver: Receiver<Message>,
}

impl PubSubClient {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    pub async fn publish(&self, channel: &str, message: &str) -> Result<(), RedisError> {
        let mut conn = self.client.get_async_connection().await?;
        
        redis::cmd("PUBLISH")
            .arg(channel)
            .arg(message)
            .query_async(&mut conn)
            .await?;
            
        Ok(())
    }
    
    pub async fn subscribe<T: AsRef<str>>(&self, channels: &[T]) -> Result<Subscriber, RedisError> {
        let mut pubsub = self.client.get_async_connection().await?
            .into_pubsub();
            
        for channel in channels {
            pubsub.subscribe(channel.as_ref()).await?;
        }
        
        let (tx, rx) = mpsc::channel(100);
        
        tokio::spawn(async move {
            Self::pubsub_loop(pubsub, tx).await;
        });
        
        Ok(Subscriber { receiver: rx })
    }
    
    async fn pubsub_loop(mut pubsub: PubSub, tx: Sender<Message>) {
        let mut consecutive_errors = 0;
        
        // Get stream of messages
        let mut msg_stream = pubsub.on_message();
        
        // Process messages from the stream
        while let Some(msg) = msg_stream.next().await {
            consecutive_errors = 0; // Reset error counter on success
            
            let channel = msg.get_channel_name().to_string();
            let payload: String = match msg.get_payload() {
                Ok(payload) => payload,
                Err(e) => {
                    tracing::error!("Failed to get payload: {}", e);
                    continue;
                }
            };
            
            let message = Message { channel, payload };
            
            if tx.send(message).await.is_err() {
                tracing::error!("Receiver dropped, terminating pubsub loop");
                break;
            }
        }
        
        // If we reach here, the stream ended unexpectedly
        tracing::error!("PubSub message stream ended unexpectedly");
    }
}

impl Subscriber {
    pub async fn next_message(&mut self) -> Option<Message> {
        self.receiver.recv().await
    }
}