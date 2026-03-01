use strike_security::agents::retest::RetestAgent;
use strike_security::models::finding::{Finding, FindingStatus, Severity};
use strike_security::models::evidence::Evidence;
use strike_security::agents::remediation::Remediation;

#[tokio::test]
async fn test_retest_agent_creation() {
    let agent = RetestAgent::new().await;
    assert!(agent.is_ok());
}

#[tokio::test]
async fn test_verify_fix_structure() {
    let agent = RetestAgent::new().await.unwrap();
    
    let finding = create_test_finding();
    let remediation = Remediation {
        finding_id: finding.id.clone(),
        fix_type: "patch".to_string(),
        code_changes: vec![],
        config_changes: vec![],
        description: "Test fix".to_string(),
        estimated_effort: "1 hour".to_string(),
        risk_level: "low".to_string(),
    };
    
    let result = agent.verify_fix(&finding, &remediation).await;
    
    // Should return a boolean result
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_verify_still_vulnerable_structure() {
    let agent = RetestAgent::new().await.unwrap();
    
    let finding = create_test_finding();
    
    let result = agent.verify_still_vulnerable(&finding).await;
    
    // Should return a boolean result
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_retest_finding_structure() {
    let agent = RetestAgent::new().await.unwrap();
    
    let finding = create_test_finding();
    
    let result = agent.retest_finding(&finding).await;
    
    // Should return a Finding
    assert!(result.is_ok());
}

#[test]
fn test_finding_status_transitions() {
    let mut finding = create_test_finding();
    
    assert_eq!(finding.status, FindingStatus::Open);
    
    finding.status = FindingStatus::Fixed;
    assert_eq!(finding.status, FindingStatus::Fixed);
    
    finding.status = FindingStatus::Verified;
    assert_eq!(finding.status, FindingStatus::Verified);
}

fn create_test_finding() -> Finding {
    Finding {
        id: "test-finding-1".to_string(),
        title: "SQL Injection in /api/users".to_string(),
        description: "SQL injection vulnerability found".to_string(),
        severity: Severity::High,
        confidence: 0.9,
        location: "https://example.com/api/users".to_string(),
        vuln_class: "A03:2021".to_string(),
        cvss_score: Some(8.5),
        cvss_vector: Some("CVSS:4.0/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:N".to_string()),
        cwe_id: Some("CWE-89".to_string()),
        owasp_category: Some("A03:2021 - Injection".to_string()),
        evidence: Some(Evidence {
            proof_of_concept: "' OR '1'='1".to_string(),
            http_trace: None,
            screenshot_path: None,
            code_snippet: None,
            diff: None,
        }),
        remediation_advice: "Use parameterized queries".to_string(),
        references: vec![],
        tags: vec!["sql".to_string(), "injection".to_string()],
        status: FindingStatus::Open,
        discovered_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        verified_at: None,
        false_positive: false,
    }
}
