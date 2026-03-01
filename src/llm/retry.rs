use anyhow::{Context, Result};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, warn};

#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub exponential_base: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay_ms: 1000,
            max_delay_ms: 8000,
            exponential_base: 2.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RetryStrategy {
    config: RetryConfig,
}

impl RetryStrategy {
    pub fn new(config: RetryConfig) -> Self {
        Self { config }
    }
    
    pub fn default() -> Self {
        Self::new(RetryConfig::default())
    }
    
    pub async fn execute_with_retry<F, Fut, T, E>(&self, mut operation: F) -> Result<T>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
        E: std::fmt::Display + std::fmt::Debug,
    {
        let mut attempt = 1;
        
        loop {
            match operation().await {
                Ok(result) => {
                    if attempt > 1 {
                        debug!("Operation succeeded on attempt {}/{}", attempt, self.config.max_attempts);
                    }
                    return Ok(result);
                }
                Err(err) => {
                    if attempt >= self.config.max_attempts {
                        warn!(
                            "Operation failed after {} attempts. Final error: {}",
                            attempt, err
                        );
                        return Err(anyhow::anyhow!(
                            "Operation failed after {} attempts: {}",
                            attempt,
                            err
                        ));
                    }
                    
                    if !Self::is_retryable_error(&format!("{:?}", err)) {
                        warn!("Non-retryable error encountered: {}", err);
                        return Err(anyhow::anyhow!("Non-retryable error: {}", err));
                    }
                    
                    let delay = self.calculate_delay(attempt);
                    warn!(
                        "Attempt {}/{} failed: {}. Retrying in {}ms...",
                        attempt,
                        self.config.max_attempts,
                        err,
                        delay.as_millis()
                    );
                    
                    sleep(delay).await;
                    attempt += 1;
                }
            }
        }
    }
    
    fn calculate_delay(&self, attempt: u32) -> Duration {
        let delay_ms = (self.config.initial_delay_ms as f64
            * self.config.exponential_base.powi((attempt - 1) as i32))
            .min(self.config.max_delay_ms as f64) as u64;
        
        Duration::from_millis(delay_ms)
    }
    
    fn is_retryable_error(error_msg: &str) -> bool {
        let retryable_patterns = [
            "timeout",
            "connection",
            "network",
            "rate limit",
            "429",
            "500",
            "502",
            "503",
            "504",
            "temporary",
            "unavailable",
            "overloaded",
        ];
        
        let error_lower = error_msg.to_lowercase();
        
        for pattern in &retryable_patterns {
            if error_lower.contains(pattern) {
                return true;
            }
        }
        
        let non_retryable_patterns = [
            "401",
            "403",
            "404",
            "invalid api key",
            "authentication",
            "authorization",
            "not found",
            "bad request",
            "400",
        ];
        
        for pattern in &non_retryable_patterns {
            if error_lower.contains(pattern) {
                return false;
            }
        }
        
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_retry_succeeds_on_first_attempt() {
        let strategy = RetryStrategy::default();
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();
        
        let result = strategy.execute_with_retry(|| {
            let c = counter_clone.clone();
            async move {
                c.fetch_add(1, Ordering::SeqCst);
                Ok::<_, anyhow::Error>("success")
            }
        }).await;
        
        assert!(result.is_ok());
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }
    
    #[tokio::test]
    async fn test_retry_succeeds_on_second_attempt() {
        let strategy = RetryStrategy::default();
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();
        
        let result = strategy.execute_with_retry(|| {
            let c = counter_clone.clone();
            async move {
                let count = c.fetch_add(1, Ordering::SeqCst);
                if count == 0 {
                    Err(anyhow::anyhow!("Temporary network error"))
                } else {
                    Ok("success")
                }
            }
        }).await;
        
        assert!(result.is_ok());
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }
    
    #[tokio::test]
    async fn test_retry_fails_after_max_attempts() {
        let config = RetryConfig {
            max_attempts: 3,
            initial_delay_ms: 10,
            max_delay_ms: 100,
            exponential_base: 2.0,
        };
        let strategy = RetryStrategy::new(config);
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();
        
        let result = strategy.execute_with_retry(|| {
            let c = counter_clone.clone();
            async move {
                c.fetch_add(1, Ordering::SeqCst);
                Err::<String, _>(anyhow::anyhow!("Persistent timeout error"))
            }
        }).await;
        
        assert!(result.is_err());
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }
    
    #[tokio::test]
    async fn test_non_retryable_error_fails_immediately() {
        let strategy = RetryStrategy::default();
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();
        
        let result = strategy.execute_with_retry(|| {
            let c = counter_clone.clone();
            async move {
                c.fetch_add(1, Ordering::SeqCst);
                Err::<String, _>(anyhow::anyhow!("401 Invalid API key"))
            }
        }).await;
        
        assert!(result.is_err());
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }
    
    #[test]
    fn test_calculate_delay_exponential() {
        let strategy = RetryStrategy::default();
        
        let delay1 = strategy.calculate_delay(1);
        assert_eq!(delay1.as_millis(), 1000);
        
        let delay2 = strategy.calculate_delay(2);
        assert_eq!(delay2.as_millis(), 2000);
        
        let delay3 = strategy.calculate_delay(3);
        assert_eq!(delay3.as_millis(), 4000);
    }
    
    #[test]
    fn test_calculate_delay_respects_max() {
        let config = RetryConfig {
            max_attempts: 5,
            initial_delay_ms: 1000,
            max_delay_ms: 3000,
            exponential_base: 2.0,
        };
        let strategy = RetryStrategy::new(config);
        
        let delay4 = strategy.calculate_delay(4);
        assert_eq!(delay4.as_millis(), 3000);
        
        let delay5 = strategy.calculate_delay(5);
        assert_eq!(delay5.as_millis(), 3000);
    }
    
    #[test]
    fn test_is_retryable_error() {
        assert!(RetryStrategy::is_retryable_error("Connection timeout"));
        assert!(RetryStrategy::is_retryable_error("Network error"));
        assert!(RetryStrategy::is_retryable_error("Rate limit exceeded (429)"));
        assert!(RetryStrategy::is_retryable_error("500 Internal Server Error"));
        assert!(RetryStrategy::is_retryable_error("503 Service Unavailable"));
        
        assert!(!RetryStrategy::is_retryable_error("401 Unauthorized"));
        assert!(!RetryStrategy::is_retryable_error("403 Forbidden"));
        assert!(!RetryStrategy::is_retryable_error("404 Not Found"));
        assert!(!RetryStrategy::is_retryable_error("Invalid API key"));
    }
}
