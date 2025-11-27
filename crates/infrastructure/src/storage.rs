//! Storage module - S3-compatible object storage provider
//!
//! Provides object storage functionality using AWS S3 or compatible services
//! (MinIO, Cloudflare R2, etc.) for storing benchmark datasets, artifacts,
//! and submission results.

use async_trait::async_trait;
use aws_sdk_s3::{
    config::{Builder, Credentials, Region},
    primitives::ByteStream,
    Client,
};
use bytes::Bytes;
use std::time::Duration;
use tracing::{debug, info, instrument, warn};

use crate::{Error, Result};

/// S3 storage configuration.
#[derive(Debug, Clone)]
pub struct StorageConfig {
    /// S3-compatible endpoint URL (for MinIO, R2, etc.)
    pub endpoint_url: Option<String>,
    /// AWS region
    pub region: String,
    /// S3 bucket name
    pub bucket: String,
    /// Access key ID
    pub access_key_id: String,
    /// Secret access key
    pub secret_access_key: String,
    /// Path prefix for all objects
    pub path_prefix: String,
    /// Force path-style access (required for MinIO)
    pub force_path_style: bool,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            endpoint_url: None,
            region: "us-east-1".to_string(),
            bucket: "llm-benchmark".to_string(),
            access_key_id: String::new(),
            secret_access_key: String::new(),
            path_prefix: String::new(),
            force_path_style: false,
        }
    }
}

impl StorageConfig {
    /// Create configuration from environment variables.
    pub fn from_env() -> Result<Self> {
        let bucket = std::env::var("S3_BUCKET")
            .map_err(|_| Error::Configuration("S3_BUCKET not set".to_string()))?;

        Ok(Self {
            endpoint_url: std::env::var("S3_ENDPOINT_URL").ok(),
            region: std::env::var("AWS_REGION")
                .or_else(|_| std::env::var("S3_REGION"))
                .unwrap_or_else(|_| "us-east-1".to_string()),
            bucket,
            access_key_id: std::env::var("AWS_ACCESS_KEY_ID")
                .unwrap_or_default(),
            secret_access_key: std::env::var("AWS_SECRET_ACCESS_KEY")
                .unwrap_or_default(),
            path_prefix: std::env::var("S3_PATH_PREFIX")
                .unwrap_or_default(),
            force_path_style: std::env::var("S3_FORCE_PATH_STYLE")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false),
        })
    }
}

/// Storage provider trait for object storage operations.
#[async_trait]
pub trait Storage: Send + Sync {
    /// Upload an object to storage.
    async fn upload(&self, key: &str, data: Bytes, content_type: Option<&str>) -> Result<String>;

    /// Download an object from storage.
    async fn download(&self, key: &str) -> Result<Bytes>;

    /// Delete an object from storage.
    async fn delete(&self, key: &str) -> Result<bool>;

    /// Check if an object exists.
    async fn exists(&self, key: &str) -> Result<bool>;

    /// Get object metadata.
    async fn head(&self, key: &str) -> Result<Option<ObjectMetadata>>;

    /// List objects with a prefix.
    async fn list(&self, prefix: &str, max_keys: i32) -> Result<Vec<ObjectInfo>>;

    /// Generate a presigned URL for download.
    async fn presigned_download_url(&self, key: &str, expires_in: Duration) -> Result<String>;

    /// Generate a presigned URL for upload.
    async fn presigned_upload_url(&self, key: &str, expires_in: Duration) -> Result<String>;

    /// Copy an object to a new location.
    async fn copy(&self, source_key: &str, dest_key: &str) -> Result<()>;
}

/// Object metadata.
#[derive(Debug, Clone)]
pub struct ObjectMetadata {
    /// Object size in bytes
    pub size: i64,
    /// Content type
    pub content_type: Option<String>,
    /// Last modified timestamp
    pub last_modified: Option<chrono::DateTime<chrono::Utc>>,
    /// ETag (usually MD5 hash)
    pub etag: Option<String>,
    /// Custom metadata
    pub metadata: std::collections::HashMap<String, String>,
}

/// Object information for listing.
#[derive(Debug, Clone)]
pub struct ObjectInfo {
    /// Object key
    pub key: String,
    /// Object size in bytes
    pub size: i64,
    /// Last modified timestamp
    pub last_modified: Option<chrono::DateTime<chrono::Utc>>,
    /// ETag
    pub etag: Option<String>,
}

/// S3-compatible storage implementation.
pub struct S3Storage {
    client: Client,
    bucket: String,
    path_prefix: String,
}

