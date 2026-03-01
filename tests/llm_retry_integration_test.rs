use strike_security::llm::retry::{RetryConfig, RetryStrategy};
use anyhow::{anyhow, Result};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

#[tokio::test]
async fn test_retry_strategy_success_first_attempt() {
    let strategy = RetryStrategy::default();
    let counter = Arc::new(AtomicU32::new(0));
    let counter_clone = counter.clone();
    
    let result = strategy.execute_with_retry(|| {
        let counter = counter_clone.clone();
        async move {
            counter.fetch_add(1, Ordering::SeqCst);
            Ok::<_, anyhow::Error>("success")
        }
    }).await;
    
    assert!(result.is_ok());
    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_retry_strategy_success_after_retries() {
    let strategy = RetryStrategy::default();
    let counter = Arc::new(AtomicU32::new(0));
    let counter_clone = counter.clone();
    
    let result = strategy.execute_with_retry(|| {
        let counter = counter_clone.clone();
        async move {
            let count = counter.fetch_add(1, Ordering::SeqCst);
            if count < 2 {
                Err(anyhow!("temporary error"))
            } else {
                Ok("success")
            }
        }
    }).await;
    
    assert!(result.is_ok());
    assert_eq!(counter.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn test_retry_strategy_max_attempts() {
    let strategy = RetryStrategy::default();
    let counter = Arc::new(AtomicU32::new(0));
    let counter_clone = counter.clone();
    
    let result = strategy.execute_with_retry(|| {
        let counter = counter_clone.clone();
        async move {
            counter.fetch_add(1, Ordering::SeqCst);
            Err::<String, _>(anyhow!("persistent error"))
        }
    }).await;
    
    assert!(result.is_err());
    assert_eq!(counter.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn test_retry_config_custom_max_attempts() {
    let config = RetryConfig {
        max_attempts: 5,
        ..Default::default()
    };
    let strategy = RetryStrategy::new(config);
    let counter = Arc::new(AtomicU32::new(0));
    let counter_clone = counter.clone();
    
    let result = strategy.execute_with_retry(|| {
        let counter = counter_clone.clone();
        async move {
            counter.fetch_add(1, Ordering::SeqCst);
            Err::<String, _>(anyhow!("error"))
        }
    }).await;
    
    assert!(result.is_err());
    assert_eq!(counter.load(Ordering::SeqCst), 5);
}

#[tokio::test]
async fn test_retry_config_custom_delays() {
    let config = RetryConfig {
        max_attempts: 3,
        initial_delay_ms: 10,
        max_delay_ms: 100,
        backoff_multiplier: 3.0,
    };
    let strategy = RetryStrategy::new(config);
    let counter = Arc::new(AtomicU32::new(0));
    let counter_clone = counter.clone();
    
    let start = std::time::Instant::now();
    
    let result = strategy.execute_with_retry(|| {
        let counter = counter_clone.clone();
        async move {
            counter.fetch_add(1, Ordering::SeqCst);
            Err::<String, _>(anyhow!("error"))
        }
    }).await;
    
    let elapsed = start.elapsed();
    
    assert!(result.is_err());
    assert!(elapsed.as_millis() >= 10);
}

#[tokio::test]
async fn test_retry_strategy_exponential_backoff() {
    let strategy = RetryStrategy::default();
    let counter = Arc::new(AtomicU32::new(0));
    let counter_clone = counter.clone();
    
    let start = std::time::Instant::now();
    
    strategy.execute_with_retry(|| {
        let counter = counter_clone.clone();
        async move {
            counter.fetch_add(1, Ordering::SeqCst);
            Err::<String, _>(anyhow!("error"))
        }
    }).await.ok();
    
    let elapsed = start.elapsed();
    
    assert!(elapsed.as_millis() >= 1000);
}

#[test]
fn test_retry_config_defaults() {
    let config = RetryConfig::default();
    
    assert_eq!(config.max_attempts, 3);
    assert_eq!(config.initial_delay_ms, 1000);
    assert_eq!(config.max_delay_ms, 8000);
    assert_eq!(config.backoff_multiplier, 2.0);
}

#[test]
fn test_retry_config_custom_values() {
    let config = RetryConfig {
        max_attempts: 10,
        initial_delay_ms: 500,
        max_delay_ms: 16000,
        backoff_multiplier: 1.5,
    };
    
    assert_eq!(config.max_attempts, 10);
    assert_eq!(config.initial_delay_ms, 500);
    assert_eq!(config.max_delay_ms, 16000);
    assert_eq!(config.backoff_multiplier, 1.5);
}

#[tokio::test]
async fn test_retry_with_different_error_types() {
    let strategy = RetryStrategy::default();
    
    let result1 = strategy.execute_with_retry(|| async {
        Err::<String, _>(anyhow!("network error"))
    }).await;
    
    let result2 = strategy.execute_with_retry(|| async {
        Err::<String, _>(anyhow!("timeout"))
    }).await;
    
    assert!(result1.is_err());
    assert!(result2.is_err());
}

#[tokio::test]
async fn test_retry_preserves_success_value() {
    let strategy = RetryStrategy::default();
    
    let result = strategy.execute_with_retry(|| async {
        Ok::<_, anyhow::Error>(42)
    }).await;
    
    assert_eq!(result.unwrap(), 42);
}

#[tokio::test]
async fn test_retry_with_complex_return_type() {
    let strategy = RetryStrategy::default();
    
    #[derive(Debug, PartialEq)]
    struct ComplexData {
        id: u32,
        name: String,
    }
    
    let result = strategy.execute_with_retry(|| async {
        Ok::<_, anyhow::Error>(ComplexData {
            id: 1,
            name: "test".to_string(),
        })
    }).await;
    
    assert!(result.is_ok());
    assert_eq!(result.unwrap().id, 1);
}

#[tokio::test]
async fn test_multiple_retry_strategies() {
    let strategy1 = RetryStrategy::new(RetryConfig {
        max_attempts: 2,
        ..Default::default()
    });
    
    let strategy2 = RetryStrategy::new(RetryConfig {
        max_attempts: 4,
        ..Default::default()
    });
    
    let counter1 = Arc::new(AtomicU32::new(0));
    let counter2 = Arc::new(AtomicU32::new(0));
    
    let c1 = counter1.clone();
    let c2 = counter2.clone();
    
    strategy1.execute_with_retry(|| {
        let c = c1.clone();
        async move {
            c.fetch_add(1, Ordering::SeqCst);
            Err::<String, _>(anyhow!("error"))
        }
    }).await.ok();
    
    strategy2.execute_with_retry(|| {
        let c = c2.clone();
        async move {
            c.fetch_add(1, Ordering::SeqCst);
            Err::<String, _>(anyhow!("error"))
        }
    }).await.ok();
    
    assert_eq!(counter1.load(Ordering::SeqCst), 2);
    assert_eq!(counter2.load(Ordering::SeqCst), 4);
}

#[tokio::test]
async fn test_retry_strategy_clone() {
    let strategy1 = RetryStrategy::default();
    let strategy2 = strategy1.clone();
    
    let result1 = strategy1.execute_with_retry(|| async {
        Ok::<_, anyhow::Error>("test1")
    }).await;
    
    let result2 = strategy2.execute_with_retry(|| async {
        Ok::<_, anyhow::Error>("test2")
    }).await;
    
    assert!(result1.is_ok());
    assert!(result2.is_ok());
}
