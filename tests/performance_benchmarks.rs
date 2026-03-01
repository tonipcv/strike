use strike_security::llm::cache::{CacheConfig, LlmCache};
use strike_security::llm::provider::LlmPrompt;
use strike_security::tools::http_client::{HttpClient, HttpClientConfig};
use std::time::Instant;

#[tokio::test]
async fn test_llm_cache_performance() {
    let cache = LlmCache::new(CacheConfig::default());
    let prompt = LlmPrompt::new("test prompt".to_string());
    
    let start = Instant::now();
    
    for i in 0..1000 {
        let key_prompt = LlmPrompt::new(format!("prompt {}", i));
        let response = strike_security::llm::provider::LlmResponse {
            content: format!("response {}", i),
            model: "test".to_string(),
            tokens_used: 100,
            finish_reason: "stop".to_string(),
            cost_usd: 0.001,
        };
        
        cache.put("test", "model", &key_prompt, response).await;
    }
    
    let duration = start.elapsed();
    
    assert!(duration.as_millis() < 5000);
}

#[tokio::test]
async fn test_llm_cache_retrieval_performance() {
    let cache = LlmCache::new(CacheConfig::default());
    let prompt = LlmPrompt::new("test prompt".to_string());
    
    let response = strike_security::llm::provider::LlmResponse {
        content: "test response".to_string(),
        model: "test".to_string(),
        tokens_used: 100,
        finish_reason: "stop".to_string(),
        cost_usd: 0.001,
    };
    
    cache.put("test", "model", &prompt, response).await;
    
    let start = Instant::now();
    
    for _ in 0..10000 {
        let _ = cache.get("test", "model", &prompt).await;
    }
    
    let duration = start.elapsed();
    
    assert!(duration.as_millis() < 1000);
}

#[tokio::test]
async fn test_http_client_concurrent_requests() {
    let config = HttpClientConfig::default();
    let client = HttpClient::with_config(config).unwrap();
    
    let start = Instant::now();
    
    let mut handles = vec![];
    
    for _ in 0..10 {
        let client_clone = client.clone();
        let handle = tokio::spawn(async move {
            let _ = client_clone.get("https://httpbin.org/delay/1").await;
        });
        handles.push(handle);
    }
    
    for handle in handles {
        let _ = handle.await;
    }
    
    let duration = start.elapsed();
    
    assert!(duration.as_secs() < 5);
}

#[test]
fn test_secret_scanner_performance() {
    let scanner = strike_security::tools::secret_scanner::SecretScanner::new().unwrap();
    
    let large_text = "AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE\n".repeat(1000);
    
    let start = Instant::now();
    let findings = scanner.scan_text(&large_text);
    let duration = start.elapsed();
    
    assert!(!findings.is_empty());
    assert!(duration.as_millis() < 1000);
}

#[test]
fn test_input_validation_performance() {
    let validator = strike_security::config::validation::InputValidator::new(false);
    
    let start = Instant::now();
    
    for i in 0..10000 {
        let url = format!("https://example.com/api/{}", i);
        let _ = validator.validate_url(&url);
    }
    
    let duration = start.elapsed();
    
    assert!(duration.as_millis() < 500);
}

#[test]
fn test_graphql_fuzzer_generation_performance() {
    let fuzzer = strike_security::tools::api_fuzzer::ApiFuzzer::new(
        strike_security::tools::http_client::HttpClient::new(50, 30).unwrap()
    );
    
    let introspection = serde_json::json!({
        "data": {
            "__schema": {
                "types": []
            }
        }
    });
    
    let start = Instant::now();
    let requests = fuzzer.fuzz_from_graphql(&introspection);
    let duration = start.elapsed();
    
    assert!(!requests.is_empty());
    assert!(duration.as_millis() < 100);
}

#[test]
fn test_shell_completion_generation_performance() {
    use strike_security::cli::completions::{CompletionGenerator, Shell};
    
    let start = Instant::now();
    
    for _ in 0..1000 {
        let _ = CompletionGenerator::generate(Shell::Bash);
        let _ = CompletionGenerator::generate(Shell::Zsh);
        let _ = CompletionGenerator::generate(Shell::Fish);
    }
    
    let duration = start.elapsed();
    
    assert!(duration.as_millis() < 1000);
}

#[test]
fn test_executive_report_generation_performance() {
    use strike_security::reporting::executive::*;
    
    let summary = ExecutiveSummary {
        title: "Test Report".to_string(),
        scan_date: "2026-02-29".to_string(),
        target: "https://example.com".to_string(),
        overall_risk: RiskLevel::Medium,
        key_findings: vec![],
        metrics: ScanMetrics {
            total_endpoints: 100,
            endpoints_scanned: 100,
            vulnerabilities_found: 10,
            critical_count: 1,
            high_count: 2,
            medium_count: 3,
            low_count: 4,
            scan_duration_seconds: 300,
            coverage_percentage: 100.0,
        },
        recommendations: vec![],
        compliance_status: ComplianceStatus {
            owasp_top_10: ComplianceScore {
                compliant: true,
                score_percentage: 90.0,
                issues_found: 1,
                recommendations: vec![],
            },
            pci_dss: ComplianceScore {
                compliant: true,
                score_percentage: 95.0,
                issues_found: 0,
                recommendations: vec![],
            },
            gdpr: ComplianceScore {
                compliant: true,
                score_percentage: 92.0,
                issues_found: 1,
                recommendations: vec![],
            },
            hipaa: ComplianceScore {
                compliant: true,
                score_percentage: 93.0,
                issues_found: 1,
                recommendations: vec![],
            },
        },
    };
    
    let start = Instant::now();
    
    for _ in 0..100 {
        let _ = ExecutiveReportGenerator::generate(&summary);
        let _ = ExecutiveReportGenerator::generate_html(&summary);
    }
    
    let duration = start.elapsed();
    
    assert!(duration.as_millis() < 500);
}

#[test]
fn test_websocket_config_creation_performance() {
    use strike_security::tools::websocket::WebSocketConfig;
    
    let start = Instant::now();
    
    for _ in 0..100000 {
        let _ = WebSocketConfig::default();
    }
    
    let duration = start.elapsed();
    
    assert!(duration.as_millis() < 100);
}

#[test]
fn test_scan_diff_performance() {
    use strike_security::ci::incremental::IncrementalScanner;
    
    let scanner = IncrementalScanner::new().unwrap();
    
    let old_endpoints: Vec<String> = (0..1000).map(|i| format!("/api/endpoint{}", i)).collect();
    let new_endpoints: Vec<String> = (500..1500).map(|i| format!("/api/endpoint{}", i)).collect();
    
    let start = Instant::now();
    let diff = scanner.compare_endpoints(&old_endpoints, &new_endpoints);
    let duration = start.elapsed();
    
    assert!(diff.total_changes() > 0);
    assert!(duration.as_millis() < 100);
}
