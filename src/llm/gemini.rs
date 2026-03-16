use super::provider::{LlmPrompt, LlmProvider, LlmResponse};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

const GEMINI_API_BASE: &str = "https://generativelanguage.googleapis.com/v1beta";

#[derive(Debug, Clone)]
pub struct GeminiProvider {
    api_key: String,
    model: String,
    client: Client,
}

#[derive(Debug, Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<GeminiSystemInstruction>,
    generation_config: GeminiGenerationConfig,
}

#[derive(Debug, Serialize)]
struct GeminiContent {
    role: String,
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize)]
struct GeminiPart {
    text: String,
}

#[derive(Debug, Serialize)]
struct GeminiSystemInstruction {
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize)]
struct GeminiGenerationConfig {
    temperature: f32,
    max_output_tokens: usize,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    stop_sequences: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_mime_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
    #[serde(default)]
    usage_metadata: GeminiUsageMetadata,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidate {
    content: GeminiResponseContent,
    finish_reason: String,
}

#[derive(Debug, Deserialize)]
struct GeminiResponseContent {
    parts: Vec<GeminiResponsePart>,
}

#[derive(Debug, Deserialize)]
struct GeminiResponsePart {
    text: String,
}

#[derive(Debug, Deserialize, Default)]
struct GeminiUsageMetadata {
    #[serde(default)]
    prompt_token_count: usize,
    #[serde(default)]
    candidates_token_count: usize,
    #[serde(default)]
    total_token_count: usize,
}

impl GeminiProvider {
    pub fn new(api_key: String) -> Self {
        Self::with_model(api_key, "gemini-1.5-pro".to_string())
    }

    pub fn with_model(api_key: String, model: String) -> Self {
        Self {
            api_key,
            model,
            client: Client::new(),
        }
    }

    pub fn gemini_pro() -> Result<Self> {
        let api_key = std::env::var("GOOGLE_API_KEY")
            .or_else(|_| std::env::var("GEMINI_API_KEY"))
            .context("GOOGLE_API_KEY or GEMINI_API_KEY environment variable not set")?;
        Ok(Self::new(api_key))
    }

    pub fn gemini_flash() -> Result<Self> {
        let api_key = std::env::var("GOOGLE_API_KEY")
            .or_else(|_| std::env::var("GEMINI_API_KEY"))
            .context("GOOGLE_API_KEY or GEMINI_API_KEY environment variable not set")?;
        Ok(Self::with_model(api_key, "gemini-1.5-flash".to_string()))
    }

    fn build_request(&self, prompt: LlmPrompt) -> GeminiRequest {
        let contents = vec![GeminiContent {
            role: "user".to_string(),
            parts: vec![GeminiPart {
                text: prompt.user,
            }],
        }];

        let system_instruction = prompt.system.map(|system| GeminiSystemInstruction {
            parts: vec![GeminiPart { text: system }],
        });

        let response_mime_type = if prompt.json_mode {
            Some("application/json".to_string())
        } else {
            None
        };

        GeminiRequest {
            contents,
            system_instruction,
            generation_config: GeminiGenerationConfig {
                temperature: prompt.temperature,
                max_output_tokens: prompt.max_tokens,
                stop_sequences: prompt.stop_sequences,
                response_mime_type,
            },
        }
    }

    fn calculate_cost(&self, tokens: usize) -> f64 {
        // Gemini 1.5 Pro pricing (as of 2024)
        // Input: $0.00125 per 1K tokens
        // Output: $0.005 per 1K tokens
        // Using average for simplicity
        let cost_per_1k = match self.model.as_str() {
            "gemini-1.5-pro" => 0.003125, // Average of input/output
            "gemini-1.5-flash" => 0.0001875, // Flash is cheaper
            _ => 0.003125,
        };
        (tokens as f64 / 1000.0) * cost_per_1k
    }
}

#[async_trait]
impl LlmProvider for GeminiProvider {
    async fn complete(&self, prompt: LlmPrompt) -> Result<LlmResponse> {
        debug!("Sending request to Gemini API (model: {})", self.model);

        let request = self.build_request(prompt);
        let url = format!(
            "{}/models/{}:generateContent?key={}",
            GEMINI_API_BASE, self.model, self.api_key
        );

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Gemini API")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            warn!("Gemini API error: {} - {}", status, error_text);
            return Err(anyhow!("Gemini API error: {} - {}", status, error_text));
        }

        let gemini_response: GeminiResponse = response
            .json()
            .await
            .context("Failed to parse Gemini API response")?;

        if gemini_response.candidates.is_empty() {
            return Err(anyhow!("No candidates in Gemini response"));
        }

        let candidate = &gemini_response.candidates[0];
        
        if candidate.content.parts.is_empty() {
            return Err(anyhow!("No parts in Gemini response"));
        }

        let content = candidate.content.parts[0].text.clone();
        let tokens_used = gemini_response.usage_metadata.total_token_count;
        let cost_usd = self.calculate_cost(tokens_used);

        debug!(
            "Gemini response: {} tokens, ${:.6}",
            tokens_used, cost_usd
        );

        Ok(LlmResponse {
            content,
            model: self.model.clone(),
            tokens_used,
            finish_reason: candidate.finish_reason.clone(),
            cost_usd,
        })
    }

    fn model_id(&self) -> &str {
        &self.model
    }

    fn token_limit(&self) -> usize {
        match self.model.as_str() {
            "gemini-1.5-pro" => 1_000_000, // 1M context window
            "gemini-1.5-flash" => 1_000_000,
            _ => 32_000,
        }
    }

    fn estimated_cost_per_1k_tokens(&self) -> f64 {
        match self.model.as_str() {
            "gemini-1.5-pro" => 0.003125,
            "gemini-1.5-flash" => 0.0001875,
            _ => 0.003125,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires API key
    async fn test_gemini_completion() {
        let provider = GeminiProvider::gemini_pro().unwrap();
        let prompt = LlmPrompt::new("Say 'Hello, World!' and nothing else.".to_string());

        let response = provider.complete(prompt).await.unwrap();
        assert!(response.content.contains("Hello"));
        assert!(response.tokens_used > 0);
    }

    #[tokio::test]
    #[ignore]
    async fn test_gemini_json_mode() {
        let provider = GeminiProvider::gemini_pro().unwrap();
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
    async fn test_gemini_with_system() {
        let provider = GeminiProvider::gemini_pro().unwrap();
        let prompt = LlmPrompt::new("What is your role?".to_string())
            .with_system("You are a helpful security analyst.".to_string());

        let response = provider.complete(prompt).await.unwrap();
        assert!(!response.content.is_empty());
    }
}
