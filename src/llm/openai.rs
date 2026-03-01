use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::env;

use super::provider::{LlmPrompt, LlmProvider, LlmResponse};
use super::retry::{RetryConfig, RetryStrategy};
use super::cache::{CacheConfig, LlmCache};

#[derive(Clone)]
pub struct OpenAiProvider {
    client: async_openai::Client<async_openai::config::OpenAIConfig>,
    model: String,
    retry_strategy: RetryStrategy,
    cache: LlmCache,
}

impl std::fmt::Debug for OpenAiProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenAiProvider")
            .field("model", &self.model)
            .finish()
    }
}

impl OpenAiProvider {
    pub fn new(model: Option<String>) -> Result<Self> {
        let api_key = env::var("OPENAI_API_KEY")
            .context("OPENAI_API_KEY environment variable not set")?;
        
        let config = async_openai::config::OpenAIConfig::new()
            .with_api_key(api_key);
        
        let client = async_openai::Client::with_config(config);
        let model = model.unwrap_or_else(|| "gpt-4o".to_string());
        
        Ok(Self { 
            client, 
            model,
            retry_strategy: RetryStrategy::default(),
            cache: LlmCache::new(CacheConfig::default()),
        })
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    async fn complete(&self, prompt: LlmPrompt) -> Result<LlmResponse> {
        if let Some(cached) = self.cache.get("openai", &self.model, &prompt).await {
            return Ok(cached);
        }
        
        let client = self.client.clone();
        let model = self.model.clone();
        let cost_per_1k: f64 = self.estimated_cost_per_1k_tokens();
        let cache = self.cache.clone();
        let prompt_for_cache = prompt.clone();
        
        let result = self.retry_strategy.execute_with_retry(|| {
            let client = client.clone();
            let model = model.clone();
            let prompt_clone = prompt.clone();
            
            async move {
                use async_openai::types::{
                    ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
                    ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs,
                };
                
                let mut messages: Vec<ChatCompletionRequestMessage> = vec![];
                
                if let Some(system) = &prompt_clone.system {
                    messages.push(
                        ChatCompletionRequestSystemMessageArgs::default()
                            .content(system.clone())
                            .build()?
                            .into()
                    );
                }
                
                messages.push(
                    ChatCompletionRequestUserMessageArgs::default()
                        .content(prompt_clone.user.clone())
                        .build()?
                        .into()
                );
                
                let mut request_builder = CreateChatCompletionRequestArgs::default();
                request_builder
                    .model(&model)
                    .messages(messages)
                    .temperature(prompt_clone.temperature)
                    .max_tokens(prompt_clone.max_tokens as u32);
                
                if !prompt_clone.stop_sequences.is_empty() {
                    request_builder.stop(prompt_clone.stop_sequences.clone());
                }
                
                let request = request_builder.build()?;
                
                let response: async_openai::types::CreateChatCompletionResponse = client
                    .chat()
                    .create(request)
                    .await
                    .context("Failed to get response from OpenAI API")?;
                
                let choice = response.choices.first()
                    .context("No choices in OpenAI response")?;
                
                let content = choice.message.content.clone()
                    .unwrap_or_default();
                
                let tokens_used = response.usage
                    .map(|u| (u.prompt_tokens + u.completion_tokens) as usize)
                    .unwrap_or(0);
                
                let cost_usd = (tokens_used as f64 / 1000.0) * cost_per_1k;
                
                Ok::<LlmResponse, anyhow::Error>(LlmResponse {
                    content,
                    model: response.model.clone(),
                    tokens_used,
                    finish_reason: choice.finish_reason.as_ref()
                        .map(|r| format!("{:?}", r))
                        .unwrap_or_else(|| "unknown".to_string()),
                    cost_usd,
                })
            }
        }).await;
        
        if let Ok(ref response) = result {
            cache.put("openai", &model, &prompt_for_cache, response.clone()).await;
        }
        
        result
    }
    
    fn model_id(&self) -> &str {
        &self.model
    }
    
    fn token_limit(&self) -> usize {
        if self.model.contains("gpt-4o") {
            128_000
        } else if self.model.contains("gpt-4") {
            8_192
        } else {
            4_096
        }
    }
    
    fn estimated_cost_per_1k_tokens(&self) -> f64 {
        if self.model.contains("gpt-4o") {
            0.0025
        } else if self.model.contains("gpt-4-turbo") {
            0.01
        } else if self.model.contains("gpt-4") {
            0.03
        } else {
            0.0005
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cost_estimation() {
        let provider = OpenAiProvider {
            client: async_openai::Client::new(),
            model: "gpt-4o".to_string(),
            retry_strategy: RetryStrategy::default(),
            cache: LlmCache::disabled(),
        };
        
        let cost = provider.estimate_cost(10_000);
        assert!((cost - 0.025).abs() < 0.001);
    }
    
    #[test]
    fn test_token_limits() {
        let provider = OpenAiProvider {
            client: async_openai::Client::new(),
            model: "gpt-4o".to_string(),
            retry_strategy: RetryStrategy::default(),
            cache: LlmCache::disabled(),
        };
        
        assert_eq!(provider.token_limit(), 128_000);
    }
}
