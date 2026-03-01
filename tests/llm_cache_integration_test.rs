use strike_security::llm::cache::{LlmCache, CacheConfig, CacheStats};
use strike_security::llm::provider::{LlmPrompt, LlmResponse};

#[tokio::test]
async fn test_cache_integration_basic() {
    let cache = LlmCache::new(CacheConfig::default());
    
    let prompt = LlmPrompt::new("Test prompt".to_string());
    let response = LlmResponse {
        content: "Test response".to_string(),
        model: "test-model".to_string(),
        tokens_used: 100,
        finish_reason: "stop".to_string(),
        cost_usd: 0.01,
    };
    
    cache.put("anthropic", "claude", &prompt, response.clone()).await;
    
    let cached = cache.get("anthropic", "claude", &prompt).await;
    assert!(cached.is_some());
    assert_eq!(cached.unwrap().content, "Test response");
}

#[tokio::test]
async fn test_cache_miss_then_hit() {
    let cache = LlmCache::new(CacheConfig::default());
    let prompt = LlmPrompt::new("Another test".to_string());
    
    let first_get = cache.get("openai", "gpt-4", &prompt).await;
    assert!(first_get.is_none());
    
    let response = LlmResponse {
        content: "Response".to_string(),
        model: "gpt-4".to_string(),
        tokens_used: 50,
        finish_reason: "stop".to_string(),
        cost_usd: 0.005,
    };
    
    cache.put("openai", "gpt-4", &prompt, response).await;
    
    let second_get = cache.get("openai", "gpt-4", &prompt).await;
    assert!(second_get.is_some());
}

#[tokio::test]
async fn test_cache_stats_tracking() {
    let cache = LlmCache::new(CacheConfig::default());
    let prompt = LlmPrompt::new("Stats test".to_string());
    
    cache.get("anthropic", "claude", &prompt).await;
    
    let stats = cache.stats().await;
    assert_eq!(stats.misses, 1);
    
    let response = LlmResponse {
        content: "Response".to_string(),
        model: "claude".to_string(),
        tokens_used: 75,
        finish_reason: "stop".to_string(),
        cost_usd: 0.008,
    };
    
    cache.put("anthropic", "claude", &prompt, response).await;
    cache.get("anthropic", "claude", &prompt).await;
    
    let stats = cache.stats().await;
    assert_eq!(stats.hits, 1);
    assert_eq!(stats.misses, 1);
}

#[tokio::test]
async fn test_cache_cost_savings() {
    let cache = LlmCache::new(CacheConfig::default());
    let prompt = LlmPrompt::new("Cost test".to_string());
    
    let response = LlmResponse {
        content: "Expensive response".to_string(),
        model: "gpt-4".to_string(),
        tokens_used: 1000,
        finish_reason: "stop".to_string(),
        cost_usd: 0.05,
    };
    
    cache.put("openai", "gpt-4", &prompt, response).await;
    
    for _ in 0..10 {
        cache.get("openai", "gpt-4", &prompt).await;
    }
    
    let stats = cache.stats().await;
    assert_eq!(stats.hits, 10);
    assert!((stats.total_cost_saved_usd - 0.5).abs() < 0.001);
}

#[tokio::test]
async fn test_cache_different_providers() {
    let cache = LlmCache::new(CacheConfig::default());
    let prompt = LlmPrompt::new("Multi-provider test".to_string());
    
    let anthropic_response = LlmResponse {
        content: "Anthropic response".to_string(),
        model: "claude".to_string(),
        tokens_used: 100,
        finish_reason: "stop".to_string(),
        cost_usd: 0.01,
    };
    
    let openai_response = LlmResponse {
        content: "OpenAI response".to_string(),
        model: "gpt-4".to_string(),
        tokens_used: 100,
        finish_reason: "stop".to_string(),
        cost_usd: 0.015,
    };
    
    cache.put("anthropic", "claude", &prompt, anthropic_response).await;
    cache.put("openai", "gpt-4", &prompt, openai_response).await;
    
    let anthropic_cached = cache.get("anthropic", "claude", &prompt).await;
    let openai_cached = cache.get("openai", "gpt-4", &prompt).await;
    
    assert!(anthropic_cached.is_some());
    assert!(openai_cached.is_some());
    assert_eq!(anthropic_cached.unwrap().content, "Anthropic response");
    assert_eq!(openai_cached.unwrap().content, "OpenAI response");
}

