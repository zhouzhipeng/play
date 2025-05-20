use crate::error::RedisError;
use redis::{Client, aio::ConnectionManager};
use std::sync::Arc;
use std::time::Duration;

pub struct RedisConnection {
    client: Client,
    connection_string: String,
}

#[derive(Clone)]
pub struct RedisClient {
    connection_manager: ConnectionManager,
}

impl RedisConnection {
    pub fn new(connection_string: &str) -> Result<Self, RedisError> {
        let client = redis::Client::open(connection_string)
            .map_err(|e| RedisError::ConnectionError(e.to_string()))?;
        
        Ok(Self {
            client,
            connection_string: connection_string.to_string(),
        })
    }
    
    pub async fn create_client(&self) -> Result<RedisClient, RedisError> {
        let connection_manager = ConnectionManager::new(self.client.clone())
            .await
            .map_err(|e| RedisError::ConnectionError(e.to_string()))?;
        
        Ok(RedisClient {
            connection_manager,
        })
    }
    
    pub fn connection_string(&self) -> &str {
        &self.connection_string
    }
    
    pub fn client(&self) -> &Client {
        &self.client
    }
}

impl RedisClient {
    // Connection health check
    pub async fn ping(&self) -> Result<(), RedisError> {
        let mut con = self.connection_manager.clone();
        
        let result: String = redis::cmd("PING")
            .query_async(&mut con)
            .await?;
            
        if result != "PONG" {
            return Err(RedisError::ConnectionError(format!("Unexpected PING response: {}", result)));
        }
        
        Ok(())
    }

    // Basic key-value operations
    pub async fn set<T: serde::Serialize>(&self, key: &str, value: &T, expiry: Option<Duration>) -> Result<(), RedisError> {
        let serialized = serde_json::to_string(value)?;
        
        let mut con = self.connection_manager.clone();
        
        match expiry {
            Some(duration) => {
                redis::cmd("SET")
                    .arg(key)
                    .arg(serialized)
                    .arg("PX")
                    .arg(duration.as_millis() as u64)
                    .query_async(&mut con)
                    .await?
            },
            None => {
                redis::cmd("SET")
                    .arg(key)
                    .arg(serialized)
                    .query_async(&mut con)
                    .await?
            }
        }
        
        Ok(())
    }
    
    pub async fn get<T: serde::de::DeserializeOwned>(&self, key: &str) -> Result<Option<T>, RedisError> {
        let mut con = self.connection_manager.clone();
        
        let result: Option<String> = redis::cmd("GET")
            .arg(key)
            .query_async(&mut con)
            .await?;
        
        match result {
            Some(data) => {
                let deserialized = serde_json::from_str(&data)?;
                Ok(Some(deserialized))
            },
            None => Ok(None),
        }
    }
    
    pub async fn delete(&self, key: &str) -> Result<bool, RedisError> {
        let mut con = self.connection_manager.clone();
        
        let result: i32 = redis::cmd("DEL")
            .arg(key)
            .query_async(&mut con)
            .await?;
        
        Ok(result > 0)
    }
    
    // List operations
    pub async fn list_push<T: serde::Serialize>(&self, key: &str, value: &T, prepend: bool) -> Result<i64, RedisError> {
        let serialized = serde_json::to_string(value)?;
        let mut con = self.connection_manager.clone();
        
        let cmd_name = if prepend { "LPUSH" } else { "RPUSH" };
        
        let result: i64 = redis::cmd(cmd_name)
            .arg(key)
            .arg(serialized)
            .query_async(&mut con)
            .await?;
            
        Ok(result)
    }
    
    pub async fn list_range<T: serde::de::DeserializeOwned>(&self, key: &str, start: isize, stop: isize) -> Result<Vec<T>, RedisError> {
        let mut con = self.connection_manager.clone();
        
        let raw_values: Vec<String> = redis::cmd("LRANGE")
            .arg(key)
            .arg(start)
            .arg(stop)
            .query_async(&mut con)
            .await?;
        
        let mut result = Vec::with_capacity(raw_values.len());
        for val in raw_values {
            let deserialized = serde_json::from_str(&val)?;
            result.push(deserialized);
        }
            
        Ok(result)
    }
    
