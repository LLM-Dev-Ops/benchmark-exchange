//! Messaging module - Event pub/sub using Redis
//!
//! Provides event-driven messaging capabilities using Redis pub/sub
//! for domain events and inter-service communication.

use async_trait::async_trait;
use futures::StreamExt;
use redis::{aio::ConnectionManager, AsyncCommands, Client};
use serde::{de::DeserializeOwned, Serialize};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::{broadcast, mpsc, RwLock};
use tracing::{debug, error, info, instrument, warn};

use crate::{Error, Result};

/// Messaging configuration.
#[derive(Debug, Clone)]
pub struct MessagingConfig {
    /// Redis connection URL
    pub url: String,
    /// Channel prefix for all messages
    pub channel_prefix: String,
    /// Maximum message size in bytes
    pub max_message_size: usize,
    /// Message retention time (for streams)
    pub retention: Duration,
}

impl Default for MessagingConfig {
    fn default() -> Self {
        Self {
            url: "redis://localhost:6379".to_string(),
            channel_prefix: "llm-benchmark:events:".to_string(),
            max_message_size: 1024 * 1024, // 1MB
            retention: Duration::from_secs(86400 * 7), // 7 days
        }
    }
}

impl MessagingConfig {
    /// Create configuration from environment variables.
    pub fn from_env() -> Result<Self> {
        let url = std::env::var("REDIS_URL")
            .unwrap_or_else(|_| "redis://localhost:6379".to_string());

        let channel_prefix = std::env::var("EVENT_CHANNEL_PREFIX")
            .unwrap_or_else(|_| "llm-benchmark:events:".to_string());

        Ok(Self {
            url,
            channel_prefix,
            ..Default::default()
        })
    }
}

/// Event message wrapper.
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct EventMessage<T> {
    /// Unique message ID
    pub id: String,
    /// Event type name
    pub event_type: String,
    /// Event timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Event payload
    pub payload: T,
    /// Correlation ID for tracing
    pub correlation_id: Option<String>,
    /// Source service/component
    pub source: String,
}

impl<T: Serialize> EventMessage<T> {
    /// Create a new event message.
    pub fn new(event_type: impl Into<String>, payload: T, source: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::now_v7().to_string(),
            event_type: event_type.into(),
            timestamp: chrono::Utc::now(),
            payload,
            correlation_id: None,
            source: source.into(),
        }
    }

    /// Add a correlation ID.
    pub fn with_correlation_id(mut self, id: impl Into<String>) -> Self {
        self.correlation_id = Some(id.into());
        self
    }
}

/// Message publisher trait.
#[async_trait]
pub trait Publisher: Send + Sync {
    /// Publish a message to a channel.
    async fn publish<T: Serialize + Send + Sync>(
        &self,
        channel: &str,
        message: &EventMessage<T>,
    ) -> Result<()>;

    /// Publish to multiple channels.
    async fn publish_many<T: Serialize + Send + Sync>(
        &self,
        channels: &[&str],
        message: &EventMessage<T>,
    ) -> Result<()>;
}

/// Message subscriber trait.
#[async_trait]
pub trait Subscriber: Send + Sync {
    /// Subscribe to a channel and receive messages.
    async fn subscribe<T: DeserializeOwned + Send + 'static>(
        &self,
        channel: &str,
    ) -> Result<mpsc::Receiver<EventMessage<T>>>;

    /// Subscribe to multiple channels with a pattern.
    async fn psubscribe<T: DeserializeOwned + Send + 'static>(
        &self,
        pattern: &str,
    ) -> Result<mpsc::Receiver<(String, EventMessage<T>)>>;

    /// Unsubscribe from a channel.
    async fn unsubscribe(&self, channel: &str) -> Result<()>;
}

/// Redis-based messaging implementation.
pub struct RedisMessaging {
    client: Client,
    connection: ConnectionManager,
    config: MessagingConfig,
    subscriptions: Arc<RwLock<HashMap<String, broadcast::Sender<Vec<u8>>>>>,
}

