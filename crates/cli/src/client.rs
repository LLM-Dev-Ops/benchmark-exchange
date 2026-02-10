//! HTTP API client for the LLM Benchmark Exchange

use anyhow::{Context, Result};
use reqwest::{Client, RequestBuilder, Response};
use serde::{de::DeserializeOwned, Serialize};
use std::time::Duration;

use crate::config::Config;

/// API client for making HTTP requests to the LLM Benchmark Exchange API
#[derive(Clone)]
pub struct ApiClient {
    client: Client,
    base_url: String,
    auth_token: Option<String>,
    /// Agentics execution ID (sent as X-Execution-Id header when present)
    execution_id: Option<String>,
    /// Agentics parent span ID (sent as X-Parent-Span-Id header when present)
    parent_span_id: Option<String>,
}

impl ApiClient {
    /// Create a new API client from configuration
    pub fn from_config(config: &Config) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            client,
            base_url: config.api_endpoint.clone(),
            auth_token: config.auth_token.clone(),
            execution_id: None,
            parent_span_id: None,
        })
    }

    /// Create a new API client with custom settings
    pub fn new(base_url: String, auth_token: Option<String>) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            client,
            base_url,
            auth_token,
            execution_id: None,
            parent_span_id: None,
        })
    }

    /// Set agentics execution context headers.
    ///
    /// When set, these are sent as `X-Execution-Id` and `X-Parent-Span-Id`
    /// headers on every request, enabling the API to create execution spans.
    pub fn with_execution_context(
        mut self,
        execution_id: Option<String>,
        parent_span_id: Option<String>,
    ) -> Self {
        self.execution_id = execution_id;
        self.parent_span_id = parent_span_id;
        self
    }

    /// Add authentication and execution context headers to a request.
    fn add_headers(&self, builder: RequestBuilder) -> RequestBuilder {
        let builder = if let Some(token) = &self.auth_token {
            builder.bearer_auth(token)
        } else {
            builder
        };
        let builder = if let Some(exec_id) = &self.execution_id {
            builder.header("X-Execution-Id", exec_id)
        } else {
            builder
        };
        if let Some(parent_id) = &self.parent_span_id {
            builder.header("X-Parent-Span-Id", parent_id)
        } else {
            builder
        }
    }

    /// Make a GET request
    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);
        let builder = self.client.get(&url);
        let builder = self.add_headers(builder);

        let response = builder
            .send()
            .await
            .context("Failed to send GET request")?;

        self.handle_response(response).await
    }

    /// Make a POST request with JSON body
    pub async fn post<T: Serialize, R: DeserializeOwned>(
        &self,
        path: &str,
        body: &T,
    ) -> Result<R> {
        let url = format!("{}{}", self.base_url, path);
        let builder = self.client.post(&url).json(body);
        let builder = self.add_headers(builder);

        let response = builder
            .send()
            .await
            .context("Failed to send POST request")?;

        self.handle_response(response).await
    }

    /// Make a PUT request with JSON body
    pub async fn put<T: Serialize, R: DeserializeOwned>(
        &self,
        path: &str,
        body: &T,
    ) -> Result<R> {
        let url = format!("{}{}", self.base_url, path);
        let builder = self.client.put(&url).json(body);
        let builder = self.add_headers(builder);

        let response = builder
            .send()
            .await
            .context("Failed to send PUT request")?;

        self.handle_response(response).await
    }

    /// Make a DELETE request
    pub async fn delete<R: DeserializeOwned>(&self, path: &str) -> Result<R> {
        let url = format!("{}{}", self.base_url, path);
        let builder = self.client.delete(&url);
        let builder = self.add_headers(builder);

        let response = builder
            .send()
            .await
            .context("Failed to send DELETE request")?;

        self.handle_response(response).await
    }

    /// Make a DELETE request without expecting a response body
    pub async fn delete_no_content(&self, path: &str) -> Result<()> {
        let url = format!("{}{}", self.base_url, path);
        let builder = self.client.delete(&url);
        let builder = self.add_headers(builder);

        let response = builder
            .send()
            .await
            .context("Failed to send DELETE request")?;

        if response.status().is_success() {
            Ok(())
        } else {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Request failed with status {}: {}", status, error_text)
        }
    }

    /// Handle response and deserialize JSON
    async fn handle_response<T: DeserializeOwned>(&self, response: Response) -> Result<T> {
        let status = response.status();

        if status.is_success() {
            response
                .json::<T>()
                .await
                .context("Failed to deserialize response")
        } else {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Request failed with status {}: {}", status, error_text)
        }
    }

    /// Upload a file
    pub async fn upload_file<R: DeserializeOwned>(
        &self,
        path: &str,
        file_name: &str,
        file_content: Vec<u8>,
    ) -> Result<R> {
        let url = format!("{}{}", self.base_url, path);

        let part = reqwest::multipart::Part::bytes(file_content)
            .file_name(file_name.to_string());

        let form = reqwest::multipart::Form::new().part("file", part);

        let builder = self.client.post(&url).multipart(form);
        let builder = self.add_headers(builder);

        let response = builder
            .send()
            .await
            .context("Failed to upload file")?;

        self.handle_response(response).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = ApiClient::new("http://localhost:3000".to_string(), None);
        assert!(client.is_ok());
    }

    #[test]
    fn test_client_with_auth() {
        let client = ApiClient::new(
            "http://localhost:3000".to_string(),
            Some("test-token".to_string()),
        );
        assert!(client.is_ok());
    }
}