    // Hash operations
    pub async fn hash_set<T: serde::Serialize>(&self, key: &str, field: &str, value: &T) -> Result<bool, RedisError> {
        let serialized = serde_json::to_string(value)?;
        let mut con = self.connection_manager.clone();
        
        let result: i32 = redis::cmd("HSET")
            .arg(key)
            .arg(field)
            .arg(serialized)
            .query_async(&mut con)
            .await?;
            
        Ok(result > 0)
    }
    
    pub async fn hash_get<T: serde::de::DeserializeOwned>(&self, key: &str, field: &str) -> Result<Option<T>, RedisError> {
        let mut con = self.connection_manager.clone();
        
        let result: Option<String> = redis::cmd("HGET")
            .arg(key)
            .arg(field)
            .query_async(&mut con)
            .await?;
            
        match result {
            Some(data) => {
                let deserialized = serde_json::from_str(&data)?;
                Ok(Some(deserialized))
            },
            None => Ok(None),
        }
    }
    
    pub async fn hash_getall<T: serde::de::DeserializeOwned>(&self, key: &str) -> Result<Vec<(String, T)>, RedisError> {
        let mut con = self.connection_manager.clone();
        
        let result: Vec<(String, String)> = redis::cmd("HGETALL")
            .arg(key)
            .query_async(&mut con)
            .await?;
            
        let mut deserialized = Vec::with_capacity(result.len());
        for (field, value) in result {
            deserialized.push((field, serde_json::from_str(&value)?));
        }
            
        Ok(deserialized)
    }
    
    // Set operations
    pub async fn set_add<T: serde::Serialize>(&self, key: &str, value: &T) -> Result<bool, RedisError> {
        let serialized = serde_json::to_string(value)?;
        let mut con = self.connection_manager.clone();
        
        let result: i32 = redis::cmd("SADD")
            .arg(key)
            .arg(serialized)
            .query_async(&mut con)
            .await?;
            
        Ok(result > 0)
    }
    
    pub async fn set_members<T: serde::de::DeserializeOwned>(&self, key: &str) -> Result<Vec<T>, RedisError> {
        let mut con = self.connection_manager.clone();
        
        let results: Vec<String> = redis::cmd("SMEMBERS")
            .arg(key)
            .query_async(&mut con)
            .await?;
        
        let mut deserialized = Vec::with_capacity(results.len());
        for item in results {
            deserialized.push(serde_json::from_str(&item)?);
        }
        
        Ok(deserialized)
    }
    
    pub async fn set_is_member<T: serde::Serialize>(&self, key: &str, value: &T) -> Result<bool, RedisError> {
        let serialized = serde_json::to_string(value)?;
        let mut con = self.connection_manager.clone();
        
        let result: i32 = redis::cmd("SISMEMBER")
            .arg(key)
            .arg(serialized)
            .query_async(&mut con)
            .await?;
            
        Ok(result == 1)
    }
    
    // Key management operations
    pub async fn set_expiry(&self, key: &str, seconds: u64) -> Result<bool, RedisError> {
        let mut con = self.connection_manager.clone();
        
        let result: i32 = redis::cmd("EXPIRE")
            .arg(key)
            .arg(seconds)
            .query_async(&mut con)
            .await?;
            
        Ok(result > 0)
    }
    
    pub async fn get_ttl(&self, key: &str) -> Result<i64, RedisError> {
        let mut con = self.connection_manager.clone();
        
        let result: i64 = redis::cmd("TTL")
            .arg(key)
            .query_async(&mut con)
            .await?;
            
        Ok(result)
    }
    
    pub async fn exists(&self, key: &str) -> Result<bool, RedisError> {
        let mut con = self.connection_manager.clone();
        
        let result: i32 = redis::cmd("EXISTS")
            .arg(key)
            .query_async(&mut con)
            .await?;
            
        Ok(result > 0)
    }
    
    pub async fn scan_keys(&self, pattern: &str, count: Option<u32>) -> Result<Vec<String>, RedisError> {
        let mut con = self.connection_manager.clone();
        let mut keys = Vec::new();
        let mut cursor = 0;
        
        loop {
            let mut cmd = redis::cmd("SCAN");
            cmd.arg(cursor).arg("MATCH").arg(pattern);
            
            if let Some(count_val) = count {
                cmd.arg("COUNT").arg(count_val);
            }
            
            let (next_cursor, mut chunk): (u64, Vec<String>) = cmd.query_async(&mut con).await?;
            keys.append(&mut chunk);
            
            cursor = next_cursor;
            if cursor == 0 {
                break;
            }
        }
        
        Ok(keys)
    }
}