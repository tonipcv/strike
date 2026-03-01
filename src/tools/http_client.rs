use reqwest::{Client, Method, Request, Response};
use anyhow::Result;
use std::time::Duration;
use std::collections::HashMap;
use tokio::sync::Semaphore;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct HttpClientConfig {
    pub rate_limit: u32,
    pub timeout_seconds: u64,
    pub pool_max_idle_per_host: usize,
    pub pool_idle_timeout_seconds: u64,
    pub tcp_keepalive_seconds: u64,
    pub http2_adaptive_window: bool,
}

impl Default for HttpClientConfig {
    fn default() -> Self {
        Self {
            rate_limit: 50,
            timeout_seconds: 30,
            pool_max_idle_per_host: 32,
            pool_idle_timeout_seconds: 90,
            tcp_keepalive_seconds: 60,
            http2_adaptive_window: true,
        }
    }
}

pub struct HttpClient {
    client: Client,
    rate_limiter: Arc<Semaphore>,
    default_headers: HashMap<String, String>,
}

impl HttpClient {
    pub fn new(rate_limit: u32, timeout_seconds: u64) -> Result<Self> {
        let config = HttpClientConfig {
            rate_limit,
            timeout_seconds,
            ..Default::default()
        };
        Self::with_config(config)
    }
    
    pub fn with_config(config: HttpClientConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .danger_accept_invalid_certs(false)
            .redirect(reqwest::redirect::Policy::limited(10))
            .pool_max_idle_per_host(config.pool_max_idle_per_host)
            .pool_idle_timeout(Duration::from_secs(config.pool_idle_timeout_seconds))
            .tcp_keepalive(Duration::from_secs(config.tcp_keepalive_seconds))
            .build()?;

        Ok(Self {
            client,
            rate_limiter: Arc::new(Semaphore::new(config.rate_limit as usize)),
            default_headers: HashMap::new(),
        })
    }

    pub fn with_headers(mut self, headers: HashMap<String, String>) -> Self {
        self.default_headers = headers;
        self
    }

    pub async fn get(&self, url: &str) -> Result<Response> {
        self.request(Method::GET, url, None).await
    }

    pub async fn post(&self, url: &str, body: Option<String>) -> Result<Response> {
        self.request(Method::POST, url, body).await
    }

    pub async fn put(&self, url: &str, body: Option<String>) -> Result<Response> {
        self.request(Method::PUT, url, body).await
    }

    pub async fn delete(&self, url: &str) -> Result<Response> {
        self.request(Method::DELETE, url, None).await
    }

    async fn request(&self, method: Method, url: &str, body: Option<String>) -> Result<Response> {
        let _permit = self.rate_limiter.acquire().await?;

        let mut request = self.client.request(method, url);

        for (key, value) in &self.default_headers {
            request = request.header(key, value);
        }

        if let Some(body_content) = body {
            request = request.body(body_content);
        }

        let response = request.send().await?;
        Ok(response)
    }

    pub async fn request_with_headers(
        &self,
        method: Method,
        url: &str,
        headers: HashMap<String, String>,
        body: Option<String>,
    ) -> Result<Response> {
        let _permit = self.rate_limiter.acquire().await?;

        let mut request = self.client.request(method, url);

        for (key, value) in headers {
            request = request.header(key, value);
        }

        if let Some(body_content) = body {
            request = request.body(body_content);
        }

        let response = request.send().await?;
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_http_client_creation() {
        let client = HttpClient::new(50, 30);
        assert!(client.is_ok());
    }
    
    #[tokio::test]
    async fn test_http_client_with_config() {
        let config = HttpClientConfig {
            rate_limit: 100,
            timeout_seconds: 60,
            pool_max_idle_per_host: 64,
            pool_idle_timeout_seconds: 120,
            tcp_keepalive_seconds: 30,
            http2_adaptive_window: true,
        };
        
        let client = HttpClient::with_config(config);
        assert!(client.is_ok());
    }
    
    #[test]
    fn test_http_client_config_default() {
        let config = HttpClientConfig::default();
        assert_eq!(config.rate_limit, 50);
        assert_eq!(config.timeout_seconds, 30);
        assert_eq!(config.pool_max_idle_per_host, 32);
        assert_eq!(config.pool_idle_timeout_seconds, 90);
        assert_eq!(config.tcp_keepalive_seconds, 60);
        assert!(config.http2_adaptive_window);
    }
}
