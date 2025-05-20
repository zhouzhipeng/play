use thiserror::Error;

#[derive(Error, Debug)]
pub enum RedisError {
    #[error("Redis connection error: {0}")]
    ConnectionError(String),
    
    #[error("Redis operation error: {0}")]
    OperationError(String),
    
    #[error("Redis publish/subscribe error: {0}")]
    PubSubError(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    #[error("Internal error: {0}")]
    InternalError(String),
}

impl From<redis::RedisError> for RedisError {
    fn from(err: redis::RedisError) -> Self {
        RedisError::OperationError(err.to_string())
    }
}

impl From<serde_json::Error> for RedisError {
    fn from(err: serde_json::Error) -> Self {
        RedisError::SerializationError(err.to_string())
    }
}