#[tokio::test]
async fn test_cache_prompt_variations() {
    let cache = LlmCache::new(CacheConfig::default());
    
    let prompt1 = LlmPrompt::new("Prompt 1".to_string());
    let prompt2 = LlmPrompt::new("Prompt 2".to_string());
    
    let response1 = LlmResponse {
        content: "Response 1".to_string(),
        model: "claude".to_string(),
        tokens_used: 50,
        finish_reason: "stop".to_string(),
        cost_usd: 0.005,
    };
    
    let response2 = LlmResponse {
        content: "Response 2".to_string(),
        model: "claude".to_string(),
        tokens_used: 60,
        finish_reason: "stop".to_string(),
        cost_usd: 0.006,
    };
    
    cache.put("anthropic", "claude", &prompt1, response1).await;
    cache.put("anthropic", "claude", &prompt2, response2).await;
    
    let cached1 = cache.get("anthropic", "claude", &prompt1).await;
    let cached2 = cache.get("anthropic", "claude", &prompt2).await;
    
    assert_eq!(cached1.unwrap().content, "Response 1");
    assert_eq!(cached2.unwrap().content, "Response 2");
}

#[tokio::test]
async fn test_cache_size_limit() {
    let config = CacheConfig {
        enabled: true,
        ttl_seconds: 3600,
        max_entries: 5,
    };
    let cache = LlmCache::new(config);
    
    for i in 0..10 {
        let prompt = LlmPrompt::new(format!("Prompt {}", i));
        let response = LlmResponse {
            content: format!("Response {}", i),
            model: "claude".to_string(),
            tokens_used: 50,
            finish_reason: "stop".to_string(),
            cost_usd: 0.005,
        };
        cache.put("anthropic", "claude", &prompt, response).await;
    }
    
    let size = cache.size().await;
    assert_eq!(size, 5);
}

#[tokio::test]
async fn test_cache_invalidation() {
    let cache = LlmCache::new(CacheConfig::default());
    let prompt = LlmPrompt::new("Invalidation test".to_string());
    
    let response = LlmResponse {
        content: "Response".to_string(),
        model: "claude".to_string(),
        tokens_used: 50,
        finish_reason: "stop".to_string(),
        cost_usd: 0.005,
    };
    
    cache.put("anthropic", "claude", &prompt, response).await;
    assert!(cache.get("anthropic", "claude", &prompt).await.is_some());
    
    cache.invalidate("anthropic", "claude", &prompt).await;
    assert!(cache.get("anthropic", "claude", &prompt).await.is_none());
}

#[tokio::test]
async fn test_cache_clear_all() {
    let cache = LlmCache::new(CacheConfig::default());
    
    for i in 0..5 {
        let prompt = LlmPrompt::new(format!("Prompt {}", i));
        let response = LlmResponse {
            content: format!("Response {}", i),
            model: "claude".to_string(),
            tokens_used: 50,
            finish_reason: "stop".to_string(),
            cost_usd: 0.005,
        };
        cache.put("anthropic", "claude", &prompt, response).await;
    }
    
    assert_eq!(cache.size().await, 5);
    
    cache.clear().await;
    assert_eq!(cache.size().await, 0);
}

#[tokio::test]
async fn test_cache_disabled() {
    let config = CacheConfig {
        enabled: false,
        ttl_seconds: 3600,
        max_entries: 100,
    };
    let cache = LlmCache::new(config);
    
    let prompt = LlmPrompt::new("Disabled test".to_string());
    let response = LlmResponse {
        content: "Response".to_string(),
        model: "claude".to_string(),
        tokens_used: 50,
        finish_reason: "stop".to_string(),
        cost_usd: 0.005,
    };
    
    cache.put("anthropic", "claude", &prompt, response).await;
    
    let cached = cache.get("anthropic", "claude", &prompt).await;
    assert!(cached.is_none());
}

#[tokio::test]
async fn test_cache_hit_rate_calculation() {
    let cache = LlmCache::new(CacheConfig::default());
    let prompt = LlmPrompt::new("Hit rate test".to_string());
    
    cache.get("anthropic", "claude", &prompt).await;
    cache.get("anthropic", "claude", &prompt).await;
    
    let response = LlmResponse {
        content: "Response".to_string(),
        model: "claude".to_string(),
        tokens_used: 50,
        finish_reason: "stop".to_string(),
        cost_usd: 0.005,
    };
    
    cache.put("anthropic", "claude", &prompt, response).await;
    
    cache.get("anthropic", "claude", &prompt).await;
    cache.get("anthropic", "claude", &prompt).await;
    cache.get("anthropic", "claude", &prompt).await;
    
    let stats = cache.stats().await;
    assert_eq!(stats.hits, 3);
    assert_eq!(stats.misses, 2);
    assert!((stats.hit_rate() - 0.6).abs() < 0.01);
}