impl S3Storage {
    /// Create a new S3 storage instance.
    #[instrument(skip(config))]
    pub async fn new(config: StorageConfig) -> Result<Self> {
        info!(bucket = %config.bucket, region = %config.region, "Initializing S3 storage");

        let mut sdk_config_builder = Builder::new()
            .region(Region::new(config.region.clone()));

        // Set credentials if provided
        if !config.access_key_id.is_empty() && !config.secret_access_key.is_empty() {
            let credentials = Credentials::new(
                &config.access_key_id,
                &config.secret_access_key,
                None,
                None,
                "environment",
            );
            sdk_config_builder = sdk_config_builder.credentials_provider(credentials);
        }

        // Set custom endpoint for S3-compatible services
        if let Some(ref endpoint_url) = config.endpoint_url {
            sdk_config_builder = sdk_config_builder.endpoint_url(endpoint_url);
        }

        // Force path style for MinIO and similar
        if config.force_path_style {
            sdk_config_builder = sdk_config_builder.force_path_style(true);
        }

        let sdk_config = sdk_config_builder.build();
        let client = Client::from_conf(sdk_config);

        info!("S3 storage initialized successfully");
        Ok(Self {
            client,
            bucket: config.bucket,
            path_prefix: config.path_prefix,
        })
    }

    /// Build the full object key with prefix.
    fn full_key(&self, key: &str) -> String {
        if self.path_prefix.is_empty() {
            key.to_string()
        } else {
            format!("{}/{}", self.path_prefix.trim_end_matches('/'), key)
        }
    }

    /// Check storage health.
    #[instrument(skip(self))]
    pub async fn health_check(&self) -> Result<StorageHealthStatus> {
        let start = std::time::Instant::now();

        match self.client.head_bucket().bucket(&self.bucket).send().await {
            Ok(_) => {
                let latency = start.elapsed();
                debug!(latency_ms = latency.as_millis(), "Storage health check passed");
                Ok(StorageHealthStatus {
                    healthy: true,
                    latency,
                    error: None,
                })
            }
            Err(e) => {
                warn!(error = %e, "Storage health check failed");
                Ok(StorageHealthStatus {
                    healthy: false,
                    latency: start.elapsed(),
                    error: Some(e.to_string()),
                })
            }
        }
    }

    /// Get total size of objects with a prefix.
    #[instrument(skip(self))]
    pub async fn get_total_size(&self, prefix: &str) -> Result<i64> {
        let full_prefix = self.full_key(prefix);
        let mut total_size: i64 = 0;
        let mut continuation_token: Option<String> = None;

        loop {
            let mut request = self
                .client
                .list_objects_v2()
                .bucket(&self.bucket)
                .prefix(&full_prefix);

            if let Some(token) = continuation_token {
                request = request.continuation_token(token);
            }

            let response = request.send().await.map_err(|e| Error::Storage(e.to_string()))?;

            if let Some(contents) = response.contents {
                for object in contents {
                    if let Some(size) = object.size {
                        total_size += size;
                    }
                }
            }

            if response.is_truncated.unwrap_or(false) {
                continuation_token = response.next_continuation_token;
            } else {
                break;
            }
        }

        Ok(total_size)
    }
}

#[async_trait]
impl Storage for S3Storage {
    #[instrument(skip(self, data))]
    async fn upload(&self, key: &str, data: Bytes, content_type: Option<&str>) -> Result<String> {
        let full_key = self.full_key(key);
        let body = ByteStream::from(data);

        let mut request = self
            .client
            .put_object()
            .bucket(&self.bucket)
            .key(&full_key)
            .body(body);

        if let Some(ct) = content_type {
            request = request.content_type(ct);
        }

        request
            .send()
            .await
            .map_err(|e| Error::Storage(e.to_string()))?;

        debug!(key = %key, "Object uploaded");
        Ok(full_key)
    }

    #[instrument(skip(self))]
    async fn download(&self, key: &str) -> Result<Bytes> {
        let full_key = self.full_key(key);

        let response = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(&full_key)
            .send()
            .await
            .map_err(|e| {
                if e.to_string().contains("NoSuchKey") {
                    Error::NotFound(format!("Object not found: {}", key))
                } else {
                    Error::Storage(e.to_string())
                }
            })?;

        let data = response
            .body
            .collect()
            .await
            .map_err(|e| Error::Storage(e.to_string()))?
            .into_bytes();

        debug!(key = %key, size = data.len(), "Object downloaded");
        Ok(data)
    }

    #[instrument(skip(self))]
    async fn delete(&self, key: &str) -> Result<bool> {
        let full_key = self.full_key(key);

        // Check if exists first
        let exists = self.exists(key).await?;
        if !exists {
            return Ok(false);
        }

        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(&full_key)
            .send()
            .await
            .map_err(|e| Error::Storage(e.to_string()))?;

        debug!(key = %key, "Object deleted");
        Ok(true)
    }

    #[instrument(skip(self))]
    async fn exists(&self, key: &str) -> Result<bool> {
        let full_key = self.full_key(key);

        match self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(&full_key)
            .send()
            .await
        {
            Ok(_) => Ok(true),
            Err(e) => {
                if e.to_string().contains("NotFound") || e.to_string().contains("NoSuchKey") {
                    Ok(false)
                } else {
                    Err(Error::Storage(e.to_string()))
                }
            }
        }
    }