impl RedisMessaging {
    /// Create a new Redis messaging instance.
    #[instrument(skip(config))]
    pub async fn new(config: MessagingConfig) -> Result<Self> {
        info!(url = %config.url, "Initializing Redis messaging");

        let client = Client::open(config.url.clone()).map_err(Error::Cache)?;

        let connection = ConnectionManager::new(client.clone())
            .await
            .map_err(Error::Cache)?;

        info!("Redis messaging initialized successfully");
        Ok(Self {
            client,
            connection,
            config,
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Get a connection manager clone.
    fn conn(&self) -> ConnectionManager {
        self.connection.clone()
    }

    /// Build the full channel name with prefix.
    fn full_channel(&self, channel: &str) -> String {
        format!("{}{}", self.config.channel_prefix, channel)
    }

    /// Check messaging health.
    #[instrument(skip(self))]
    pub async fn health_check(&self) -> Result<MessagingHealthStatus> {
        let start = std::time::Instant::now();

        let mut conn = self.conn();
        match redis::cmd("PING")
            .query_async::<_, String>(&mut conn)
            .await
        {
            Ok(response) if response == "PONG" => {
                Ok(MessagingHealthStatus {
                    healthy: true,
                    latency: start.elapsed(),
                    error: None,
                })
            }
            Ok(response) => {
                Ok(MessagingHealthStatus {
                    healthy: false,
                    latency: start.elapsed(),
                    error: Some(format!("Unexpected response: {}", response)),
                })
            }
            Err(e) => {
                warn!(error = %e, "Messaging health check failed");
                Ok(MessagingHealthStatus {
                    healthy: false,
                    latency: start.elapsed(),
                    error: Some(e.to_string()),
                })
            }
        }
    }

    /// Publish to Redis Stream (for persistent message queues).
    #[instrument(skip(self, message))]
    pub async fn stream_publish<T: Serialize + Send + Sync>(
        &self,
        stream: &str,
        message: &EventMessage<T>,
    ) -> Result<String> {
        let full_stream = self.full_channel(stream);
        let serialized = serde_json::to_string(message).map_err(Error::Serialization)?;
        let mut conn = self.conn();

        // Add to stream with auto-generated ID
        let id: String = redis::cmd("XADD")
            .arg(&full_stream)
            .arg("MAXLEN")
            .arg("~")
            .arg(100000) // Keep approximately 100k messages
            .arg("*")
            .arg("data")
            .arg(&serialized)
            .arg("type")
            .arg(&message.event_type)
            .query_async(&mut conn)
            .await
            .map_err(Error::Cache)?;

        debug!(stream = %stream, message_id = %id, "Message published to stream");
        Ok(id)
    }

    /// Read from Redis Stream with consumer groups.
    #[instrument(skip(self))]
    pub async fn stream_read<T: DeserializeOwned>(
        &self,
        stream: &str,
        group: &str,
        consumer: &str,
        count: usize,
        block_ms: Option<u64>,
    ) -> Result<Vec<(String, EventMessage<T>)>> {
        let full_stream = self.full_channel(stream);
        let mut conn = self.conn();

        // Ensure consumer group exists
        let _: std::result::Result<(), redis::RedisError> = redis::cmd("XGROUP")
            .arg("CREATE")
            .arg(&full_stream)
            .arg(group)
            .arg("$")
            .arg("MKSTREAM")
            .query_async(&mut conn)
            .await;

        // Read new messages
        let mut cmd = redis::cmd("XREADGROUP");
        cmd.arg("GROUP")
            .arg(group)
            .arg(consumer)
            .arg("COUNT")
            .arg(count);

        if let Some(ms) = block_ms {
            cmd.arg("BLOCK").arg(ms);
        }

        cmd.arg("STREAMS").arg(&full_stream).arg(">");

        let result: Option<Vec<(String, Vec<(String, HashMap<String, String>)>)>> =
            cmd.query_async(&mut conn).await.map_err(Error::Cache)?;

        let mut messages = Vec::new();

        if let Some(streams) = result {
            for (_stream_name, entries) in streams {
                for (id, fields) in entries {
                    if let Some(data) = fields.get("data") {
                        if let Ok(message) = serde_json::from_str::<EventMessage<T>>(data) {
                            messages.push((id, message));
                        }
                    }
                }
            }
        }

        Ok(messages)
    }

    /// Acknowledge processed messages.
    #[instrument(skip(self, message_ids))]
    pub async fn stream_ack(&self, stream: &str, group: &str, message_ids: &[&str]) -> Result<u64> {
        if message_ids.is_empty() {
            return Ok(0);
        }

        let full_stream = self.full_channel(stream);
        let mut conn = self.conn();

        let mut cmd = redis::cmd("XACK");
        cmd.arg(&full_stream).arg(group);
        for id in message_ids {
            cmd.arg(*id);
        }

        let acked: u64 = cmd.query_async(&mut conn).await.map_err(Error::Cache)?;
        debug!(stream = %stream, group = %group, acked = acked, "Messages acknowledged");
        Ok(acked)
    }

    /// Get pending messages for a consumer group.
    #[instrument(skip(self))]
    pub async fn stream_pending(
        &self,
        stream: &str,
        group: &str,
    ) -> Result<StreamPendingInfo> {
        let full_stream = self.full_channel(stream);
        let mut conn = self.conn();

        let result: (u64, Option<String>, Option<String>, Option<Vec<(String, u64)>>) =
            redis::cmd("XPENDING")
                .arg(&full_stream)
                .arg(group)
                .query_async(&mut conn)
                .await
                .map_err(Error::Cache)?;

        Ok(StreamPendingInfo {
            pending_count: result.0,
            smallest_id: result.1,
            largest_id: result.2,
            consumers: result.3.unwrap_or_default(),
        })
    }
}

#[async_trait]
impl Publisher for RedisMessaging {
    #[instrument(skip(self, message))]
    async fn publish<T: Serialize + Send + Sync>(
        &self,
        channel: &str,
        message: &EventMessage<T>,
    ) -> Result<()> {
        let full_channel = self.full_channel(channel);
        let serialized = serde_json::to_string(message).map_err(Error::Serialization)?;

        if serialized.len() > self.config.max_message_size {
            return Err(Error::Messaging(format!(
                "Message size {} exceeds maximum {}",
                serialized.len(),
                self.config.max_message_size
            )));
        }

        let mut conn = self.conn();
        let subscribers: i32 = conn
            .publish(&full_channel, &serialized)
            .await
            .map_err(Error::Cache)?;

        debug!(
            channel = %channel,
            message_id = %message.id,
            subscribers = subscribers,
            "Message published"
        );
        Ok(())
    }

    #[instrument(skip(self, message))]
    async fn publish_many<T: Serialize + Send + Sync>(
        &self,
        channels: &[&str],
        message: &EventMessage<T>,
    ) -> Result<()> {
        for channel in channels {
            self.publish(channel, message).await?;
        }
        Ok(())
    }
}

#[async_trait]
impl Subscriber for RedisMessaging {
    #[instrument(skip(self))]
    async fn subscribe<T: DeserializeOwned + Send + 'static>(
        &self,
        channel: &str,
    ) -> Result<mpsc::Receiver<EventMessage<T>>> {
        let full_channel = self.full_channel(channel);
        let (tx, rx) = mpsc::channel(100);

        let client = self.client.clone();
        let channel_owned = full_channel.clone();

        tokio::spawn(async move {
            match client.get_async_connection().await {
                Ok(conn) => {
                    let mut pubsub = conn.into_pubsub();
                    if let Err(e) = pubsub.subscribe(&channel_owned).await {
                        error!(error = %e, channel = %channel_owned, "Failed to subscribe");
                        return;
                    }

                    let mut stream = pubsub.on_message();
                    while let Some(msg) = stream.next().await {
                        let payload: String = match msg.get_payload() {
                            Ok(p) => p,
                            Err(e) => {
                                warn!(error = %e, "Failed to get message payload");
                                continue;
                            }
                        };

                        match serde_json::from_str::<EventMessage<T>>(&payload) {
                            Ok(event) => {
                                if tx.send(event).await.is_err() {
                                    // Receiver dropped, stop listening
                                    break;
                                }
                            }
                            Err(e) => {
                                warn!(error = %e, "Failed to deserialize message");
                            }
                        }
                    }
                }
                Err(e) => {
                    error!(error = %e, "Failed to get pubsub connection");
                }
            }
        });

        debug!(channel = %channel, "Subscribed to channel");
        Ok(rx)
    }

    #[instrument(skip(self))]
    async fn psubscribe<T: DeserializeOwned + Send + 'static>(
        &self,
        pattern: &str,
    ) -> Result<mpsc::Receiver<(String, EventMessage<T>)>> {
        let full_pattern = self.full_channel(pattern);
        let (tx, rx) = mpsc::channel(100);
        let prefix_len = self.config.channel_prefix.len();

        let client = self.client.clone();
        let pattern_owned = full_pattern.clone();

        tokio::spawn(async move {
            match client.get_async_connection().await {
                Ok(conn) => {
                    let mut pubsub = conn.into_pubsub();
                    if let Err(e) = pubsub.psubscribe(&pattern_owned).await {
                        error!(error = %e, pattern = %pattern_owned, "Failed to pattern subscribe");
                        return;
                    }

                    let mut stream = pubsub.on_message();
                    while let Some(msg) = stream.next().await {
                        let channel: String = msg.get_channel_name().to_string();
                        let short_channel = if channel.len() > prefix_len {
                            channel[prefix_len..].to_string()
                        } else {
                            channel.clone()
                        };

                        let payload: String = match msg.get_payload() {
                            Ok(p) => p,
                            Err(e) => {
                                warn!(error = %e, "Failed to get message payload");
                                continue;
                            }
                        };

                        match serde_json::from_str::<EventMessage<T>>(&payload) {
                            Ok(event) => {
                                if tx.send((short_channel, event)).await.is_err() {
                                    break;
                                }
                            }
                            Err(e) => {
                                warn!(error = %e, "Failed to deserialize message");
                            }
                        }
                    }
                }
                Err(e) => {
                    error!(error = %e, "Failed to get pubsub connection");
                }
            }
        });

        debug!(pattern = %pattern, "Subscribed to pattern");
        Ok(rx)
    }

    #[instrument(skip(self))]
    async fn unsubscribe(&self, channel: &str) -> Result<()> {
        // For pub/sub, unsubscription happens when the receiver is dropped
        // This is a no-op but kept for interface consistency
        debug!(channel = %channel, "Unsubscribe requested");
        Ok(())
    }
}

impl std::fmt::Debug for RedisMessaging {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RedisMessaging")
            .field("config", &self.config)
            .finish()
    }
}

