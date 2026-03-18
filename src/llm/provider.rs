use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmPrompt {
    pub system: Option<String>,
    pub user: String,
    pub temperature: f32,
    pub max_tokens: usize,
    pub json_mode: bool,
    pub stop_sequences: Vec<String>,
}

impl LlmPrompt {
    pub fn new(user: String) -> Self {
        Self {
            system: None,
            user,
            temperature: 0.7,
            max_tokens: 4096,
            json_mode: false,
            stop_sequences: vec![],
        }
    }

    pub fn with_system(mut self, system: String) -> Self {
        self.system = Some(system);
        self
    }

    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature;
        self
    }

    pub fn with_max_tokens(mut self, max_tokens: usize) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    pub fn with_json_mode(mut self, json_mode: bool) -> Self {
        self.json_mode = json_mode;
        self
    }

    pub fn with_stop_sequences(mut self, stop_sequences: Vec<String>) -> Self {
        self.stop_sequences = stop_sequences;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    pub content: String,
    pub model: String,
    pub tokens_used: usize,
    pub finish_reason: String,
    pub cost_usd: f64,
}

impl LlmResponse {
    pub fn parse_json<T: for<'de> Deserialize<'de>>(&self) -> Result<T> {
        let json: T = serde_json::from_str(&self.content)?;
        Ok(json)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskClass {
    ReconSummary,
    VulnHypothesis,
    RootCauseAnalysis,
    RemediationGeneration,
    ReportSynthesis,
}

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn complete(&self, prompt: LlmPrompt) -> Result<LlmResponse>;
    
    fn model_id(&self) -> &str;
    fn token_limit(&self) -> usize;
    fn estimated_cost_per_1k_tokens(&self) -> f64;
    
    fn estimate_cost(&self, tokens: usize) -> f64 {
        (tokens as f64 / 1000.0) * self.estimated_cost_per_1k_tokens()
    }
}

pub async fn complete_structured<T, P>(
    provider: &P,
    prompt: LlmPrompt,
) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
    P: LlmProvider + ?Sized,
{
    let prompt_with_json = prompt.with_json_mode(true);
    let response = provider.complete(prompt_with_json).await?;
    response.parse_json()
}
