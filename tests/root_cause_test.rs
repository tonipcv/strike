use strike_security::agents::root_cause::RootCauseAgent;
use strike_security::models::finding::{Finding, FindingStatus, Severity};
use strike_security::models::evidence::Evidence;

#[tokio::test]
async fn test_root_cause_agent_creation() {
    let agent = RootCauseAgent::new().await;
    assert!(agent.is_ok());
}

#[tokio::test]
async fn test_analyze_whitebox_sql_injection() {
    let agent = RootCauseAgent::new().await.unwrap();
    
    let finding = create_sql_injection_finding();
    let source_code = r#"
        fn get_user(id: &str) -> Result<User> {
            let query = format!("SELECT * FROM users WHERE id = '{}'", id);
            database.execute(&query)
        }
    "#;
    
    let result = agent.analyze(&finding, Some(source_code)).await;
    
    assert!(result.is_ok());
    if let Ok(analysis) = result {
        assert!(analysis.confidence > 0.0);
        assert!(!analysis.affected_code.is_empty());
    }
}

#[tokio::test]
async fn test_analyze_whitebox_xss() {
    let agent = RootCauseAgent::new().await.unwrap();
    
    let mut finding = create_sql_injection_finding();
    finding.title = "XSS in user profile".to_string();
    
    let source_code = r#"
        function displayName(name) {
            document.getElementById('profile').innerHTML = name;
        }
    "#;
    
    let result = agent.analyze(&finding, Some(source_code)).await;
    
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_analyze_whitebox_ssrf() {
    let agent = RootCauseAgent::new().await.unwrap();
    
    let mut finding = create_sql_injection_finding();
    finding.title = "SSRF in webhook".to_string();
    
    let source_code = r#"
        async fn fetch_url(url: &str) -> Result<String> {
            let response = http.get(url).await?;
            Ok(response.text().await?)
        }
    "#;
    
    let result = agent.analyze(&finding, Some(source_code)).await;
    
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_analyze_blackbox() {
    let agent = RootCauseAgent::new().await.unwrap();
    
    let finding = create_sql_injection_finding();
    
    let result = agent.analyze(&finding, None).await;
    
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_root_cause_analysis_structure() {
    let agent = RootCauseAgent::new().await.unwrap();
    
    let finding = create_sql_injection_finding();
    let source_code = "fn test() { execute(query) }";
    
    let result = agent.analyze(&finding, Some(source_code)).await;
    
    if let Ok(analysis) = result {
        assert!(analysis.confidence >= 0.0 && analysis.confidence <= 1.0);
        assert!(!analysis.description.is_empty());
    }
}

#[tokio::test]
async fn test_data_flow_tracking() {
    let agent = RootCauseAgent::new().await.unwrap();
    
    let finding = create_sql_injection_finding();
    let source_code = r#"
        fn handler(request: Request) -> Response {
            let input = request.get_param("id");
            let result = query_database(input);
            return response.json(result);
        }
    "#;
    
    let result = agent.analyze(&finding, Some(source_code)).await;
    
    if let Ok(analysis) = result {
        assert!(!analysis.data_flow.is_empty());
    }
}

fn create_sql_injection_finding() -> Finding {
    Finding {
        id: uuid::Uuid::new_v4(),
        run_id: uuid::Uuid::new_v4(),
        timestamp: chrono::Utc::now(),
        title: "SQL Injection in /api/users".to_string(),
        vuln_class: strike_security::models::VulnClass::SqlInjection,
        severity: Severity::High,
        cvss_v4_score: strike_security::models::CvssV4Score::default(),
        status: FindingStatus::Open,
        target: strike_security::models::Target {
            base_url: "https://example.com".to_string(),
            endpoint: "/api/users".to_string(),
            method: "POST".to_string(),
        },
        evidence: Evidence {
            proof_of_concept: "' OR '1'='1".to_string(),
            http_trace: None,
            screenshot_path: None,
            code_snippet: None,
            diff: None,
        },
        root_cause: None,
        remediation: strike_security::models::finding::Remediation {
            summary: "Use parameterized queries".to_string(),
            steps: vec![],
            code_example: None,
            references: vec![],
        },
        environment: strike_security::models::finding::Environment {
            os: "Linux".to_string(),
            runtime: "Node.js".to_string(),
            dependencies: vec![],
        },
        authorization: strike_security::models::finding::Authorization {
            required: false,
            level: "none".to_string(),
        },
        retest_history: vec![],
        human_review: None,
    }
}
