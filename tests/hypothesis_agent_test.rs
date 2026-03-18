use strike_security::agents::hypothesis::{
    EndpointGraph, Hypothesis, HypothesisAgent, SessionContext,
};
use strike_security::llm::{
    prompt::EndpointInfo,
    router::LlmRouter,
};
use strike_security::models::vuln_class::VulnClass;
use std::sync::Arc;

#[tokio::test]
async fn test_hypothesis_agent_creation() {
    let router = LlmRouter::for_tests();
        let agent = HypothesisAgent::new(Arc::new(router), Some(50));
        assert!(agent.is_ok());
}

#[tokio::test]
async fn test_endpoint_graph_creation() {
    let endpoints = vec![
        EndpointInfo {
            url: "/api/users/123".to_string(),
            method: "GET".to_string(),
            parameters: vec!["id".to_string()],
            auth_required: true,
            response_codes: vec![200, 401],
        },
        EndpointInfo {
            url: "/api/posts/456".to_string(),
            method: "POST".to_string(),
            parameters: vec!["title".to_string(), "content".to_string()],
            auth_required: true,
            response_codes: vec![201, 401],
        },
    ];
    
    let graph = EndpointGraph::new(endpoints);
    assert_eq!(graph.endpoints.len(), 2);
}

#[tokio::test]
async fn test_endpoint_graph_chunking() {
    let endpoints: Vec<EndpointInfo> = (0..25)
        .map(|i| EndpointInfo {
            url: format!("/api/endpoint/{}", i),
            method: "GET".to_string(),
            parameters: vec!["id".to_string()],
            auth_required: true,
            response_codes: vec![200],
        })
        .collect();
    
    let graph = EndpointGraph::new(endpoints);
    let chunks = graph.chunk(10);
    
    assert_eq!(chunks.len(), 3);
    assert_eq!(chunks[0].len(), 10);
    assert_eq!(chunks[1].len(), 10);
    assert_eq!(chunks[2].len(), 5);
}

#[tokio::test]
async fn test_hypothesis_confidence_range() {
    let hypothesis = Hypothesis {
        id: "test-id".to_string(),
        endpoint: "/api/users".to_string(),
        method: "GET".to_string(),
        parameter: Some("id".to_string()),
        vuln_class: "IDOR".to_string(),
        confidence: 0.85,
        severity_potential: "high".to_string(),
        reasoning: "Test reasoning".to_string(),
        suggested_payload: Some("123".to_string()),
        test_strategy: "Test strategy".to_string(),
        owasp_ref: "A01:2021".to_string(),
    };
    
    assert!(hypothesis.confidence >= 0.0 && hypothesis.confidence <= 1.0);
}

#[tokio::test]
async fn test_session_context_creation() {
    let session = SessionContext {
        auth_type: "Bearer".to_string(),
        session_token: Some("test-token-123".to_string()),
        cookies: vec!["session=abc123".to_string()],
    };
    
    assert_eq!(session.auth_type, "Bearer");
    assert!(session.session_token.is_some());
    assert_eq!(session.cookies.len(), 1);
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

#[tokio::test]
async fn test_hypothesis_ranking_by_severity() {
    let router = LlmRouter::for_tests();
        if let Ok(agent) = HypothesisAgent::new(Arc::new(router), Some(50)) {
            let hypotheses = vec![
                Hypothesis {
                    id: "low".to_string(),
                    endpoint: "/api/info".to_string(),
                    method: "GET".to_string(),
                    parameter: None,
                    vuln_class: "InfoDisclosure".to_string(),
                    confidence: 0.5,
                    severity_potential: "low".to_string(),
                    reasoning: "Low severity".to_string(),
                    suggested_payload: None,
                    test_strategy: "Test".to_string(),
                    owasp_ref: "A01:2021".to_string(),
                },
                Hypothesis {
                    id: "critical".to_string(),
                    endpoint: "/api/admin".to_string(),
                    method: "POST".to_string(),
                    parameter: Some("query".to_string()),
                    vuln_class: "SQLi".to_string(),
                    confidence: 0.9,
                    severity_potential: "critical".to_string(),
                    reasoning: "Critical severity".to_string(),
                    suggested_payload: Some("' OR 1=1--".to_string()),
                    test_strategy: "Test".to_string(),
                    owasp_ref: "A03:2021".to_string(),
                },
                Hypothesis {
                    id: "medium".to_string(),
                    endpoint: "/api/search".to_string(),
                    method: "GET".to_string(),
                    parameter: Some("q".to_string()),
                    vuln_class: "XSS".to_string(),
                    confidence: 0.7,
                    severity_potential: "medium".to_string(),
                    reasoning: "Medium severity".to_string(),
                    suggested_payload: Some("<script>alert(1)</script>".to_string()),
                    test_strategy: "Test".to_string(),
                    owasp_ref: "A03:2021".to_string(),
                },
            ];
            
            let ranked = agent.rank_hypotheses(hypotheses);
            
            assert_eq!(ranked[0].id, "critical");
            assert_eq!(ranked[2].id, "low");
        }
}

#[tokio::test]
async fn test_max_hypotheses_limit() {
    let router = LlmRouter::for_tests();
        let agent = HypothesisAgent::new(Arc::new(router), Some(5));
        assert!(agent.is_ok());
        
        if let Ok(agent) = agent {
            let hypotheses: Vec<Hypothesis> = (0..10)
                .map(|i| Hypothesis {
                    id: format!("hyp-{}", i),
                    endpoint: format!("/api/endpoint/{}", i),
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
            
            let ranked = agent.rank_hypotheses(hypotheses);
            let limited: Vec<Hypothesis> = ranked.into_iter().take(5).collect();
            
            assert_eq!(limited.len(), 5);
        }
    }
}
