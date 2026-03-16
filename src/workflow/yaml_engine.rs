use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlWorkflow {
    pub name: String,
    pub description: Option<String>,
    pub version: Option<String>,
    pub author: Option<String>,
    pub tags: Option<Vec<String>>,
    pub metadata: Option<WorkflowMetadata>,
    pub config: Option<WorkflowConfig>,
    pub phases: Vec<WorkflowPhase>,
    pub output: Option<OutputConfig>,
    pub notifications: Option<NotificationConfig>,
    pub roe: Option<RulesOfEngagement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowMetadata {
    pub priority: Option<Priority>,
    pub estimated_duration: Option<u64>,
    pub requires_confirmation: Option<bool>,
    pub safe_mode: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowConfig {
    pub timeout: Option<u64>,
    pub max_concurrent: Option<usize>,
    pub retry_on_failure: Option<bool>,
    pub continue_on_error: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowPhase {
    pub name: String,
    pub description: Option<String>,
    pub enabled: Option<bool>,
    pub timeout: Option<u64>,
    pub depends_on: Option<Vec<String>>,
    pub tools: Option<Vec<ToolConfig>>,
    pub llm_analysis: Option<LlmAnalysisConfig>,
    pub conditions: Option<Vec<Condition>>,
    pub output: Option<PhaseOutputConfig>,
    pub parallel: Option<bool>,
    pub script: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolConfig {
    Simple(String),
    Detailed {
        name: String,
        enabled: Option<bool>,
        timeout: Option<u64>,
        args: Option<Vec<String>>,
        config: Option<HashMap<String, serde_json::Value>>,
        conditions: Option<Vec<Condition>>,
        retry: Option<RetryConfig>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    pub max_attempts: Option<u32>,
    pub backoff: Option<BackoffStrategy>,
    pub delay: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BackoffStrategy {
    Linear,
    Exponential,
    Fixed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmAnalysisConfig {
    pub enabled: bool,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<usize>,
    pub prompt: Option<String>,
    pub context_sources: Option<Vec<String>>,
    pub models: Option<Vec<MultiModelConfig>>,
    pub consensus_required: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiModelConfig {
    pub provider: String,
    pub model: String,
    pub weight: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    #[serde(rename = "type")]
    pub condition_type: String,
    pub value: Option<serde_json::Value>,
    pub operator: Option<String>,
    pub severity: Option<String>,
    pub category: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseOutputConfig {
    pub save: Option<bool>,
    pub format: Option<String>,
    pub filter: Option<FilterConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterConfig {
    pub severity: Option<Vec<String>>,
    pub confidence: Option<Vec<String>>,
    pub exclude_false_positives: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    pub formats: Option<Vec<String>>,
    pub directory: Option<String>,
    pub filename_template: Option<String>,
    pub include_screenshots: Option<bool>,
    pub include_raw_output: Option<bool>,
    pub fail_on: Option<FailOnConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailOnConfig {
    pub severity: Option<String>,
    pub count: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    pub on_start: Option<bool>,
    pub on_completion: Option<bool>,
    pub on_finding: Option<bool>,
    pub on_error: Option<bool>,
    pub channels: Option<Vec<String>>,
    pub webhooks: Option<Vec<WebhookConfig>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    pub url: String,
    pub on: String,
    pub severity: Option<Vec<String>>,
    pub method: Option<String>,
    pub headers: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulesOfEngagement {
    pub scope: Option<Vec<String>>,
    pub excluded: Option<Vec<String>>,
    pub allowed_actions: Option<Vec<String>>,
    pub restricted_actions: Option<Vec<String>>,
    pub rate_limit: Option<RateLimitConfig>,
    pub allowed_times: Option<Vec<TimeWindow>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub requests_per_second: Option<u64>,
    pub concurrent_scans: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeWindow {
    pub start: String,
    pub end: String,
    pub timezone: Option<String>,
    pub days: Option<Vec<String>>,
}

impl YamlWorkflow {
    pub async fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path.as_ref())
            .await
            .context("Failed to read workflow file")?;
        
        Self::from_str(&content)
    }

    pub fn from_str(content: &str) -> Result<Self> {
        let workflow: YamlWorkflow = serde_yaml::from_str(content)
            .context("Failed to parse YAML workflow")?;
        
        workflow.validate()?;
        Ok(workflow)
    }

    pub fn validate(&self) -> Result<()> {
        if self.name.is_empty() {
            return Err(anyhow!("Workflow name is required"));
        }

        if self.phases.is_empty() {
            return Err(anyhow!("Workflow must have at least one phase"));
        }

        // Validate phase names are unique
        let mut phase_names = std::collections::HashSet::new();
        for phase in &self.phases {
            if !phase_names.insert(&phase.name) {
                return Err(anyhow!("Duplicate phase name: {}", phase.name));
            }
        }

        // Validate phase dependencies
        for phase in &self.phases {
            if let Some(deps) = &phase.depends_on {
                for dep in deps {
                    if !phase_names.contains(dep) {
                        return Err(anyhow!(
                            "Phase '{}' depends on non-existent phase '{}'",
                            phase.name,
                            dep
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    pub fn get_phase(&self, name: &str) -> Option<&WorkflowPhase> {
        self.phases.iter().find(|p| p.name == name)
    }

    pub fn get_enabled_phases(&self) -> Vec<&WorkflowPhase> {
        self.phases
            .iter()
            .filter(|p| p.enabled.unwrap_or(true))
            .collect()
    }

    pub fn get_phase_execution_order(&self) -> Result<Vec<&WorkflowPhase>> {
        let mut ordered = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut visiting = std::collections::HashSet::new();

        for phase in &self.phases {
            if !visited.contains(&phase.name) {
                self.visit_phase(phase, &mut ordered, &mut visited, &mut visiting)?;
            }
        }

        Ok(ordered)
    }

    fn visit_phase<'a>(
        &'a self,
        phase: &'a WorkflowPhase,
        ordered: &mut Vec<&'a WorkflowPhase>,
        visited: &mut std::collections::HashSet<String>,
        visiting: &mut std::collections::HashSet<String>,
    ) -> Result<()> {
        if visiting.contains(&phase.name) {
            return Err(anyhow!("Circular dependency detected in phase '{}'", phase.name));
        }

        if visited.contains(&phase.name) {
            return Ok(());
        }

        visiting.insert(phase.name.clone());

        if let Some(deps) = &phase.depends_on {
            for dep_name in deps {
                if let Some(dep_phase) = self.get_phase(dep_name) {
                    self.visit_phase(dep_phase, ordered, visited, visiting)?;
                }
            }
        }

        visiting.remove(&phase.name);
        visited.insert(phase.name.clone());
        ordered.push(phase);

        Ok(())
    }

    pub fn to_yaml(&self) -> Result<String> {
        serde_yaml::to_string(self).context("Failed to serialize workflow to YAML")
    }

    pub async fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let yaml = self.to_yaml()?;
        fs::write(path.as_ref(), yaml)
            .await
            .context("Failed to write workflow file")?;
        Ok(())
    }
}

impl WorkflowPhase {
    pub fn is_enabled(&self) -> bool {
        self.enabled.unwrap_or(true)
    }

    pub fn get_tool_names(&self) -> Vec<String> {
        self.tools
            .as_ref()
            .map(|tools| {
                tools
                    .iter()
                    .map(|t| match t {
                        ToolConfig::Simple(name) => name.clone(),
                        ToolConfig::Detailed { name, .. } => name.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn should_run_llm_analysis(&self) -> bool {
        self.llm_analysis
            .as_ref()
            .map(|config| config.enabled)
            .unwrap_or(false)
    }
}

pub struct WorkflowEngine {
    workflows_dir: std::path::PathBuf,
}

impl WorkflowEngine {
    pub fn new<P: AsRef<Path>>(workflows_dir: P) -> Self {
        Self {
            workflows_dir: workflows_dir.as_ref().to_path_buf(),
        }
    }

    pub async fn load_workflow(&self, name: &str) -> Result<YamlWorkflow> {
        let path = self.workflows_dir.join(format!("{}.yaml", name));
        YamlWorkflow::from_file(path).await
    }

    pub async fn list_workflows(&self) -> Result<Vec<String>> {
        let mut workflows = Vec::new();
        let mut entries = fs::read_dir(&self.workflows_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("yaml") {
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    workflows.push(name.to_string());
                }
            }
        }

        Ok(workflows)
    }

    pub async fn validate_workflow(&self, name: &str) -> Result<()> {
        let workflow = self.load_workflow(name).await?;
        workflow.validate()?;
        info!("Workflow '{}' is valid", name);
        Ok(())
    }

    pub async fn create_workflow(&self, workflow: YamlWorkflow) -> Result<()> {
        workflow.validate()?;
        let path = self.workflows_dir.join(format!("{}.yaml", workflow.name));
        workflow.save(path).await?;
        info!("Created workflow '{}'", workflow.name);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_workflow() {
        let yaml = r#"
name: Test Workflow
description: A test workflow
version: 1.0

phases:
  - name: recon
    tools:
      - subfinder
      - httpx
  
  - name: scan
    depends_on:
      - recon
    tools:
      - nuclei
"#;

        let workflow = YamlWorkflow::from_str(yaml).unwrap();
        assert_eq!(workflow.name, "Test Workflow");
        assert_eq!(workflow.phases.len(), 2);
    }

    #[test]
    fn test_parse_complex_workflow() {
        let yaml = r#"
name: Complex Workflow
description: Advanced security testing
version: 2.0
author: Security Team
tags:
  - web
  - api

metadata:
  priority: high
  estimated_duration: 120
  requires_confirmation: true
  safe_mode: true

config:
  timeout: 3600
  max_concurrent: 5
  retry_on_failure: true

phases:
  - name: reconnaissance
    description: Gather information
    tools:
      - name: subfinder
        args:
          - "-all"
      - name: httpx
        config:
          status_code: true
    
  - name: vulnerability_scanning
    depends_on:
      - reconnaissance
    llm_analysis:
      enabled: true
      model: gpt-4
      prompt: "Analyze findings"

output:
  formats:
    - json
    - html
  directory: ./reports
"#;

        let workflow = YamlWorkflow::from_str(yaml).unwrap();
        assert_eq!(workflow.name, "Complex Workflow");
        assert_eq!(workflow.version, Some("2.0".to_string()));
        assert!(workflow.metadata.is_some());
        assert!(workflow.config.is_some());
    }

    #[test]
    fn test_workflow_validation() {
        let yaml = r#"
name: Invalid Workflow
phases:
  - name: phase1
    depends_on:
      - nonexistent
"#;

        let result = YamlWorkflow::from_str(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_phase_execution_order() {
        let yaml = r#"
name: Ordered Workflow
phases:
  - name: phase3
    depends_on:
      - phase2
  - name: phase1
  - name: phase2
    depends_on:
      - phase1
"#;

        let workflow = YamlWorkflow::from_str(yaml).unwrap();
        let order = workflow.get_phase_execution_order().unwrap();
        
        assert_eq!(order[0].name, "phase1");
        assert_eq!(order[1].name, "phase2");
        assert_eq!(order[2].name, "phase3");
    }

    #[test]
    fn test_circular_dependency_detection() {
        let yaml = r#"
name: Circular Workflow
phases:
  - name: phase1
    depends_on:
      - phase2
  - name: phase2
    depends_on:
      - phase1
"#;

        let workflow = YamlWorkflow::from_str(yaml).unwrap();
        let result = workflow.get_phase_execution_order();
        assert!(result.is_err());
    }

    #[test]
    fn test_tool_config_parsing() {
        let yaml = r#"
name: Tool Config Test
phases:
  - name: test
    tools:
      - simple_tool
      - name: complex_tool
        enabled: true
        timeout: 300
        args:
          - "-v"
          - "--debug"
        config:
          threads: 10
"#;

        let workflow = YamlWorkflow::from_str(yaml).unwrap();
        let phase = &workflow.phases[0];
        let tool_names = phase.get_tool_names();
        
        assert_eq!(tool_names.len(), 2);
        assert_eq!(tool_names[0], "simple_tool");
        assert_eq!(tool_names[1], "complex_tool");
    }
}
