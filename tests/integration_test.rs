use strike_security::models::*;
use strike_security::storage::*;

#[tokio::test]
async fn test_database_initialization() {
    let database_url = "sqlite::memory:";
    
    let db = Database::new(&database_url).await.unwrap();
    db.initialize().await.unwrap();
    
    assert!(db.pool().is_closed() == false);
}

#[tokio::test]
async fn test_finding_repository() {
    let database_url = "sqlite::memory:";
    
    let db = Database::new(&database_url).await.unwrap();
    db.initialize().await.unwrap();
    
    let run_repo = RunStateRepository::new(db.pool().clone());
    let finding_repo = FindingRepository::new(db.pool().clone());
    
    let run_state = RunState::new(
        "https://example.com".to_string(),
        RunProfile::Web,
        EnvironmentTag::Local,
        RunConfig::default(),
    );
    
    run_repo.save(&run_state).await.unwrap();
    
    let target = Target::new(
        "https://example.com".to_string(),
        "/api/users".to_string(),
        HttpMethod::Get,
    );
    
    let evidence = Evidence::new(
        HttpTrace {
            method: "GET".to_string(),
            url: "https://example.com/api/users".to_string(),
            headers: std::collections::HashMap::new(),
            body: None,
            status_code: None,
            timestamp: chrono::Utc::now(),
        },
        HttpTrace {
            method: "GET".to_string(),
            url: "https://example.com/api/users".to_string(),
            headers: std::collections::HashMap::new(),
            body: Some("response".to_string()),
            status_code: Some(200),
            timestamp: chrono::Utc::now(),
        },
        "TestAgent".to_string(),
    );
    
    let cvss = CvssV4Score::calculate(BaseMetrics {
        attack_vector: AttackVector::Network,
        attack_complexity: AttackComplexity::Low,
        attack_requirements: AttackRequirements::None,
        privileges_required: PrivilegesRequired::None,
        user_interaction: UserInteraction::None,
        confidentiality: Impact::High,
        integrity: Impact::High,
        availability: Impact::None,
    });
    
    let finding = Finding::new(
        run_state.id,
        "Test IDOR Vulnerability".to_string(),
        VulnClass::Idor,
        cvss,
        target,
        evidence,
        Environment {
            tag: EnvironmentTag::Local,
            target_build_sha: None,
            strike_version: "0.1.0".to_string(),
            run_config_hash: "test".to_string(),
        },
        Authorization {
            roe_reference: "test-roe".to_string(),
            authorized_by: "test-user".to_string(),
            authorized_at: chrono::Utc::now(),
        },
    );
    
    finding_repo.save(&finding).await.unwrap();
    
    let retrieved = finding_repo.find_by_id(finding.id).await.unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().title, "Test IDOR Vulnerability");
}

#[tokio::test]
async fn test_run_state_repository() {
    let database_url = "sqlite::memory:";
    
    let db = Database::new(&database_url).await.unwrap();
    db.initialize().await.unwrap();
    
    let repo = RunStateRepository::new(db.pool().clone());
    
    let run_state = RunState::new(
        "https://example.com".to_string(),
        RunProfile::Web,
        EnvironmentTag::Local,
        RunConfig::default(),
    );
    
    repo.save(&run_state).await.unwrap();
    
    let retrieved = repo.find_by_id(run_state.id).await.unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().target, "https://example.com");
}

#[test]
fn test_cvss_scoring() {
    let cvss = CvssV4Score::calculate(BaseMetrics {
        attack_vector: AttackVector::Network,
        attack_complexity: AttackComplexity::Low,
        attack_requirements: AttackRequirements::None,
        privileges_required: PrivilegesRequired::None,
        user_interaction: UserInteraction::None,
        confidentiality: Impact::High,
        integrity: Impact::High,
        availability: Impact::High,
    });
    
    assert!(cvss.score > 0.0);
    assert!(cvss.score <= 10.0);
    assert!(!cvss.vector.is_empty());
}

#[test]
fn test_vuln_class_mappings() {
    let idor = VulnClass::Idor;
    assert_eq!(idor.owasp_top10_mapping(), Some("A01:2021 - Broken Access Control"));
    assert_eq!(idor.cwe_id(), Some(639));
    
    let bola = VulnClass::Bola;
    assert_eq!(bola.owasp_api_top10_mapping(), Some("API1:2023 - Broken Object Level Authorization"));
    
    let sqli = VulnClass::SqlInjection;
    assert_eq!(sqli.owasp_top10_mapping(), Some("A03:2021 - Injection"));
    assert_eq!(sqli.cwe_id(), Some(89));
}

#[test]
fn test_severity_from_cvss() {
    assert_eq!(Severity::from_cvss_score(9.5), Severity::Critical);
    assert_eq!(Severity::from_cvss_score(7.5), Severity::High);
    assert_eq!(Severity::from_cvss_score(5.0), Severity::Medium);
    assert_eq!(Severity::from_cvss_score(2.0), Severity::Low);
    assert_eq!(Severity::from_cvss_score(0.0), Severity::Info);
}
