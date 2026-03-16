use super::provider::{LlmPrompt, LlmProvider, LlmResponse};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

const OPENROUTER_API_BASE: &str = "https://openrouter.ai/api/v1";

#[derive(Debug, Clone)]
pub struct OpenRouterProvider {
    api_key: String,
    model: String,
    client: Client,
    site_url: Option<String>,
    app_name: Option<String>,
}

#[derive(Debug, Serialize)]
struct OpenRouterRequest {
    model: String,
    messages: Vec<OpenRouterMessage>,
    temperature: f32,
    max_tokens: usize,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    stop: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormat>,
}

#[derive(Debug, Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    format_type: String,
}

#[derive(Debug, Serialize)]
struct OpenRouterMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OpenRouterResponse {
    id: String,
    model: String,
    choices: Vec<OpenRouterChoice>,
    usage: OpenRouterUsage,
}

#[derive(Debug, Deserialize)]
struct OpenRouterChoice {
    message: OpenRouterResponseMessage,
    finish_reason: String,
}

#[derive(Debug, Deserialize)]
struct OpenRouterResponseMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OpenRouterUsage {
    prompt_tokens: usize,
    completion_tokens: usize,
    total_tokens: usize,
}

impl OpenRouterProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            api_key,
            model,
            client: Client::new(),
            site_url: None,
            app_name: Some("Strike Security Scanner".to_string()),
        }
    }

    pub fn from_env() -> Result<Self> {
        let api_key = std::env::var("OPENROUTER_API_KEY")
            .context("OPENROUTER_API_KEY environment variable not set")?;
        let model = std::env::var("OPENROUTER_MODEL")
            .unwrap_or_else(|_| "anthropic/claude-3-opus".to_string());
        Ok(Self::new(api_key, model))
    }

    pub fn with_site_url(mut self, site_url: String) -> Self {
        self.site_url = Some(site_url);
        self
    }

    pub fn with_app_name(mut self, app_name: String) -> Self {
        self.app_name = Some(app_name);
        self
    }

    // Popular model shortcuts
    pub fn gpt4_turbo(api_key: String) -> Self {
        Self::new(api_key, "openai/gpt-4-turbo".to_string())
    }

    pub fn claude_opus(api_key: String) -> Self {
        Self::new(api_key, "anthropic/claude-3-opus".to_string())
    }

    pub fn claude_sonnet(api_key: String) -> Self {
        Self::new(api_key, "anthropic/claude-3-sonnet".to_string())
    }

    pub fn gemini_pro(api_key: String) -> Self {
        Self::new(api_key, "google/gemini-pro-1.5".to_string())
    }

    pub fn llama_70b(api_key: String) -> Self {
        Self::new(api_key, "meta-llama/llama-3-70b-instruct".to_string())
    }

    pub fn mixtral_8x7b(api_key: String) -> Self {
        Self::new(api_key, "mistralai/mixtral-8x7b-instruct".to_string())
    }

    fn build_request(&self, prompt: LlmPrompt) -> OpenRouterRequest {
        let mut messages = Vec::new();

        if let Some(system) = prompt.system {
            messages.push(OpenRouterMessage {
                role: "system".to_string(),
                content: system,
            });
        }

        messages.push(OpenRouterMessage {
            role: "user".to_string(),
            content: prompt.user,
        });

        let response_format = if prompt.json_mode {
            Some(ResponseFormat {
                format_type: "json_object".to_string(),
            })
        } else {
            None
        };

        OpenRouterRequest {
            model: self.model.clone(),
            messages,
            temperature: prompt.temperature,
            max_tokens: prompt.max_tokens,
            stop: prompt.stop_sequences,
            response_format,
        }
    }

    fn calculate_cost(&self, prompt_tokens: usize, completion_tokens: usize) -> f64 {
        // OpenRouter pricing varies by model
        // These are approximate costs (as of 2024)
        let (prompt_cost_per_1m, completion_cost_per_1m) = match self.model.as_str() {
            // OpenAI models
            "openai/gpt-4-turbo" => (10.0, 30.0),
            "openai/gpt-4" => (30.0, 60.0),
            "openai/gpt-3.5-turbo" => (0.5, 1.5),
            
            // Anthropic models
            "anthropic/claude-3-opus" => (15.0, 75.0),
            "anthropic/claude-3-sonnet" => (3.0, 15.0),
            "anthropic/claude-3-haiku" => (0.25, 1.25),
            
            // Google models
            "google/gemini-pro-1.5" => (1.25, 5.0),
            "google/gemini-flash-1.5" => (0.075, 0.3),
            
            // Meta models
            "meta-llama/llama-3-70b-instruct" => (0.9, 0.9),
            "meta-llama/llama-3-8b-instruct" => (0.2, 0.2),
            
            // Mistral models
            "mistralai/mixtral-8x7b-instruct" => (0.7, 0.7),
            "mistralai/mistral-7b-instruct" => (0.2, 0.2),
            
            // Default fallback
            _ => (1.0, 1.0),
        };

        let prompt_cost = (prompt_tokens as f64 / 1_000_000.0) * prompt_cost_per_1m;
        let completion_cost = (completion_tokens as f64 / 1_000_000.0) * completion_cost_per_1m;
        
        prompt_cost + completion_cost
    }

    fn get_token_limit(&self) -> usize {
        match self.model.as_str() {
            "openai/gpt-4-turbo" => 128_000,
            "openai/gpt-4" => 8_192,
            "openai/gpt-3.5-turbo" => 16_385,
            "anthropic/claude-3-opus" => 200_000,
            "anthropic/claude-3-sonnet" => 200_000,
            "anthropic/claude-3-haiku" => 200_000,
            "google/gemini-pro-1.5" => 1_000_000,
            "google/gemini-flash-1.5" => 1_000_000,
            "meta-llama/llama-3-70b-instruct" => 8_192,
            "mistralai/mixtral-8x7b-instruct" => 32_768,
            _ => 8_192,
        }
    }
}

