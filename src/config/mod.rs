use serde::{Deserialize, Serialize};
use anyhow::Result;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrikeConfig {
    pub target: String,
    pub env: String,
    pub profile: String,
    pub workers: u32,
    pub rate_limit: u32,
    pub llm: LlmConfig,
    pub sandbox: SandboxConfig,
    pub output: OutputConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub provider: String,
    pub model: String,
    pub max_tokens_per_agent: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    pub driver: String,
    pub network_allowlist: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    pub dir: String,
    pub formats: Vec<String>,
}

impl StrikeConfig {
    pub async fn load(path: &Path) -> Result<Self> {
        let content = tokio::fs::read_to_string(path).await?;
        let config: StrikeConfig = toml::from_str(&content)?;
        Ok(config)
    }

    pub async fn save(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        tokio::fs::write(path, content).await?;
        Ok(())
    }
}

impl Default for StrikeConfig {
    fn default() -> Self {
        Self {
            target: String::new(),
            env: "local".to_string(),
            profile: "full".to_string(),
            workers: 16,
            rate_limit: 50,
            llm: LlmConfig {
                provider: "anthropic".to_string(),
                model: "claude-sonnet-4-6".to_string(),
                max_tokens_per_agent: 4096,
            },
            sandbox: SandboxConfig {
                driver: "docker".to_string(),
                network_allowlist: Vec::new(),
            },
            output: OutputConfig {
                dir: "./.strike/runs".to_string(),
                formats: vec!["json".to_string(), "md".to_string()],
            },
        }
    }
}
