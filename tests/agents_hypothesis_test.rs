use strike_security::agents::hypothesis::{Hypothesis, HypothesisAgent};
use strike_security::llm::router::LlmRouter;
use std::sync::Arc;

#[tokio::test]
async fn test_hypothesis_creation() {
    let hypothesis = Hypothesis {
        id: "hyp-1".to_string(),
        endpoint: "/api/users".to_string(),
        method: "GET".to_string(),
        parameter: Some("id".to_string()),
        vuln_class: "IDOR".to_string(),
        confidence: 0.9,
        severity_potential: "high".to_string(),
        reasoning: "User ID parameter without authorization check".to_string(),
        suggested_payload: Some("?id=2".to_string()),
        test_strategy: "Try accessing other user IDs".to_string(),
        owasp_ref: "A01:2021".to_string(),
    };
    
    assert_eq!(hypothesis.endpoint, "/api/users");
    assert_eq!(hypothesis.vuln_class, "IDOR");
    assert!(hypothesis.confidence > 0.8);
}

#[tokio::test]
async fn test_hypothesis_agent_creation() {
    let router = LlmRouter::for_tests();
    let agent = HypothesisAgent::new(Arc::new(router), Some(50));
    assert!(agent.is_ok());
}

#[tokio::test]
async fn test_hypothesis_deduplication() {
    let router = LlmRouter::for_tests();
    if let Ok(agent) = HypothesisAgent::new(Arc::new(router), Some(50)) {
            let hypotheses = vec![
                Hypothesis {
                    id: "1".to_string(),
                    endpoint: "/api/users".to_string(),
                    method: "GET".to_string(),
                    parameter: Some("id".to_string()),
                    vuln_class: "IDOR".to_string(),
                    confidence: 0.9,
                    severity_potential: "high".to_string(),
                    reasoning: "First".to_string(),
                    suggested_payload: None,
                    test_strategy: "Test".to_string(),
                    owasp_ref: "A01:2021".to_string(),
                },
                Hypothesis {
                    id: "2".to_string(),
                    endpoint: "/api/users".to_string(),
                    method: "GET".to_string(),
                    parameter: Some("id".to_string()),
                    vuln_class: "IDOR".to_string(),
                    confidence: 0.8,
                    severity_potential: "high".to_string(),
                    reasoning: "Duplicate".to_string(),
                    suggested_payload: None,
                    test_strategy: "Test".to_string(),
                    owasp_ref: "A01:2021".to_string(),
                },
                Hypothesis {
                    id: "3".to_string(),
                    endpoint: "/api/posts".to_string(),
                    method: "POST".to_string(),
                    parameter: None,
                    vuln_class: "SQLi".to_string(),
                    confidence: 0.7,
                    severity_potential: "critical".to_string(),
                    reasoning: "Different".to_string(),
                    suggested_payload: Some("' OR 1=1--".to_string()),
                    test_strategy: "Test".to_string(),
                    owasp_ref: "A03:2021".to_string(),
                },
            ];
            
            let deduplicated = agent.deduplicate_hypotheses(hypotheses);
            assert_eq!(deduplicated.len(), 2);
    }
}

#[tokio::test]
async fn test_hypothesis_ranking() {
    let router = LlmRouter::for_tests();
    if let Ok(agent) = HypothesisAgent::new(Arc::new(router), Some(50)) {
            let mut hypotheses = vec![
                Hypothesis {
                    id: "1".to_string(),
                    endpoint: "/api/users".to_string(),
                    method: "GET".to_string(),
                    parameter: Some("id".to_string()),
                    vuln_class: "IDOR".to_string(),
                    confidence: 0.6,
                    severity_potential: "medium".to_string(),
                    reasoning: "Low confidence".to_string(),
                    suggested_payload: None,
                    test_strategy: "Test".to_string(),
                    owasp_ref: "A01:2021".to_string(),
                },
                Hypothesis {
                    id: "2".to_string(),
                    endpoint: "/api/admin".to_string(),
                    method: "POST".to_string(),
                    parameter: None,
                    vuln_class: "SQLi".to_string(),
                    confidence: 0.95,
                    severity_potential: "critical".to_string(),
                    reasoning: "High confidence".to_string(),
                    suggested_payload: Some("' OR 1=1--".to_string()),
                    test_strategy: "Test".to_string(),
                    owasp_ref: "A03:2021".to_string(),
                },
            ];
            
            let ranked = agent.rank_hypotheses(hypotheses.clone());
            assert_eq!(ranked[0].id, "2");
            assert_eq!(ranked[1].id, "1");
    }
}

