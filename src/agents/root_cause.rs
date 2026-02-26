use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[cfg(not(test))]
use crate::llm::{
    provider::{LlmPrompt, LlmResponse, TaskClass},
    prompt::{FindingContext, PromptTemplate},
    router::LlmRouter,
};
#[cfg(not(test))]
use crate::models::finding::Finding;

#[cfg(test)]
use strike_security::llm::{
    provider::{LlmPrompt, LlmResponse, TaskClass},
    prompt::{FindingContext, PromptTemplate},
    router::LlmRouter,
};
#[cfg(test)]
use strike_security::models::finding::Finding;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootCauseAnalysis {
    pub code_file: Option<String>,
    pub code_line_range: Option<(usize, usize)>,
    pub vulnerable_function: Option<String>,
    pub taint_path: Option<Vec<String>>,
    pub root_cause_pattern: String,
    pub cwe_id: String,
    pub asvs_control: String,
    pub fix_category: FixCategory,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FixCategory {
    InputValidation,
    AccessControl,
    Auth,
    Crypto,
    Config,
    Other,
}

#[derive(Debug, Clone)]
pub enum AnalysisMode {
    WhiteBox { repo_path: String },
    BlackBox,
}

pub struct RootCauseAgent {
    llm_router: Arc<LlmRouter>,
    prompt_template: PromptTemplate,
    mode: AnalysisMode,
}

impl RootCauseAgent {
    pub fn new(llm_router: Arc<LlmRouter>, mode: AnalysisMode) -> Result<Self> {
        Ok(Self {
            llm_router,
            prompt_template: PromptTemplate::new()?,
            mode,
        })
    }
    
    pub async fn analyze_finding(&self, finding: &Finding) -> Result<RootCauseAnalysis> {
        match &self.mode {
            AnalysisMode::WhiteBox { repo_path } => {
                self.analyze_white_box(finding, repo_path).await
            }
            AnalysisMode::BlackBox => {
                self.analyze_black_box(finding).await
            }
        }
    }
    
    async fn analyze_white_box(
        &self,
        finding: &Finding,
        _repo_path: &str,
    ) -> Result<RootCauseAnalysis> {
        let finding_context = FindingContext {
            title: finding.title.clone(),
            vuln_class: format!("{:?}", finding.vuln_class),
            severity: format!("{:?}", finding.severity),
            endpoint: finding.target.url.clone(),
            method: format!("{:?}", finding.target.method),
            parameter: finding.target.parameter.clone(),
            evidence: "Evidence data".to_string(),
        };
        
        let code_context = Some("// Code analysis not yet implemented");
        
        let prompt_text = self.prompt_template.render_root_cause_analysis(
            &finding_context,
            code_context,
        )?;
        
        let prompt = LlmPrompt::new(prompt_text)
            .with_temperature(0.2)
            .with_max_tokens(2048)
            .with_json_mode(true);
        
        let response: LlmResponse = self.llm_router
            .complete_with_task_class(prompt, TaskClass::RootCauseAnalysis)
            .await
            .context("Failed to get root cause analysis from LLM")?;
        
        let analysis: RootCauseAnalysis = serde_json::from_str(&response.content)
            .context("Failed to parse root cause analysis response")?;
        
        Ok(analysis)
    }
    
    async fn analyze_black_box(&self, finding: &Finding) -> Result<RootCauseAnalysis> {
        let finding_context = FindingContext {
            title: finding.title.clone(),
            vuln_class: format!("{:?}", finding.vuln_class),
            severity: format!("{:?}", finding.severity),
            endpoint: finding.target.url.clone(),
            method: format!("{:?}", finding.target.method),
            parameter: finding.target.parameter.clone(),
            evidence: "Evidence data".to_string(),
        };
        
        let prompt_text = self.prompt_template.render_root_cause_analysis(
            &finding_context,
            None,
        )?;
        
        let prompt = LlmPrompt::new(prompt_text)
            .with_temperature(0.2)
            .with_max_tokens(2048)
            .with_json_mode(true);
        
        let response: LlmResponse = self.llm_router
            .complete_with_task_class(prompt, TaskClass::RootCauseAnalysis)
            .await
            .context("Failed to get root cause analysis from LLM")?;
        
        let analysis: RootCauseAnalysis = serde_json::from_str(&response.content)
            .context("Failed to parse root cause analysis response")?;
        
        Ok(analysis)
    }
    
    pub fn infer_cwe_from_vuln_class(vuln_class: &str) -> String {
        match vuln_class {
            "IDOR" | "BOLA" => "CWE-639".to_string(),
            "SQLi" => "CWE-89".to_string(),
            "XSS" => "CWE-79".to_string(),
            "SSRF" => "CWE-918".to_string(),
            "AuthBypass" => "CWE-287".to_string(),
            "CSRF" => "CWE-352".to_string(),
            "PathTraversal" => "CWE-22".to_string(),
            "XXE" => "CWE-611".to_string(),
            "Deserialization" => "CWE-502".to_string(),
            "CommandInjection" => "CWE-78".to_string(),
            _ => "CWE-1000".to_string(),
        }
    }
    
    pub fn infer_asvs_control(vuln_class: &str) -> String {
        match vuln_class {
            "IDOR" | "BOLA" => "ASVS 4.1.1".to_string(),
            "SQLi" => "ASVS 5.3.4".to_string(),
            "XSS" => "ASVS 5.3.3".to_string(),
            "SSRF" => "ASVS 12.5.1".to_string(),
            "AuthBypass" => "ASVS 2.1.1".to_string(),
            "CSRF" => "ASVS 4.2.2".to_string(),
            "PathTraversal" => "ASVS 12.3.1".to_string(),
            "XXE" => "ASVS 5.5.2".to_string(),
            "Deserialization" => "ASVS 5.5.3".to_string(),
            "CommandInjection" => "ASVS 5.3.8".to_string(),
            _ => "ASVS 1.1.1".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use strike_security::llm::router::RouterConfig;
    use strike_security::models::evidence::{ProofOfConcept, Target};

    #[test]
    fn test_cwe_inference() {
        assert_eq!(RootCauseAgent::infer_cwe_from_vuln_class("IDOR"), "CWE-639");
        assert_eq!(RootCauseAgent::infer_cwe_from_vuln_class("SQLi"), "CWE-89");
        assert_eq!(RootCauseAgent::infer_cwe_from_vuln_class("XSS"), "CWE-79");
    }
    
    #[test]
    fn test_asvs_inference() {
        assert_eq!(RootCauseAgent::infer_asvs_control("IDOR"), "ASVS 4.1.1");
        assert_eq!(RootCauseAgent::infer_asvs_control("SQLi"), "ASVS 5.3.4");
        assert_eq!(RootCauseAgent::infer_asvs_control("XSS"), "ASVS 5.3.3");
    }
    
    #[test]
    fn test_root_cause_agent_creation() {
        let config = RouterConfig::default();
        if let Ok(router) = LlmRouter::new(config) {
            let agent = RootCauseAgent::new(
                Arc::new(router),
                AnalysisMode::BlackBox,
            );
            assert!(agent.is_ok());
        }
    }
    
    #[test]
    fn test_fix_category_serialization() {
        let category = FixCategory::InputValidation;
        let json = serde_json::to_string(&category).unwrap();
        assert_eq!(json, "\"input_validation\"");
        
        let deserialized: FixCategory = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, FixCategory::InputValidation);
    }
}
