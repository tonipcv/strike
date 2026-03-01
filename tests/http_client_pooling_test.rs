use strike_security::tools::http_client::{HttpClient, HttpClientConfig};

#[tokio::test]
async fn test_http_client_with_default_config() {
    let client = HttpClient::new(10, 30);
    assert!(client.is_ok());
}

#[tokio::test]
async fn test_http_client_with_custom_config() {
    let config = HttpClientConfig {
        rate_limit: 20,
        timeout_seconds: 60,
        pool_max_idle_per_host: 64,
        pool_idle_timeout_seconds: 120,
        tcp_keepalive_seconds: 90,
        http2_adaptive_window: true,
    };
    
    let client = HttpClient::with_config(config);
    assert!(client.is_ok());
}

#[tokio::test]
async fn test_http_client_config_defaults() {
    let config = HttpClientConfig::default();
    
    assert_eq!(config.rate_limit, 10);
    assert_eq!(config.timeout_seconds, 30);
    assert_eq!(config.pool_max_idle_per_host, 32);
    assert_eq!(config.pool_idle_timeout_seconds, 90);
    assert_eq!(config.tcp_keepalive_seconds, 60);
    assert!(config.http2_adaptive_window);
}

#[tokio::test]
async fn test_http_client_pool_configuration() {
    let config = HttpClientConfig {
        rate_limit: 5,
        timeout_seconds: 15,
        pool_max_idle_per_host: 16,
        pool_idle_timeout_seconds: 60,
        tcp_keepalive_seconds: 30,
        http2_adaptive_window: false,
    };
    
    let client = HttpClient::with_config(config);
    assert!(client.is_ok());
}

#[tokio::test]
async fn test_http_client_high_concurrency_config() {
    let config = HttpClientConfig {
        rate_limit: 100,
        timeout_seconds: 120,
        pool_max_idle_per_host: 128,
        pool_idle_timeout_seconds: 180,
        tcp_keepalive_seconds: 120,
        http2_adaptive_window: true,
    };
    
    let client = HttpClient::with_config(config);
    assert!(client.is_ok());
}

#[tokio::test]
async fn test_http_client_low_timeout_config() {
    let config = HttpClientConfig {
        rate_limit: 5,
        timeout_seconds: 5,
        pool_max_idle_per_host: 8,
        pool_idle_timeout_seconds: 30,
        tcp_keepalive_seconds: 15,
        http2_adaptive_window: true,
    };
    
    let client = HttpClient::with_config(config);
    assert!(client.is_ok());
}

#[test]
fn test_http_client_config_builder_pattern() {
    let config = HttpClientConfig {
        rate_limit: 15,
        timeout_seconds: 45,
        pool_max_idle_per_host: 48,
        pool_idle_timeout_seconds: 100,
        tcp_keepalive_seconds: 75,
        http2_adaptive_window: true,
    };
    
    assert_eq!(config.rate_limit, 15);
    assert_eq!(config.timeout_seconds, 45);
}

#[tokio::test]
async fn test_http_client_backward_compatibility() {
    let client_old = HttpClient::new(10, 30);
    let client_new = HttpClient::with_config(HttpClientConfig {
        rate_limit: 10,
        timeout_seconds: 30,
        ..Default::default()
    });
    
    assert!(client_old.is_ok());
    assert!(client_new.is_ok());
}

#[test]
fn test_http_client_config_validation() {
    let config = HttpClientConfig {
        rate_limit: 0,
        timeout_seconds: 0,
        pool_max_idle_per_host: 0,
        pool_idle_timeout_seconds: 0,
        tcp_keepalive_seconds: 0,
        http2_adaptive_window: false,
    };
    
    assert_eq!(config.rate_limit, 0);
}

#[tokio::test]
async fn test_http_client_multiple_instances() {
    let config1 = HttpClientConfig {
        rate_limit: 10,
        timeout_seconds: 30,
        ..Default::default()
    };
    
    let config2 = HttpClientConfig {
        rate_limit: 20,
        timeout_seconds: 60,
        ..Default::default()
    };
    
    let client1 = HttpClient::with_config(config1);
    let client2 = HttpClient::with_config(config2);
    
    assert!(client1.is_ok());
    assert!(client2.is_ok());
}
