use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;

use super::provider::{LlmPrompt, LlmResponse};

#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub enabled: bool,
    pub ttl_seconds: u64,
    pub max_entries: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            ttl_seconds: 3600,
            max_entries: 1000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry {
    response: LlmResponse,
    created_at: SystemTime,
    access_count: u64,
}

impl CacheEntry {
    fn new(response: LlmResponse) -> Self {
        Self {
            response,
            created_at: SystemTime::now(),
            access_count: 0,
        }
    }
    
    fn is_expired(&self, ttl: Duration) -> bool {
        match self.created_at.elapsed() {
            Ok(elapsed) => elapsed > ttl,
            Err(_) => true,
        }
    }
}

#[derive(Clone)]
pub struct LlmCache {
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
    stats: Arc<RwLock<CacheStats>>,
    config: CacheConfig,
}

impl std::fmt::Debug for LlmCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LlmCache")
            .field("config", &self.config)
            .finish()
    }
}

#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub total_cost_saved_usd: f64,
    pub evictions: u64,
}

impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

impl LlmCache {
    pub fn new(config: CacheConfig) -> Self {
        Self {
            config,
            cache: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(CacheStats::default())),
        }
    }
    
    pub fn disabled() -> Self {
        Self::new(CacheConfig {
            enabled: false,
            ..Default::default()
        })
    }
    
    fn generate_cache_key(
        provider: &str,
        model: &str,
        prompt: &LlmPrompt,
    ) -> String {
        let mut hasher = Sha256::new();
        
        hasher.update(provider.as_bytes());
        hasher.update(model.as_bytes());
        hasher.update(prompt.user.as_bytes());
        
        if let Some(system) = &prompt.system {
            hasher.update(system.as_bytes());
        }
        
        hasher.update(prompt.temperature.to_string().as_bytes());
        hasher.update(prompt.max_tokens.to_string().as_bytes());
        hasher.update(prompt.json_mode.to_string().as_bytes());
        
        for seq in &prompt.stop_sequences {
            hasher.update(seq.as_bytes());
        }
        
        format!("{:x}", hasher.finalize())
    }
    
    pub async fn get(
        &self,
        provider: &str,
        model: &str,
        prompt: &LlmPrompt,
    ) -> Option<LlmResponse> {
        if !self.config.enabled {
            return None;
        }
        
        let key = Self::generate_cache_key(provider, model, prompt);
        let ttl = Duration::from_secs(self.config.ttl_seconds);
        
        let mut cache = self.cache.write().await;
        
        if let Some(entry) = cache.get_mut(&key) {
            if entry.is_expired(ttl) {
                cache.remove(&key);
                let mut stats = self.stats.write().await;
                stats.misses += 1;
                stats.evictions += 1;
                return None;
            }
            
            entry.access_count += 1;
            let mut stats = self.stats.write().await;
            stats.hits += 1;
            stats.total_cost_saved_usd += entry.response.cost_usd;
            
            return Some(entry.response.clone());
        }
        
        let mut stats = self.stats.write().await;
        stats.misses += 1;
        None
    }
    
    pub async fn put(
        &self,
        provider: &str,
        model: &str,
        prompt: &LlmPrompt,
        response: LlmResponse,
    ) {
        if !self.config.enabled {
            return;
        }
        
        let key = Self::generate_cache_key(provider, model, prompt);
        let mut cache = self.cache.write().await;
        
        if cache.len() >= self.config.max_entries {
            self.evict_oldest(&mut cache).await;
        }
        
        cache.insert(key, CacheEntry::new(response));
    }
    
    async fn evict_oldest(&self, cache: &mut HashMap<String, CacheEntry>) {
        if cache.is_empty() {
            return;
        }
        
        let oldest_key = cache
            .iter()
            .min_by_key(|(_, entry)| entry.created_at)
            .map(|(key, _)| key.clone());
        
        if let Some(key) = oldest_key {
            cache.remove(&key);
            let mut stats = self.stats.write().await;
            stats.evictions += 1;
        }
    }
    
    pub async fn invalidate(&self, provider: &str, model: &str, prompt: &LlmPrompt) {
        let key = Self::generate_cache_key(provider, model, prompt);
        let mut cache = self.cache.write().await;
        cache.remove(&key);
    }
    
    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }
    
    pub async fn stats(&self) -> CacheStats {
        self.stats.read().await.clone()
    }
    
    pub async fn size(&self) -> usize {
        self.cache.read().await.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_prompt() -> LlmPrompt {
        LlmPrompt::new("Test prompt".to_string())
            .with_system("System prompt".to_string())
            .with_temperature(0.7)
            .with_max_tokens(1000)
    }
    
    fn create_test_response() -> LlmResponse {
        LlmResponse {
            content: "Test response".to_string(),
            model: "test-model".to_string(),
            tokens_used: 100,
            finish_reason: "stop".to_string(),
            cost_usd: 0.01,
        }
    }

    #[tokio::test]
    async fn test_cache_creation() {
        let cache = LlmCache::new(CacheConfig::default());
        assert_eq!(cache.size().await, 0);
    }
    
    #[tokio::test]
    async fn test_cache_put_and_get() {
        let cache = LlmCache::new(CacheConfig::default());
        let prompt = create_test_prompt();
        let response = create_test_response();
        
        cache.put("anthropic", "claude", &prompt, response.clone()).await;
        
        let cached = cache.get("anthropic", "claude", &prompt).await;
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().content, "Test response");
    }
    
    #[tokio::test]
    async fn test_cache_miss() {
        let cache = LlmCache::new(CacheConfig::default());
        let prompt = create_test_prompt();
        
        let cached = cache.get("anthropic", "claude", &prompt).await;
        assert!(cached.is_none());
    }
    
    #[tokio::test]
    async fn test_cache_key_uniqueness() {
        let prompt1 = LlmPrompt::new("Prompt 1".to_string());
        let prompt2 = LlmPrompt::new("Prompt 2".to_string());
        
        let key1 = LlmCache::generate_cache_key("anthropic", "claude", &prompt1);
        let key2 = LlmCache::generate_cache_key("anthropic", "claude", &prompt2);
        
        assert_ne!(key1, key2);
    }
    
    #[tokio::test]
    async fn test_cache_key_includes_provider_and_model() {
        let prompt = create_test_prompt();
        
        let key1 = LlmCache::generate_cache_key("anthropic", "claude", &prompt);
        let key2 = LlmCache::generate_cache_key("openai", "gpt-4", &prompt);
        
        assert_ne!(key1, key2);
    }
    
    #[tokio::test]
    async fn test_cache_stats_hit() {
        let cache = LlmCache::new(CacheConfig::default());
        let prompt = create_test_prompt();
        let response = create_test_response();
        
        cache.put("anthropic", "claude", &prompt, response).await;
        cache.get("anthropic", "claude", &prompt).await;
        
        let stats = cache.stats().await;
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 0);
    }
    
    #[tokio::test]
    async fn test_cache_stats_miss() {
        let cache = LlmCache::new(CacheConfig::default());
        let prompt = create_test_prompt();
        
        cache.get("anthropic", "claude", &prompt).await;
        
        let stats = cache.stats().await;
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 1);
    }
    
    #[tokio::test]
    async fn test_cache_hit_rate() {
        let cache = LlmCache::new(CacheConfig::default());
        let prompt = create_test_prompt();
        let response = create_test_response();
        
        cache.put("anthropic", "claude", &prompt, response).await;
        
        cache.get("anthropic", "claude", &prompt).await;
        cache.get("anthropic", "claude", &prompt).await;
        cache.get("openai", "gpt-4", &prompt).await;
        
        let stats = cache.stats().await;
        assert_eq!(stats.hits, 2);
        assert_eq!(stats.misses, 1);
        assert!((stats.hit_rate() - 0.666).abs() < 0.01);
    }
    
    #[tokio::test]
    async fn test_cache_cost_tracking() {
        let cache = LlmCache::new(CacheConfig::default());
        let prompt = create_test_prompt();
        let response = create_test_response();
        
        cache.put("anthropic", "claude", &prompt, response).await;
        
        cache.get("anthropic", "claude", &prompt).await;
        cache.get("anthropic", "claude", &prompt).await;
        
        let stats = cache.stats().await;
        assert!((stats.total_cost_saved_usd - 0.02).abs() < 0.001);
    }
    
    #[tokio::test]
    async fn test_cache_max_entries_eviction() {
        let config = CacheConfig {
            enabled: true,
            ttl_seconds: 3600,
            max_entries: 2,
        };
        let cache = LlmCache::new(config);
        
        let prompt1 = LlmPrompt::new("Prompt 1".to_string());
        let prompt2 = LlmPrompt::new("Prompt 2".to_string());
        let prompt3 = LlmPrompt::new("Prompt 3".to_string());
        
        cache.put("anthropic", "claude", &prompt1, create_test_response()).await;
        cache.put("anthropic", "claude", &prompt2, create_test_response()).await;
        cache.put("anthropic", "claude", &prompt3, create_test_response()).await;
        
        assert_eq!(cache.size().await, 2);
        
        let stats = cache.stats().await;
        assert_eq!(stats.evictions, 1);
    }
    
    #[tokio::test]
    async fn test_cache_invalidation() {
        let cache = LlmCache::new(CacheConfig::default());
        let prompt = create_test_prompt();
        let response = create_test_response();
        
        cache.put("anthropic", "claude", &prompt, response).await;
        assert_eq!(cache.size().await, 1);
        
        cache.invalidate("anthropic", "claude", &prompt).await;
        assert_eq!(cache.size().await, 0);
    }
    
    #[tokio::test]
    async fn test_cache_clear() {
        let cache = LlmCache::new(CacheConfig::default());
        
        for i in 0..5 {
            let prompt = LlmPrompt::new(format!("Prompt {}", i));
            cache.put("anthropic", "claude", &prompt, create_test_response()).await;
        }
        
        assert_eq!(cache.size().await, 5);
        
        cache.clear().await;
        assert_eq!(cache.size().await, 0);
    }
    
    #[tokio::test]
    async fn test_cache_disabled() {
        let cache = LlmCache::disabled();
        let prompt = create_test_prompt();
        let response = create_test_response();
        
        cache.put("anthropic", "claude", &prompt, response).await;
        
        let cached = cache.get("anthropic", "claude", &prompt).await;
        assert!(cached.is_none());
        assert_eq!(cache.size().await, 0);
    }
    
    #[tokio::test]
    async fn test_cache_ttl_expiration() {
        let config = CacheConfig {
            enabled: true,
            ttl_seconds: 1,
            max_entries: 100,
        };
        let cache = LlmCache::new(config);
        let prompt = create_test_prompt();
        let response = create_test_response();
        
        cache.put("anthropic", "claude", &prompt, response).await;
        
        let cached = cache.get("anthropic", "claude", &prompt).await;
        assert!(cached.is_some());
        
        tokio::time::sleep(Duration::from_secs(2)).await;
        
        let cached = cache.get("anthropic", "claude", &prompt).await;
        assert!(cached.is_none());
    }
}
