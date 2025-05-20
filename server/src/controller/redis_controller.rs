#[cfg(feature = "play-redis")]
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{sse::{Event, Sse}, Html, IntoResponse},
    routing::{get, post},
    Json, Router,
};
#[cfg(feature = "play-redis")]
use futures_util::stream::Stream;
#[cfg(feature = "play-redis")]
use play_redis::{Message, PubSubClient, RedisClient, RedisConnection};
#[cfg(feature = "play-redis")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "play-redis")]
use std::sync::Arc;
#[cfg(feature = "play-redis")]
use std::time::Duration;
#[cfg(feature = "play-redis")]
use tokio::sync::Mutex;
#[cfg(feature = "play-redis")]
use tracing::error;
#[cfg(feature = "play-redis")]
use futures_util::StreamExt;
#[cfg(feature = "play-redis")]
use futures_util::stream;

#[cfg(feature = "play-redis")]
#[derive(Clone)]
pub struct RedisState {
    pub client: RedisClient,
    pub connection: Arc<RedisConnection>,
}

#[cfg(feature = "play-redis")]
#[derive(Deserialize)]
pub struct RedisSetRequest {
    key: String,
    value: String,
    #[serde(default)]
    ttl_seconds: Option<u64>,
}

#[cfg(feature = "play-redis")]
#[derive(Deserialize)]
pub struct RedisGetQuery {
    key: String,
}

#[cfg(feature = "play-redis")]
#[derive(Serialize)]
pub struct RedisGetResponse {
    key: String,
    value: Option<String>,
}

#[cfg(feature = "play-redis")]
#[derive(Deserialize)]
pub struct RedisPubRequest {
    channel: String,
    message: String,
}

#[cfg(feature = "play-redis")]
#[derive(Deserialize)]
pub struct RedisSubQuery {
    channels: String, // Comma-separated list of channels
}

#[cfg(feature = "play-redis")]
pub async fn init_redis_client(config: Option<&crate::config::Config>) -> Result<RedisState, anyhow::Error> {
    // Use configured Redis URL or default to localhost
    let redis_url = match config {
        Some(cfg) if cfg.redis_url.is_some() => cfg.redis_url.as_ref().unwrap().as_str(),
        _ => "redis://127.0.0.1:6379"
    };
    
    tracing::info!("Connecting to Redis at: {}", redis_url);
    
    let connection = RedisConnection::new(redis_url)?;
    let client = connection.create_client().await?;
    
    // Verify connection with ping
    let ping_result = client.ping().await;
    if let Err(ref e) = ping_result {
        tracing::error!("Redis ping failed: {}", e);
        return Err(anyhow::anyhow!("Redis connection test failed: {}", e));
    }
    
    tracing::info!("Redis connection established successfully");
    
    Ok(RedisState {
        client,
        connection: Arc::new(connection),
    })
}

#[cfg(feature = "play-redis")]
pub fn routes() -> Router<Arc<Mutex<RedisState>>> {
    Router::new()
        .route("/redis/get", get(get_value))
        .route("/redis/set", post(set_value))
        .route("/redis/publish", post(publish_message))
        .route("/redis/subscribe", get(subscribe_sse))
        .route("/redis", get(redis_manager))
}

#[cfg(feature = "play-redis")]
pub fn init() -> Router<Arc<crate::AppState>> {
    Router::new()
}

#[cfg(feature = "play-redis")]
async fn redis_manager() -> impl IntoResponse {
    let html = include_str!("../../static/redis-manager.html");
    Html(html.to_string())
}

#[cfg(feature = "play-redis")]
async fn get_value(
    State(state): State<Arc<Mutex<RedisState>>>,
    Query(query): Query<RedisGetQuery>,
) -> Result<Json<RedisGetResponse>, StatusCode> {
    let redis_state = state.lock().await;
    
    match redis_state.client.get::<String>(&query.key).await {
        Ok(value) => Ok(Json(RedisGetResponse { 
            key: query.key, 
            value 
        })),
        Err(err) => {
            error!("Redis get error: {}", err);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[cfg(feature = "play-redis")]
async fn set_value(
    State(state): State<Arc<Mutex<RedisState>>>,
    Json(request): Json<RedisSetRequest>,
) -> StatusCode {
    let redis_state = state.lock().await;
    
    let expiry = request.ttl_seconds.map(|secs| Duration::from_secs(secs));
    
    match redis_state.client.set(&request.key, &request.value, expiry).await {
        Ok(_) => StatusCode::OK,
        Err(err) => {
            error!("Redis set error: {}", err);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

#[cfg(feature = "play-redis")]
async fn publish_message(
    State(state): State<Arc<Mutex<RedisState>>>,
    Json(request): Json<RedisPubRequest>,
) -> StatusCode {
    let redis_state = state.lock().await;
    let client = redis_state.connection.client().clone();
    let pubsub_client = PubSubClient::new(client);
    
    match pubsub_client.publish(&request.channel, &request.message).await {
        Ok(_) => StatusCode::OK,
        Err(err) => {
            error!("Redis publish error: {}", err);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

#[cfg(feature = "play-redis")]
async fn subscribe_sse(
    State(state): State<Arc<Mutex<RedisState>>>,
    Query(query): Query<RedisSubQuery>,
) -> Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>> {
    let redis_state = state.lock().await;
    let client = redis_state.connection.client().clone();
    let pubsub_client = PubSubClient::new(client);
    
    let channels: Vec<String> = query.channels.split(',').map(String::from).collect();
    
    tracing::info!("Subscribing to Redis channels: {:?}", channels);
    
    // Use stream adapter pattern
    let stream = stream::unfold(
        (pubsub_client, channels, true, None),
        |(client, channels, is_first, mut subscriber)| async move {
            // On first call, send connection event
            if is_first {
                let connect_event = Event::default()
                    .data("Connected to Redis PubSub")
                    .event("connect");
                
                return Some((
                    Ok(connect_event),
                    (client, channels, false, None),
                ));
            }
            
            // Try to get or create subscriber
            if subscriber.is_none() {
                match client.subscribe(&channels).await {
                    Ok(sub) => {
                        subscriber = Some(sub);
                    }
                    Err(err) => {
                        error!("Redis subscribe error: {}", err);
                        
                        let error_event = Event::default()
                            .data(format!("Error: {}", err))
                            .event("error");
                        
                        // Return error event and end the stream
                        return Some((
                            Ok(error_event),
                            (client, channels, false, None),
                        ));
                    }
                }
            }
            
            if let Some(ref mut sub) = subscriber {
                // Try to get next message
                if let Some(msg) = sub.next_message().await {
                    // Special handling for error channel
                    let event = if msg.channel == "error" {
                        Event::default()
                            .data(msg.payload)
                            .event("error")
                    } else {
                        Event::default()
                            .data(msg.payload)
                            .event(msg.channel)
                    };
                    
                    return Some((
                        Ok(event),
                        (client, channels, false, subscriber),
                    ));
                } else {
                    // Subscriber closed, send disconnect event and end stream
                    let disconnect_event = Event::default()
                        .data("Disconnected from Redis PubSub")
                        .event("disconnect");
                    
                    return Some((
                        Ok(disconnect_event),
                        (client, channels, false, None),
                    ));
                }
            }
            
            // Shouldn't reach here, but just in case
            None
        },
    );
    
    Sse::new(stream)
}

// 添加空实现，以便在未启用 play-redis 特性时仍然可以编译
#[cfg(not(feature = "play-redis"))]
pub fn init() -> axum::Router<std::sync::Arc<crate::AppState>> {
    axum::Router::new()
}