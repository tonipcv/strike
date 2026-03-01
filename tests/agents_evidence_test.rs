use strike_security::agents::evidence_agent::{EvidenceAgent, EvidenceQuality};
use strike_security::models::evidence::{Evidence, HttpTrace};
use std::collections::HashMap;
use chrono::Utc;

#[test]
fn test_evidence_quality_scoring() {
    let high_quality = EvidenceQuality {
        completeness: 0.95,
        reproducibility: 0.9,
        clarity: 0.85,
        impact_demonstration: 0.9,
        overall_score: 0.9,
    };
    
    assert!(high_quality.overall_score > 0.8);
    assert!(high_quality.completeness > 0.9);
}

#[test]
fn test_evidence_agent_creation() {
    let agent = EvidenceAgent::new();
    assert!(agent.is_ok());
}

#[tokio::test]
async fn test_evidence_validation() {
    let agent = EvidenceAgent::new().unwrap();
    
    let request = HttpTrace {
        method: "GET".to_string(),
        url: "https://example.com/api/users/123".to_string(),
        headers: HashMap::new(),
        body: None,
        status_code: None,
        timestamp: Utc::now(),
    };
    
    let response = HttpTrace {
        method: "GET".to_string(),
        url: "https://example.com/api/users/123".to_string(),
        headers: HashMap::new(),
        body: Some("{\"id\": 123, \"name\": \"Alice\"}".to_string()),
        status_code: Some(200),
        timestamp: Utc::now(),
    };
    
    let evidence = Evidence::new(request, response, "TestAgent".to_string());
    
    let quality = agent.assess_quality(&evidence).await;
    assert!(quality.is_ok());
}

#[tokio::test]
async fn test_evidence_completeness_check() {
    let agent = EvidenceAgent::new().unwrap();
    
    let complete_request = HttpTrace {
        method: "POST".to_string(),
        url: "https://example.com/api/login".to_string(),
        headers: {
            let mut h = HashMap::new();
            h.insert("Content-Type".to_string(), "application/json".to_string());
            h
        },
        body: Some("{\"username\":\"test\",\"password\":\"test\"}".to_string()),
        status_code: None,
        timestamp: Utc::now(),
    };
    
    let complete_response = HttpTrace {
        method: "POST".to_string(),
        url: "https://example.com/api/login".to_string(),
        headers: {
            let mut h = HashMap::new();
            h.insert("Set-Cookie".to_string(), "session=abc123".to_string());
            h
        },
        body: Some("{\"token\":\"xyz\"}".to_string()),
        status_code: Some(200),
        timestamp: Utc::now(),
    };
    
    let evidence = Evidence::new(complete_request, complete_response, "TestAgent".to_string());
    let quality = agent.assess_quality(&evidence).await.unwrap();
    
    assert!(quality.completeness > 0.5);
}

#[tokio::test]
async fn test_evidence_reproducibility() {
    let agent = EvidenceAgent::new().unwrap();
    
    let request = HttpTrace {
        method: "GET".to_string(),
        url: "https://example.com/api/users/123".to_string(),
        headers: HashMap::new(),
        body: None,
        status_code: None,
        timestamp: Utc::now(),
    };
    
    let response = HttpTrace {
        method: "GET".to_string(),
        url: "https://example.com/api/users/123".to_string(),
        headers: HashMap::new(),
        body: Some("{\"id\": 123}".to_string()),
        status_code: Some(200),
        timestamp: Utc::now(),
    };
    
    let evidence = Evidence::new(request, response, "TestAgent".to_string());
    let quality = agent.assess_quality(&evidence).await.unwrap();
    
    assert!(quality.reproducibility >= 0.0);
}

#[test]
fn test_evidence_timestamp_ordering() {
    let request = HttpTrace {
        method: "GET".to_string(),
        url: "https://example.com/api/test".to_string(),
        headers: HashMap::new(),
        body: None,
        status_code: None,
        timestamp: Utc::now(),
    };
    
    let response = HttpTrace {
        method: "GET".to_string(),
        url: "https://example.com/api/test".to_string(),
        headers: HashMap::new(),
        body: Some("response".to_string()),
        status_code: Some(200),
        timestamp: Utc::now(),
    };
    
    let evidence = Evidence::new(request.clone(), response.clone(), "TestAgent".to_string());
    
    assert!(evidence.response.timestamp >= evidence.request.timestamp);
}

#[test]
fn test_evidence_agent_name() {
    let request = HttpTrace {
        method: "GET".to_string(),
        url: "https://example.com/api/test".to_string(),
        headers: HashMap::new(),
        body: None,
        status_code: None,
        timestamp: Utc::now(),
    };
    
    let response = HttpTrace {
        method: "GET".to_string(),
        url: "https://example.com/api/test".to_string(),
        headers: HashMap::new(),
        body: Some("response".to_string()),
        status_code: Some(200),
        timestamp: Utc::now(),
    };
    
    let evidence = Evidence::new(request, response, "HypothesisAgent".to_string());
    
    assert_eq!(evidence.agent_name, "HypothesisAgent");
}

#[tokio::test]
async fn test_multiple_evidence_assessment() {
    let agent = EvidenceAgent::new().unwrap();
    
    let evidences: Vec<Evidence> = (0..5)
        .map(|i| {
            let request = HttpTrace {
                method: "GET".to_string(),
                url: format!("https://example.com/api/test{}", i),
                headers: HashMap::new(),
                body: None,
                status_code: None,
                timestamp: Utc::now(),
            };
            
            let response = HttpTrace {
                method: "GET".to_string(),
                url: format!("https://example.com/api/test{}", i),
                headers: HashMap::new(),
                body: Some(format!("response{}", i)),
                status_code: Some(200),
                timestamp: Utc::now(),
            };
            
            Evidence::new(request, response, "TestAgent".to_string())
        })
        .collect();
    
    for evidence in evidences {
        let quality = agent.assess_quality(&evidence).await;
        assert!(quality.is_ok());
    }
}

#[test]
fn test_http_trace_with_headers() {
    let mut headers = HashMap::new();
    headers.insert("Authorization".to_string(), "Bearer token123".to_string());
    headers.insert("Content-Type".to_string(), "application/json".to_string());
    
    let trace = HttpTrace {
        method: "POST".to_string(),
        url: "https://api.example.com/data".to_string(),
        headers,
        body: Some("{\"key\":\"value\"}".to_string()),
        status_code: Some(201),
        timestamp: Utc::now(),
    };
    
    assert_eq!(trace.method, "POST");
    assert!(trace.headers.contains_key("Authorization"));
    assert_eq!(trace.status_code, Some(201));
}

#[test]
fn test_http_trace_without_body() {
    let trace = HttpTrace {
        method: "DELETE".to_string(),
        url: "https://api.example.com/resource/123".to_string(),
        headers: HashMap::new(),
        body: None,
        status_code: Some(204),
        timestamp: Utc::now(),
    };
    
    assert!(trace.body.is_none());
    assert_eq!(trace.status_code, Some(204));
}