/// Messaging health status.
#[derive(Debug, Clone)]
pub struct MessagingHealthStatus {
    /// Whether messaging is healthy
    pub healthy: bool,
    /// Query latency
    pub latency: Duration,
    /// Error message if unhealthy
    pub error: Option<String>,
}

/// Stream pending information.
#[derive(Debug, Clone)]
pub struct StreamPendingInfo {
    /// Number of pending messages
    pub pending_count: u64,
    /// Smallest pending message ID
    pub smallest_id: Option<String>,
    /// Largest pending message ID
    pub largest_id: Option<String>,
    /// Consumers and their pending counts
    pub consumers: Vec<(String, u64)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = MessagingConfig::default();
        assert_eq!(config.url, "redis://localhost:6379");
        assert_eq!(config.channel_prefix, "llm-benchmark:events:");
        assert_eq!(config.max_message_size, 1024 * 1024);
    }

    #[test]
    fn test_event_message_creation() {
        let message = EventMessage::new("test_event", "payload", "test_source");
        assert_eq!(message.event_type, "test_event");
        assert_eq!(message.source, "test_source");
        assert!(message.correlation_id.is_none());

        let message = message.with_correlation_id("corr-123");
        assert_eq!(message.correlation_id, Some("corr-123".to_string()));
    }

    #[test]
    fn test_full_channel() {
        let prefix = "test:events:";
        let channel = "benchmark.created";
        let full = format!("{}{}", prefix, channel);
        assert_eq!(full, "test:events:benchmark.created");
    }
}