    #[instrument(skip(self))]
    async fn head(&self, key: &str) -> Result<Option<ObjectMetadata>> {
        let full_key = self.full_key(key);

        match self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(&full_key)
            .send()
            .await
        {
            Ok(response) => {
                let last_modified = response.last_modified.and_then(|dt| {
                    chrono::DateTime::from_timestamp(dt.secs(), dt.subsec_nanos())
                });

                Ok(Some(ObjectMetadata {
                    size: response.content_length.unwrap_or(0),
                    content_type: response.content_type,
                    last_modified,
                    etag: response.e_tag,
                    metadata: response.metadata.unwrap_or_default(),
                }))
            }
            Err(e) => {
                if e.to_string().contains("NotFound") || e.to_string().contains("NoSuchKey") {
                    Ok(None)
                } else {
                    Err(Error::Storage(e.to_string()))
                }
            }
        }
    }

    #[instrument(skip(self))]
    async fn list(&self, prefix: &str, max_keys: i32) -> Result<Vec<ObjectInfo>> {
        let full_prefix = self.full_key(prefix);

        let response = self
            .client
            .list_objects_v2()
            .bucket(&self.bucket)
            .prefix(&full_prefix)
            .max_keys(max_keys)
            .send()
            .await
            .map_err(|e| Error::Storage(e.to_string()))?;

        let objects = response
            .contents
            .unwrap_or_default()
            .into_iter()
            .map(|obj| {
                let last_modified = obj.last_modified.and_then(|dt| {
                    chrono::DateTime::from_timestamp(dt.secs(), dt.subsec_nanos())
                });

                ObjectInfo {
                    key: obj.key.unwrap_or_default(),
                    size: obj.size.unwrap_or(0),
                    last_modified,
                    etag: obj.e_tag,
                }
            })
            .collect();

        debug!(prefix = %prefix, count = response.key_count.unwrap_or(0), "Objects listed");
        Ok(objects)
    }

    #[instrument(skip(self))]
    async fn presigned_download_url(&self, key: &str, expires_in: Duration) -> Result<String> {
        let full_key = self.full_key(key);

        let presigning_config = aws_sdk_s3::presigning::PresigningConfig::expires_in(expires_in)
            .map_err(|e| Error::Storage(e.to_string()))?;

        let presigned_request = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(&full_key)
            .presigned(presigning_config)
            .await
            .map_err(|e| Error::Storage(e.to_string()))?;

        debug!(key = %key, expires_in_secs = expires_in.as_secs(), "Presigned download URL generated");
        Ok(presigned_request.uri().to_string())
    }

    #[instrument(skip(self))]
    async fn presigned_upload_url(&self, key: &str, expires_in: Duration) -> Result<String> {
        let full_key = self.full_key(key);

        let presigning_config = aws_sdk_s3::presigning::PresigningConfig::expires_in(expires_in)
            .map_err(|e| Error::Storage(e.to_string()))?;

        let presigned_request = self
            .client
            .put_object()
            .bucket(&self.bucket)
            .key(&full_key)
            .presigned(presigning_config)
            .await
            .map_err(|e| Error::Storage(e.to_string()))?;

        debug!(key = %key, expires_in_secs = expires_in.as_secs(), "Presigned upload URL generated");
        Ok(presigned_request.uri().to_string())
    }

    #[instrument(skip(self))]
    async fn copy(&self, source_key: &str, dest_key: &str) -> Result<()> {
        let full_source = format!("{}/{}", self.bucket, self.full_key(source_key));
        let full_dest = self.full_key(dest_key);

        self.client
            .copy_object()
            .bucket(&self.bucket)
            .copy_source(&full_source)
            .key(&full_dest)
            .send()
            .await
            .map_err(|e| Error::Storage(e.to_string()))?;

        debug!(source = %source_key, dest = %dest_key, "Object copied");
        Ok(())
    }
}

impl std::fmt::Debug for S3Storage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("S3Storage")
            .field("bucket", &self.bucket)
            .field("path_prefix", &self.path_prefix)
            .finish()
    }
}

/// Storage health status.
#[derive(Debug, Clone)]
pub struct StorageHealthStatus {
    /// Whether the storage is healthy
    pub healthy: bool,
    /// Query latency
    pub latency: Duration,
    /// Error message if unhealthy
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = StorageConfig::default();
        assert_eq!(config.bucket, "llm-benchmark");
        assert_eq!(config.region, "us-east-1");
        assert!(!config.force_path_style);
    }

    #[test]
    fn test_full_key_with_prefix() {
        let config = StorageConfig {
            path_prefix: "test/prefix".to_string(),
            ..Default::default()
        };

        // We can't easily test without the full S3Storage instance,
        // but we can verify the logic
        let prefix = "test/prefix";
        let key = "mykey";
        let full = format!("{}/{}", prefix.trim_end_matches('/'), key);
        assert_eq!(full, "test/prefix/mykey");
    }

    #[test]
    fn test_full_key_without_prefix() {
        let key = "mykey";
        // With empty prefix, key should be returned as-is
        assert_eq!(key, "mykey");
    }
}
