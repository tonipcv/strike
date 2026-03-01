use strike_security::tools::traffic_replayer::{TrafficReplayer, HttpRequest, MutationStrategy};

#[tokio::test]
async fn test_traffic_replayer_creation() {
    let replayer = TrafficReplayer::new().await;
    assert!(replayer.is_ok());
}

#[tokio::test]
async fn test_replay_with_parameter_fuzzing() {
    let replayer = TrafficReplayer::new().await.unwrap();
    
    let request = HttpRequest {
        method: "POST".to_string(),
        url: "https://httpbin.org/post".to_string(),
        headers: vec![],
        body: Some("test=value".to_string()),
    };
    
    let results = replayer.replay_with_mutations(&request, MutationStrategy::ParameterFuzzing).await;
    
    assert!(results.is_ok());
    let results = results.unwrap();
    assert!(results.len() > 0);
}

#[tokio::test]
async fn test_replay_with_header_injection() {
    let replayer = TrafficReplayer::new().await.unwrap();
    
    let request = HttpRequest {
        method: "GET".to_string(),
        url: "https://httpbin.org/headers".to_string(),
        headers: vec![],
        body: None,
    };
    
    let results = replayer.replay_with_mutations(&request, MutationStrategy::HeaderInjection).await;
    
    assert!(results.is_ok());
    let results = results.unwrap();
    assert!(results.len() > 0);
}

#[tokio::test]
async fn test_replay_with_method_swapping() {
    let replayer = TrafficReplayer::new().await.unwrap();
    
    let request = HttpRequest {
        method: "GET".to_string(),
        url: "https://httpbin.org/get".to_string(),
        headers: vec![],
        body: None,
    };
    
    let results = replayer.replay_with_mutations(&request, MutationStrategy::MethodSwapping).await;
    
    assert!(results.is_ok());
    let results = results.unwrap();
    assert!(results.len() > 0);
}

#[tokio::test]
async fn test_replay_with_auth_bypass() {
    let replayer = TrafficReplayer::new().await.unwrap();
    
    let request = HttpRequest {
        method: "GET".to_string(),
        url: "https://httpbin.org/headers".to_string(),
        headers: vec![],
        body: None,
    };
    
    let results = replayer.replay_with_mutations(&request, MutationStrategy::AuthBypass).await;
    
    assert!(results.is_ok());
    let results = results.unwrap();
    assert!(results.len() > 0);
}

#[test]
fn test_http_request_structure() {
    let request = HttpRequest {
        method: "POST".to_string(),
        url: "https://example.com/api".to_string(),
        headers: vec![("Content-Type".to_string(), "application/json".to_string())],
        body: Some("{\"test\": \"data\"}".to_string()),
    };
    
    assert_eq!(request.method, "POST");
    assert_eq!(request.url, "https://example.com/api");
    assert_eq!(request.headers.len(), 1);
    assert!(request.body.is_some());
}

#[tokio::test]
async fn test_mutation_strategy_parameter_fuzzing() {
    let replayer = TrafficReplayer::new().await.unwrap();
    
    let request = HttpRequest {
        method: "POST".to_string(),
        url: "https://httpbin.org/post".to_string(),
        headers: vec![],
        body: Some("param=value".to_string()),
    };
    
    let results = replayer.replay_with_mutations(&request, MutationStrategy::ParameterFuzzing).await;
    
    if let Ok(results) = results {
        // Should have multiple mutations
        assert!(results.len() >= 3);
        
        // Check that mutations were applied
        for result in results {
            assert!(result.mutation_applied.is_some());
        }
    }
}

#[tokio::test]
async fn test_replay_result_contains_response() {
    let replayer = TrafficReplayer::new().await.unwrap();
    
    let request = HttpRequest {
        method: "GET".to_string(),
        url: "https://httpbin.org/get".to_string(),
        headers: vec![],
        body: None,
    };
    
    let results = replayer.replay_with_mutations(&request, MutationStrategy::MethodSwapping).await;
    
    if let Ok(results) = results {
        for result in results {
            assert!(result.response_status > 0);
            assert!(!result.response_body.is_empty() || result.response_status >= 400);
        }
    }
}
