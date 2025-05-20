mod connection;
mod error;
mod pubsub;

pub use connection::{RedisClient, RedisConnection};
pub use error::RedisError;
pub use pubsub::{Message, PubSubClient, Subscriber};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_redis_connection() {
        // Add tests when implementation is complete
    }
}