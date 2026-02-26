use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::provider::{LlmPrompt, LlmProvider, LlmResponse};

#[derive(Debug, Clone)]
pub struct OllamaProvider {
    base_url: String,
    model: String,
    client: Client,
}

#[derive(Debug, Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    stream: bool,
    options: OllamaOptions,
}

#[derive(Debug, Serialize)]
struct OllamaOptions {
    temperature: f32,
    num_predict: i32,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    stop: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct OllamaResponse {
    model: String,
    response: String,
    done: bool,
    #[serde(default)]
    total_duration: u64,
    #[serde(default)]
    prompt_eval_count: usize,
    #[serde(default)]
    eval_count: usize,
}

impl OllamaProvider {
    pub fn new(base_url: Option<String>, model: Option<String>) -> Result<Self> {
        let base_url = base_url.unwrap_or_else(|| "http://localhost:11434".to_string());
        let model = model.unwrap_or_else(|| "llama3.1".to_string());
        
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()?;
        
        Ok(Self {
            base_url,
            model,
            client,
        })
    }
    
    pub async fn is_available(&self) -> bool {
        self.client
            .get(format!("{}/api/tags", self.base_url))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    async fn complete(&self, prompt: LlmPrompt) -> Result<LlmResponse> {
        let request = OllamaRequest {
            model: self.model.clone(),
            prompt: prompt.user,
            system: prompt.system,
            stream: false,
            options: OllamaOptions {
                temperature: prompt.temperature,
                num_predict: prompt.max_tokens as i32,
                stop: prompt.stop_sequences,
            },
        };
        
        let response = self.client
            .post(format!("{}/api/generate", self.base_url))
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Ollama API")?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Ollama API error {}: {}", status, error_text);
        }
        
        let ollama_response: OllamaResponse = response
            .json()
            .await
            .context("Failed to parse Ollama API response")?;
        
        let tokens_used = ollama_response.prompt_eval_count + ollama_response.eval_count;
        
        Ok(LlmResponse {
            content: ollama_response.response,
            model: ollama_response.model,
            tokens_used,
            finish_reason: if ollama_response.done { "stop" } else { "length" }.to_string(),
            cost_usd: 0.0,
        })
    }
    
    fn model_id(&self) -> &str {
        &self.model
    }
    
    fn token_limit(&self) -> usize {
        if self.model.contains("llama3.1") {
            128_000
        } else if self.model.contains("llama3") {
            8_192
        } else {
            4_096
        }
    }
    
    fn estimated_cost_per_1k_tokens(&self) -> f64 {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ollama_provider_creation() {
        let provider = OllamaProvider::new(None, None).unwrap();
        assert_eq!(provider.base_url, "http://localhost:11434");
        assert_eq!(provider.model, "llama3.1");
    }
    
    #[test]
    fn test_cost_is_zero() {
        let provider = OllamaProvider::new(None, None).unwrap();
        assert_eq!(provider.estimate_cost(10_000), 0.0);
    }
}