#[async_trait]
impl LlmProvider for OpenRouterProvider {
    async fn complete(&self, prompt: LlmPrompt) -> Result<LlmResponse> {
        debug!("Sending request to OpenRouter API (model: {})", self.model);

        let request = self.build_request(prompt);
        let url = format!("{}/chat/completions", OPENROUTER_API_BASE);

        let mut req_builder = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json");

        // Add optional headers
        if let Some(site_url) = &self.site_url {
            req_builder = req_builder.header("HTTP-Referer", site_url);
        }
        if let Some(app_name) = &self.app_name {
            req_builder = req_builder.header("X-Title", app_name);
        }

        let response = req_builder
            .json(&request)
            .send()
            .await
            .context("Failed to send request to OpenRouter API")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            warn!("OpenRouter API error: {} - {}", status, error_text);
            return Err(anyhow!("OpenRouter API error: {} - {}", status, error_text));
        }

        let openrouter_response: OpenRouterResponse = response
            .json()
            .await
            .context("Failed to parse OpenRouter API response")?;

        if openrouter_response.choices.is_empty() {
            return Err(anyhow!("No choices in OpenRouter response"));
        }

        let choice = &openrouter_response.choices[0];
        let content = choice.message.content.clone();
        let tokens_used = openrouter_response.usage.total_tokens;
        let cost_usd = self.calculate_cost(
            openrouter_response.usage.prompt_tokens,
            openrouter_response.usage.completion_tokens,
        );

        debug!(
            "OpenRouter response: {} tokens (prompt: {}, completion: {}), ${:.6}",
            tokens_used,
            openrouter_response.usage.prompt_tokens,
            openrouter_response.usage.completion_tokens,
            cost_usd
        );

        Ok(LlmResponse {
            content,
            model: openrouter_response.model,
            tokens_used,
            finish_reason: choice.finish_reason.clone(),
            cost_usd,
        })
    }

    fn model_id(&self) -> &str {
        &self.model
    }

    fn token_limit(&self) -> usize {
        self.get_token_limit()
    }

    fn estimated_cost_per_1k_tokens(&self) -> f64 {
        // Return average cost per 1k tokens
        let (prompt_cost, completion_cost) = match self.model.as_str() {
            "openai/gpt-4-turbo" => (0.01, 0.03),
            "openai/gpt-4" => (0.03, 0.06),
            "openai/gpt-3.5-turbo" => (0.0005, 0.0015),
            "anthropic/claude-3-opus" => (0.015, 0.075),
            "anthropic/claude-3-sonnet" => (0.003, 0.015),
            "google/gemini-pro-1.5" => (0.00125, 0.005),
            _ => (0.001, 0.001),
        };
        (prompt_cost + completion_cost) / 2.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires API key
    async fn test_openrouter_completion() {
        let provider = OpenRouterProvider::from_env().unwrap();
        let prompt = LlmPrompt::new("Say 'Hello, World!' and nothing else.".to_string());

        let response = provider.complete(prompt).await.unwrap();
        assert!(response.content.contains("Hello"));
        assert!(response.tokens_used > 0);
    }

    #[tokio::test]
    #[ignore]
    async fn test_openrouter_json_mode() {
        let api_key = std::env::var("OPENROUTER_API_KEY").unwrap();
        let provider = OpenRouterProvider::gpt4_turbo(api_key);
        
        let prompt = LlmPrompt::new(
            "Return a JSON object with a single field 'message' containing 'test'".to_string(),
        )
        .with_json_mode(true);

        let response = provider.complete(prompt).await.unwrap();
        let json: serde_json::Value = serde_json::from_str(&response.content).unwrap();
        assert_eq!(json["message"], "test");
    }

    #[tokio::test]
    #[ignore]
    async fn test_openrouter_multiple_models() {
        let api_key = std::env::var("OPENROUTER_API_KEY").unwrap();
        
        let models = vec![
            OpenRouterProvider::claude_opus(api_key.clone()),
            OpenRouterProvider::gemini_pro(api_key.clone()),
            OpenRouterProvider::llama_70b(api_key.clone()),
        ];

        for provider in models {
            let prompt = LlmPrompt::new("Say hello".to_string());
            let response = provider.complete(prompt).await.unwrap();
            assert!(!response.content.is_empty());
            println!("Model: {} - Response: {}", provider.model_id(), response.content);
        }
    }

    #[test]
    fn test_cost_calculation() {
        let api_key = "test-key".to_string();
        let provider = OpenRouterProvider::gpt4_turbo(api_key);
        
        let cost = provider.calculate_cost(1000, 1000);
        assert!(cost > 0.0);
        println!("Cost for 1k prompt + 1k completion tokens: ${:.6}", cost);
    }

    #[test]
    fn test_token_limits() {
        let api_key = "test-key".to_string();
        
        let gpt4 = OpenRouterProvider::gpt4_turbo(api_key.clone());
        assert_eq!(gpt4.token_limit(), 128_000);
        
        let claude = OpenRouterProvider::claude_opus(api_key.clone());
        assert_eq!(claude.token_limit(), 200_000);
        
        let gemini = OpenRouterProvider::gemini_pro(api_key);
        assert_eq!(gemini.token_limit(), 1_000_000);
    }
}
