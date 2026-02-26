use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;

use super::provider::{LlmPrompt, LlmProvider, LlmResponse};

#[derive(Debug, Clone)]
pub struct AnthropicProvider {
    api_key: String,
    model: String,
    client: Client,
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
        })
    }
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    async fn complete(&self, prompt: LlmPrompt) -> Result<LlmResponse> {
        let request = AnthropicRequest {
            model: self.model.clone(),
            max_tokens: prompt.max_tokens,
            temperature: prompt.temperature,
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt.user,
            }],
            system: prompt.system,
            stop_sequences: prompt.stop_sequences,
        };
        
        let response = self.client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
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
        
        let anthropic_response: AnthropicResponse = response
            .json()
            .await
            .context("Failed to parse Anthropic API response")?;
        
        let content = anthropic_response.content
            .into_iter()
            .map(|block| block.text)
            .collect::<Vec<_>>()
            .join("\n");
        
        let total_tokens = anthropic_response.usage.input_tokens + anthropic_response.usage.output_tokens;
        
        let cost_usd = self.estimate_cost(total_tokens);
        
        Ok(LlmResponse {
            content,
            model: anthropic_response.model,
            tokens_used: total_tokens,
            finish_reason: anthropic_response.stop_reason,
            cost_usd,
        })
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
        };
        
        let cost = provider.estimate_cost(10_000);
        assert!((cost - 0.03).abs() < 0.001);
    }
}
