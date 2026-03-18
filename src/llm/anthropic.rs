use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;

use super::provider::{LlmPrompt, LlmProvider, LlmResponse};
use super::retry::RetryStrategy;
use super::cache::{CacheConfig, LlmCache};

#[derive(Clone)]
pub struct AnthropicProvider {
    api_key: String,
    model: String,
    client: Client,
    retry_strategy: RetryStrategy,
    cache: LlmCache,
}

impl std::fmt::Debug for AnthropicProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnthropicProvider")
            .field("model", &self.model)
            .finish()
    }
}

#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: usize,
    temperature: f32,
    messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    stop_sequences: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<ContentBlock>,
    model: String,
    usage: Usage,
    stop_reason: String,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
    text: String,
}

#[derive(Debug, Deserialize)]
struct Usage {
    input_tokens: usize,
    output_tokens: usize,
}

impl AnthropicProvider {
    pub fn new(model: Option<String>) -> Result<Self> {
        let api_key = env::var("ANTHROPIC_API_KEY")
            .context("ANTHROPIC_API_KEY environment variable not set")?;
        
        let model = model.unwrap_or_else(|| "claude-sonnet-4-20250514".to_string());
        
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()?;
        
        Ok(Self {
            api_key,
            model,
            client,
            retry_strategy: RetryStrategy::default(),
            cache: LlmCache::new(CacheConfig::default()),
        })
    }
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    async fn complete(&self, prompt: LlmPrompt) -> Result<LlmResponse> {
        if let Some(cached) = self.cache.get("anthropic", &self.model, &prompt).await {
            return Ok(cached);
        }
        
        let api_key = self.api_key.clone();
        let model = self.model.clone();
        let client = self.client.clone();
        let cost_per_1k = self.estimated_cost_per_1k_tokens();
        let cache = self.cache.clone();
        let prompt_for_cache = prompt.clone();
        
        let result = self.retry_strategy.execute_with_retry(|| {
            let api_key = api_key.clone();
            let model = model.clone();
            let client = client.clone();
            let prompt_clone = prompt.clone();
            
            async move {
                let request: AnthropicRequest = AnthropicRequest {
                    model: model.clone(),
                    max_tokens: prompt_clone.max_tokens,
                    temperature: prompt_clone.temperature,
                    messages: vec![Message {
                        role: "user".to_string(),
                        content: prompt_clone.user.clone(),
                    }],
                    system: prompt_clone.system.clone(),
                    stop_sequences: prompt_clone.stop_sequences.clone(),
                };
                
                let response: reqwest::Response = client
                    .post("https://api.anthropic.com/v1/messages")
                    .header("x-api-key", &api_key)
                    .header("anthropic-version", "2023-06-01")
                    .header("content-type", "application/json")
                    .json(&request)
                    .send()
                    .await
                    .context("Failed to send request to Anthropic API")?;
                
                if !response.status().is_success() {
                    let status = response.status();
                    let error_text = response.text().await.unwrap_or_default();
                    anyhow::bail!("Anthropic API error {}: {}", status, error_text);
                }
                
                let anthropic_response: AnthropicResponse = response.json::<AnthropicResponse>()
                    .await
                    .context("Failed to parse Anthropic response")?;
                
                let content = anthropic_response.content
                    .into_iter()
                    .map(|block| block.text)
                    .collect::<Vec<_>>()
                    .join("\n");
                
                let total_tokens = anthropic_response.usage.input_tokens + anthropic_response.usage.output_tokens;
                
                let cost_usd = (total_tokens as f64 / 1000.0) * cost_per_1k;
                
                Ok::<LlmResponse, anyhow::Error>(LlmResponse {
                    content,
                    model: anthropic_response.model,
                    tokens_used: total_tokens,
                    finish_reason: anthropic_response.stop_reason,
                    cost_usd,
                })
            }
        }).await;
        
        if let Ok(ref response) = result {
            cache.put("anthropic", &model, &prompt_for_cache, response.clone()).await;
        }
        
        result
    }
    
    fn model_id(&self) -> &str {
        &self.model
    }
    
    fn token_limit(&self) -> usize {
        200_000
    }
    
    fn estimated_cost_per_1k_tokens(&self) -> f64 {
        if self.model.contains("opus") {
            0.015
        } else if self.model.contains("sonnet") {
            0.003
        } else {
            0.00025
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cost_estimation() {
        let provider = AnthropicProvider {
            api_key: "test".to_string(),
            model: "claude-sonnet-4-20250514".to_string(),
            client: Client::new(),
            retry_strategy: RetryStrategy::default(),
            cache: LlmCache::disabled(),
        };
        
        let cost = provider.estimate_cost(10_000);
        assert!((cost - 0.03).abs() < 0.001);
    }
}
