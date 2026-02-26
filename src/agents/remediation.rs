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
#[cfg(not(test))]
use crate::agents::root_cause::RootCauseAnalysis;

#[cfg(test)]
use strike_security::llm::{
    provider::{LlmPrompt, LlmResponse, TaskClass},
    prompt::{FindingContext, PromptTemplate},
    router::LlmRouter,
};
#[cfg(test)]
use strike_security::models::finding::Finding;
#[cfg(test)]
use strike_security::agents::root_cause::RootCauseAnalysis;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemediationGuidance {
    pub summary: String,
    pub fix_steps: Vec<String>,
    pub code_diff: Option<String>,
    pub code_example: String,
    pub library_recommendations: Vec<String>,
    pub verification_checklist: Vec<String>,
    pub references: RemediationReferences,
    pub estimated_fix_effort: FixEffort,
    pub regression_test_hint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemediationReferences {
    pub owasp: String,
    pub asvs: String,
    pub cwe: String,
    pub additional: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FixEffort {
    Minutes,
    Hours,
    Days,
}

pub struct RemediationAgent {
    llm_router: Arc<LlmRouter>,
    prompt_template: PromptTemplate,
}

impl RemediationAgent {
    pub fn new(llm_router: Arc<LlmRouter>) -> Result<Self> {
        Ok(Self {
            llm_router,
            prompt_template: PromptTemplate::new()?,
        })
    }
    
    pub async fn generate_remediation(
        &self,
        finding: &Finding,
        root_cause: &RootCauseAnalysis,
    ) -> Result<RemediationGuidance> {
        let finding_context = FindingContext {
            title: finding.title.clone(),
            vuln_class: format!("{:?}", finding.vuln_class),
            severity: format!("{:?}", finding.severity),
            endpoint: finding.target.url.clone(),
            method: format!("{:?}", finding.target.method),
            parameter: finding.target.parameter.clone(),
            evidence: "Evidence data".to_string(),
        };
        
        let root_cause_text = format!(
            "Root cause: {}\nCWE: {}\nASVS: {}\nFix category: {:?}",
            root_cause.root_cause_pattern,
            root_cause.cwe_id,
            root_cause.asvs_control,
            root_cause.fix_category
        );
        
        let prompt_text = self.prompt_template.render_remediation_generation(
            &finding_context,
            &root_cause_text,
        )?;
        
        let prompt = LlmPrompt::new(prompt_text)
            .with_temperature(0.3)
            .with_max_tokens(3072)
            .with_json_mode(true);
        
        let response: LlmResponse = self.llm_router
            .complete_with_task_class(prompt, TaskClass::RemediationGeneration)
            .await
            .context("Failed to generate remediation guidance from LLM")?;
        
        let guidance: RemediationGuidance = serde_json::from_str(&response.content)
            .context("Failed to parse remediation guidance response")?;
        
        Ok(guidance)
    }
    
    pub fn generate_quick_fix_summary(vuln_class: &str) -> String {
        match vuln_class {
            "IDOR" | "BOLA" => {
                "Implement proper authorization checks to verify user owns the requested resource".to_string()
            }
            "SQLi" => {
                "Use parameterized queries or prepared statements instead of string concatenation".to_string()
            }
            "XSS" => {
                "Sanitize and encode all user input before rendering in HTML context".to_string()
            }
            "SSRF" => {
                "Validate and whitelist allowed destination URLs, disable redirects".to_string()
            }
            "AuthBypass" => {
                "Enforce authentication checks on all protected endpoints".to_string()
            }
            "CSRF" => {
                "Implement CSRF tokens for all state-changing operations".to_string()
            }
            "PathTraversal" => {
                "Validate file paths and use allowlist of permitted directories".to_string()
            }
            "XXE" => {
                "Disable external entity processing in XML parsers".to_string()
            }
            "Deserialization" => {
                "Avoid deserializing untrusted data or use safe serialization formats".to_string()
            }
            "CommandInjection" => {
                "Avoid executing shell commands with user input, use safe APIs".to_string()
            }
            _ => {
                "Review security best practices for this vulnerability class".to_string()
            }
        }
    }
    
    pub fn get_owasp_reference(vuln_class: &str) -> String {
        match vuln_class {
            "IDOR" | "BOLA" => {
                "https://owasp.org/Top10/A01_2021-Broken_Access_Control/".to_string()
            }
            "SQLi" => {
                "https://owasp.org/Top10/A03_2021-Injection/".to_string()
            }
            "XSS" => {
                "https://owasp.org/Top10/A03_2021-Injection/".to_string()
            }
            "SSRF" => {
                "https://owasp.org/Top10/A10_2021-Server-Side_Request_Forgery_(SSRF)/".to_string()
            }
            "AuthBypass" => {
                "https://owasp.org/Top10/A07_2021-Identification_and_Authentication_Failures/".to_string()
            }
            "CSRF" => {
                "https://owasp.org/www-community/attacks/csrf".to_string()
            }
            _ => {
                "https://owasp.org/www-project-top-ten/".to_string()
            }
        }
    }
    
    pub fn get_cwe_reference(cwe_id: &str) -> String {
        format!("https://cwe.mitre.org/data/definitions/{}.html", 
                cwe_id.trim_start_matches("CWE-"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use strike_security::llm::router::RouterConfig;

    #[test]
    fn test_quick_fix_summary() {
        let summary = RemediationAgent::generate_quick_fix_summary("IDOR");
        assert!(summary.contains("authorization"));
        
        let summary = RemediationAgent::generate_quick_fix_summary("SQLi");
        assert!(summary.contains("parameterized"));
    }
    
    #[test]
    fn test_owasp_reference() {
        let reference = RemediationAgent::get_owasp_reference("IDOR");
        assert!(reference.contains("owasp.org"));
        assert!(reference.contains("A01_2021"));
    }
    
    #[test]
    fn test_cwe_reference() {
        let reference = RemediationAgent::get_cwe_reference("CWE-639");
        assert_eq!(reference, "https://cwe.mitre.org/data/definitions/639.html");
    }
    
    #[test]
    fn test_remediation_agent_creation() {
        let config = RouterConfig::default();
        if let Ok(router) = LlmRouter::new(config) {
            let agent = RemediationAgent::new(Arc::new(router));
            assert!(agent.is_ok());
        }
    }
    
    #[test]
    fn test_fix_effort_serialization() {
        let effort = FixEffort::Hours;
        let json = serde_json::to_string(&effort).unwrap();
        assert_eq!(json, "\"hours\"");
        
        let deserialized: FixEffort = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, FixEffort::Hours);
    }
}
