use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::provider::{LlmPrompt, LlmProvider, LlmResponse, TaskClass};
use super::anthropic::AnthropicProvider;
use super::openai::OpenAiProvider;
use super::ollama::OllamaProvider;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterConfig {
    pub enable_cost_routing: bool,
    pub monthly_budget_usd: f64,
    pub current_spend_usd: f64,
    pub prefer_local: bool,
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            enable_cost_routing: true,
            monthly_budget_usd: 50.0,
            current_spend_usd: 0.0,
            prefer_local: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LlmRouter {
    anthropic: Option<Arc<AnthropicProvider>>,
    openai: Option<Arc<OpenAiProvider>>,
    ollama: Option<Arc<OllamaProvider>>,
    config: RouterConfig,
}

impl LlmRouter {
    pub async fn new() -> Result<Self> {
        Self::with_config(RouterConfig::default()).await
    }
    
    pub async fn with_config(config: RouterConfig) -> Result<Self> {
        let anthropic = AnthropicProvider::new(None).ok().map(Arc::new);
        let openai = OpenAiProvider::new(None).ok().map(Arc::new);
        let ollama = OllamaProvider::new(None, None).ok().map(Arc::new);
        
        if anthropic.is_none() && openai.is_none() && ollama.is_none() {
            anyhow::bail!("No LLM providers available. Set ANTHROPIC_API_KEY, OPENAI_API_KEY, or ensure Ollama is running.");
        }
        
        Ok(Self {
            anthropic,
            openai,
            ollama,
            config,
        })
    }
    
    pub async fn check_ollama_availability(&self) -> bool {
        if let Some(ollama) = &self.ollama {
            ollama.is_available().await
        } else {
            false
        }
    }
    
    fn select_provider_for_task(&self, task_class: TaskClass) -> Result<Arc<dyn LlmProvider>> {
        if !self.config.enable_cost_routing {
            return self.get_default_provider();
        }
        
        let budget_remaining = self.config.monthly_budget_usd - self.config.current_spend_usd;
        let low_budget = budget_remaining < 5.0;
        
        match task_class {
            TaskClass::ReconSummary | TaskClass::ReportSynthesis => {
                if self.config.prefer_local && self.ollama.is_some() {
                    Ok(self.ollama.as_ref().unwrap().clone() as Arc<dyn LlmProvider>)
                } else if let Some(claude) = &self.anthropic {
                    Ok(claude.clone() as Arc<dyn LlmProvider>)
                } else if let Some(gpt) = &self.openai {
                    Ok(gpt.clone() as Arc<dyn LlmProvider>)
                } else {
                    anyhow::bail!("No suitable provider for low-stakes task")
                }
            }
            
            TaskClass::VulnHypothesis | TaskClass::RootCauseAnalysis | TaskClass::RemediationGeneration => {
                if low_budget && self.ollama.is_some() {
                    Ok(self.ollama.as_ref().unwrap().clone() as Arc<dyn LlmProvider>)
                } else if let Some(claude) = &self.anthropic {
                    Ok(claude.clone() as Arc<dyn LlmProvider>)
                } else if let Some(gpt) = &self.openai {
                    Ok(gpt.clone() as Arc<dyn LlmProvider>)
                } else if let Some(ollama) = &self.ollama {
                    Ok(ollama.clone() as Arc<dyn LlmProvider>)
                } else {
                    anyhow::bail!("No suitable provider for high-stakes task")
                }
            }
        }
    }
    
    fn get_default_provider(&self) -> Result<Arc<dyn LlmProvider>> {
        if let Some(claude) = &self.anthropic {
            Ok(claude.clone() as Arc<dyn LlmProvider>)
        } else if let Some(gpt) = &self.openai {
            Ok(gpt.clone() as Arc<dyn LlmProvider>)
        } else if let Some(ollama) = &self.ollama {
            Ok(ollama.clone() as Arc<dyn LlmProvider>)
        } else {
            anyhow::bail!("No LLM providers available")
        }
    }
    
    pub async fn complete_with_task_class(
        &self,
        prompt: LlmPrompt,
        task_class: TaskClass,
    ) -> Result<LlmResponse> {
        let provider = self.select_provider_for_task(task_class)?;
        provider.complete(prompt).await
    }
    
    pub fn update_spend(&mut self, cost_usd: f64) {
        self.config.current_spend_usd += cost_usd;
    }
    
    pub fn get_budget_status(&self) -> (f64, f64, f64) {
        let remaining = self.config.monthly_budget_usd - self.config.current_spend_usd;
        let percentage = (self.config.current_spend_usd / self.config.monthly_budget_usd) * 100.0;
        (self.config.current_spend_usd, remaining, percentage)
    }

    // Public helper for tests and offline scenarios where providers are not configured
    pub fn for_tests() -> Self {
        Self {
            anthropic: None,
            openai: None,
            ollama: None,
            config: RouterConfig::default(),
        }
    }
}

#[cfg(test)]
impl LlmRouter {
    pub fn test_router() -> Self {
        Self {
            anthropic: None,
            openai: None,
            ollama: None,
            config: RouterConfig::default(),
        }
    }
}

#[async_trait]
impl LlmProvider for LlmRouter {
    async fn complete(&self, prompt: LlmPrompt) -> Result<LlmResponse> {
        let provider = self.get_default_provider()?;
        provider.complete(prompt).await
    }
    
    fn model_id(&self) -> &str {
        "router"
    }
    
    fn token_limit(&self) -> usize {
        128_000
    }
    
    fn estimated_cost_per_1k_tokens(&self) -> f64 {
        0.003
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router_config_default() {
        let config = RouterConfig::default();
        assert_eq!(config.monthly_budget_usd, 50.0);
        assert_eq!(config.current_spend_usd, 0.0);
        assert!(config.enable_cost_routing);
    }
    
    #[test]
    fn test_budget_tracking() {
        let mut router = LlmRouter {
            anthropic: None,
            openai: None,
            ollama: None,
            config: RouterConfig::default(),
        };
        
        router.update_spend(10.0);
        let (spent, remaining, percentage) = router.get_budget_status();
        
        assert_eq!(spent, 10.0);
        assert_eq!(remaining, 40.0);
        assert_eq!(percentage, 20.0);
    }
}