#[test]
fn test_hypothesis_confidence_levels() {
    let high_confidence = Hypothesis {
        id: "h1".to_string(),
        endpoint: "/api/users".to_string(),
        method: "GET".to_string(),
        parameter: Some("id".to_string()),
        vuln_class: "IDOR".to_string(),
        confidence: 0.95,
        severity_potential: "high".to_string(),
        reasoning: "Clear pattern".to_string(),
        suggested_payload: None,
        test_strategy: "Test".to_string(),
        owasp_ref: "A01:2021".to_string(),
    };
    
    let low_confidence = Hypothesis {
        id: "h2".to_string(),
        endpoint: "/api/posts".to_string(),
        method: "GET".to_string(),
        parameter: None,
        vuln_class: "XSS".to_string(),
        confidence: 0.3,
        severity_potential: "low".to_string(),
        reasoning: "Unclear pattern".to_string(),
        suggested_payload: None,
        test_strategy: "Test".to_string(),
        owasp_ref: "A03:2021".to_string(),
    };
    
    assert!(high_confidence.confidence > 0.9);
    assert!(low_confidence.confidence < 0.5);
}

#[test]
fn test_hypothesis_severity_mapping() {
    let critical = Hypothesis {
        id: "h1".to_string(),
        endpoint: "/api/admin".to_string(),
        method: "POST".to_string(),
        parameter: None,
        vuln_class: "SQLi".to_string(),
        confidence: 0.9,
        severity_potential: "critical".to_string(),
        reasoning: "SQL injection in admin panel".to_string(),
        suggested_payload: Some("' OR 1=1--".to_string()),
        test_strategy: "Test".to_string(),
        owasp_ref: "A03:2021".to_string(),
    };
    
    assert_eq!(critical.severity_potential, "critical");
    assert_eq!(critical.vuln_class, "SQLi");
}

#[test]
fn test_hypothesis_owasp_references() {
    let idor = Hypothesis {
        id: "h1".to_string(),
        endpoint: "/api/users".to_string(),
        method: "GET".to_string(),
        parameter: Some("id".to_string()),
        vuln_class: "IDOR".to_string(),
        confidence: 0.9,
        severity_potential: "high".to_string(),
        reasoning: "Test".to_string(),
        suggested_payload: None,
        test_strategy: "Test".to_string(),
        owasp_ref: "A01:2021".to_string(),
    };
    
    let sqli = Hypothesis {
        id: "h2".to_string(),
        endpoint: "/api/search".to_string(),
        method: "GET".to_string(),
        parameter: Some("q".to_string()),
        vuln_class: "SQLi".to_string(),
        confidence: 0.85,
        severity_potential: "critical".to_string(),
        reasoning: "Test".to_string(),
        suggested_payload: Some("' OR 1=1--".to_string()),
        test_strategy: "Test".to_string(),
        owasp_ref: "A03:2021".to_string(),
    };
    
    assert_eq!(idor.owasp_ref, "A01:2021");
    assert_eq!(sqli.owasp_ref, "A03:2021");
}

#[tokio::test]
async fn test_hypothesis_chunking() {
    let router = LlmRouter::for_tests();
    if let Ok(agent) = HypothesisAgent::new(Arc::new(router), Some(3)) {
            let hypotheses: Vec<Hypothesis> = (0..10)
                .map(|i| Hypothesis {
                    id: format!("h{}", i),
                    endpoint: format!("/api/endpoint{}", i),
                    method: "GET".to_string(),
                    parameter: Some("id".to_string()),
                    vuln_class: "IDOR".to_string(),
                    confidence: 0.8,
                    severity_potential: "medium".to_string(),
                    reasoning: "Test".to_string(),
                    suggested_payload: None,
                    test_strategy: "Test".to_string(),
                    owasp_ref: "A01:2021".to_string(),
                })
                .collect();
            
            let chunks = agent.chunk_hypotheses(hypotheses);
            // With 10 hypotheses and chunk size 3, we should get at least 1 chunk
            assert!(chunks.len() >= 1);
            // Verify total hypotheses are preserved
            let total: usize = chunks.iter().map(|c| c.len()).sum();
            assert_eq!(total, 10);
    }
}
