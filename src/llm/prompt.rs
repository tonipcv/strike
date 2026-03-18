use anyhow::{Context, Result};
use handlebars::Handlebars;
use serde::Serialize;
use serde_json::json;

#[derive(Clone)]
pub struct PromptTemplate {
    handlebars: Handlebars<'static>,
}

impl PromptTemplate {
    pub fn new() -> Result<Self> {
        let mut handlebars = Handlebars::new();
        
        handlebars.register_template_string(
            "hypothesis_generation",
            include_str!("templates/hypothesis_generation.hbs")
        )?;
        
        handlebars.register_template_string(
            "root_cause_analysis",
            include_str!("templates/root_cause_analysis.hbs")
        )?;
        
        handlebars.register_template_string(
            "remediation_generation",
            include_str!("templates/remediation_generation.hbs")
        )?;
        
        handlebars.register_template_string(
            "recon_summary",
            include_str!("templates/recon_summary.hbs")
        )?;
        
        handlebars.register_template_string(
            "finding_triage",
            include_str!("templates/finding_triage.hbs")
        )?;
        
        Ok(Self { handlebars })
    }
    
    pub fn render<T: Serialize>(&self, template_name: &str, data: &T) -> Result<String> {
        self.handlebars
            .render(template_name, data)
            .context(format!("Failed to render template: {}", template_name))
    }
    
    pub fn render_hypothesis_generation(
        &self,
        endpoints: &[EndpointInfo],
        auth_context: Option<&str>,
        focus_classes: &[String],
    ) -> Result<String> {
        let data = json!({
            "endpoints": endpoints,
            "auth_context": auth_context,
            "focus_classes": focus_classes,
        });
        
        self.render("hypothesis_generation", &data)
    }
    
    pub fn render_root_cause_analysis(
        &self,
        finding: &FindingContext,
        code_context: Option<&str>,
    ) -> Result<String> {
        let data = json!({
            "finding": finding,
            "code_context": code_context,
        });
        
        self.render("root_cause_analysis", &data)
    }
    
    pub fn render_remediation_generation(
        &self,
        finding: &FindingContext,
        root_cause: &str,
    ) -> Result<String> {
        let data = json!({
            "finding": finding,
            "root_cause": root_cause,
        });
        
        self.render("remediation_generation", &data)
    }
}

impl Default for PromptTemplate {
    fn default() -> Self {
        Self::new().expect("Failed to initialize PromptTemplate")
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct EndpointInfo {
    pub url: String,
    pub method: String,
    pub parameters: Vec<String>,
    pub auth_required: bool,
    pub response_codes: Vec<u16>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FindingContext {
    pub title: String,
    pub vuln_class: String,
    pub severity: String,
    pub endpoint: String,
    pub method: String,
    pub parameter: Option<String>,
    pub evidence: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_initialization() {
        let template = PromptTemplate::new();
        assert!(template.is_ok());
    }
    
    #[test]
    fn test_render_with_data() {
        let template = PromptTemplate::new().unwrap();
        let endpoints = vec![
            EndpointInfo {
                url: "/api/users/123".to_string(),
                method: "GET".to_string(),
                parameters: vec!["id".to_string()],
                auth_required: true,
                response_codes: vec![200, 401],
            }
        ];
        
        let result = template.render_hypothesis_generation(
            &endpoints,
            Some("Bearer token"),
            &vec!["IDOR".to_string()],
        );
        
        assert!(result.is_ok());
    }
}
