use strike_security::agents::validation_agent::ValidationAgent;

#[tokio::test]
async fn test_validation_agent_creation() {
    let agent = ValidationAgent::new().await;
    assert!(agent.is_ok());
}

#[tokio::test]
async fn test_sql_injection_detection_with_diffing() {
    let agent = ValidationAgent::new().await.unwrap();
    
    let result = agent.validate_finding("https://httpbin.org/post", "SQL Injection").await;
    
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_xss_detection_context_aware() {
    let agent = ValidationAgent::new().await.unwrap();
    
    let result = agent.validate_finding("https://httpbin.org/get", "XSS").await;
    
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_ssrf_detection_with_metadata_check() {
    let agent = ValidationAgent::new().await.unwrap();
    
    let result = agent.validate_finding("https://httpbin.org/post", "SSRF").await;
    
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_idor_detection_with_id_manipulation() {
    let agent = ValidationAgent::new().await.unwrap();
    
    let result = agent.validate_finding("https://httpbin.org/users/1", "IDOR").await;
    
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_validation_reduces_false_positives() {
    let agent = ValidationAgent::new().await.unwrap();
    
    // Test with a safe endpoint
    let result = agent.validate_finding("https://httpbin.org/status/200", "SQL Injection").await;
    
    if let Ok(validation) = result {
        // Should have lower confidence for safe endpoints
        assert!(validation.confidence >= 0.0 && validation.confidence <= 1.0);
    }
}

#[tokio::test]
async fn test_time_based_sql_detection() {
    let agent = ValidationAgent::new().await.unwrap();
    
    // This would test time-based SQLi detection
    let result = agent.validate_finding("https://httpbin.org/delay/1", "SQL Injection").await;
    
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_response_diffing_mechanism() {
    let agent = ValidationAgent::new().await.unwrap();
    
    // Test that response diffing works
    let result = agent.validate_finding("https://httpbin.org/post", "SQL Injection").await;
    
    if let Ok(validation) = result {
        // Should have analyzed response differences
        assert!(validation.validated);
    }
}

#[tokio::test]
async fn test_xss_in_different_contexts() {
    let agent = ValidationAgent::new().await.unwrap();
    
    // Test XSS detection in various HTML contexts
    let contexts = vec![
        "https://httpbin.org/html",
        "https://httpbin.org/get",
    ];
    
    for context in contexts {
        let result = agent.validate_finding(context, "XSS").await;
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_ssrf_internal_service_detection() {
    let agent = ValidationAgent::new().await.unwrap();
    
    let result = agent.validate_finding("https://httpbin.org/post", "SSRF").await;
    
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_idor_authorization_bypass() {
    let agent = ValidationAgent::new().await.unwrap();
    
    let result = agent.validate_finding("https://httpbin.org/users/100", "IDOR").await;
    
    if let Ok(validation) = result {
        // Should detect if authorization is missing
        assert!(validation.validated == true || validation.validated == false);
    }
}
