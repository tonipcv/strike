use reqwest::{Client, Method, Request, Response};
use anyhow::Result;
use std::time::Duration;
use std::collections::HashMap;
use tokio::sync::Semaphore;
use std::sync::Arc;

pub struct HttpClient {
    client: Client,
    rate_limiter: Arc<Semaphore>,
    default_headers: HashMap<String, String>,
}

impl HttpClient {
    pub fn new(rate_limit: u32, timeout_seconds: u64) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_seconds))
            .danger_accept_invalid_certs(false)
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()?;

        Ok(Self {
            client,
            rate_limiter: Arc::new(Semaphore::new(rate_limit as usize)),
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
}
