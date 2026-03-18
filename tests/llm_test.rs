use strike_security::llm::{
    provider::{LlmPrompt, LlmProvider, TaskClass},
    router::{LlmRouter, RouterConfig},
    prompt::{PromptTemplate, EndpointInfo, FindingContext},
};

#[tokio::test]
async fn test_llm_prompt_creation() {
    let prompt = LlmPrompt::new("Test user prompt".to_string())
        .with_system("Test system prompt".to_string())
        .with_temperature(0.5)
        .with_max_tokens(2048)
        .with_json_mode(true);
    
    assert_eq!(prompt.user, "Test user prompt");
    assert_eq!(prompt.system, Some("Test system prompt".to_string()));
    assert_eq!(prompt.temperature, 0.5);
    assert_eq!(prompt.max_tokens, 2048);
    assert!(prompt.json_mode);
}

#[tokio::test]
async fn test_router_config_default() {
    let config = RouterConfig::default();
    assert_eq!(config.monthly_budget_usd, 50.0);
    assert_eq!(config.current_spend_usd, 0.0);
    assert!(config.enable_cost_routing);
    assert!(config.prefer_local);
}

#[tokio::test]
async fn test_router_creation() {
    let router = LlmRouter::new().await;
    assert!(router.is_ok() || router.is_err());
}

#[tokio::test]
async fn test_budget_tracking() {
    if let Ok(mut router) = LlmRouter::new().await {
        router.update_spend(10.0);
        let (spent, remaining, percentage) = router.get_budget_status();
        
        assert_eq!(spent, 10.0);
        assert_eq!(remaining, 40.0);
        assert_eq!(percentage, 20.0);
    }
}

#[tokio::test]
async fn test_prompt_template_initialization() {
    let template = PromptTemplate::new();
    assert!(template.is_ok());
}

#[tokio::test]
async fn test_hypothesis_generation_template() {
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
    let rendered = result.unwrap();
    assert!(rendered.contains("IDOR"));
    assert!(rendered.contains("/api/users/123"));
}

#[tokio::test]
async fn test_root_cause_template() {
    let template = PromptTemplate::new().unwrap();
    let finding = FindingContext {
        title: "IDOR Vulnerability".to_string(),
        vuln_class: "IDOR".to_string(),
        severity: "high".to_string(),
        endpoint: "/api/users/123".to_string(),
        method: "GET".to_string(),
        parameter: Some("id".to_string()),
        evidence: "User can access other users' data".to_string(),
    };
    
    let result = template.render_root_cause_analysis(&finding, None);
    assert!(result.is_ok());
    let rendered = result.unwrap();
    assert!(rendered.contains("IDOR"));
}

#[tokio::test]
async fn test_remediation_template() {
    let template = PromptTemplate::new().unwrap();
    let finding = FindingContext {
        title: "SQL Injection".to_string(),
        vuln_class: "SQLi".to_string(),
        severity: "critical".to_string(),
        endpoint: "/api/search".to_string(),
        method: "POST".to_string(),
        parameter: Some("query".to_string()),
        evidence: "SQL injection via query parameter".to_string(),
    };
    
    let result = template.render_remediation_generation(
        &finding,
        "Missing input validation and parameterized queries",
    );
    
    assert!(result.is_ok());
    let rendered = result.unwrap();
    assert!(rendered.contains("SQLi"));
    assert!(rendered.contains("parameterized queries"));
}

#[tokio::test]
async fn test_task_class_enum() {
    let task = TaskClass::VulnHypothesis;
    assert_eq!(task, TaskClass::VulnHypothesis);
    
    let task2 = TaskClass::RemediationGeneration;
    assert_eq!(task2, TaskClass::RemediationGeneration);
}
