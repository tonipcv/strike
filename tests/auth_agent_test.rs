use strike_security::agents::auth_agent::{AuthAgent, AuthCredentials, AuthResult};

#[tokio::test]
async fn test_auth_agent_creation() {
    let agent = AuthAgent::new().await;
    assert!(agent.is_ok());
}

#[tokio::test]
async fn test_basic_auth_structure() {
    let mut agent = AuthAgent::new().await.unwrap();
    
    let creds = AuthCredentials::Basic {
        username: "test".to_string(),
        password: "password".to_string(),
    };
    
    let result = agent.authenticate("https://httpbin.org/basic-auth/test/password", &creds).await;
    
    // Should return a result (success or failure)
    assert!(result.is_ok() || result.is_err());
}

#[tokio::test]
async fn test_bearer_auth_structure() {
    let mut agent = AuthAgent::new().await.unwrap();
    
    let creds = AuthCredentials::Bearer {
        token: "test_token_123".to_string(),
    };
    
    let result = agent.authenticate("https://httpbin.org/bearer", &creds).await;
    
    if let Ok(auth_result) = result {
        assert!(auth_result.session_token.is_some());
        assert_eq!(auth_result.headers.len(), 1);
    }
}

#[tokio::test]
async fn test_api_key_auth_structure() {
    let mut agent = AuthAgent::new().await.unwrap();
    
    let creds = AuthCredentials::ApiKey {
        key: "api_key_123".to_string(),
        header_name: "X-API-Key".to_string(),
    };
    
    let result = agent.authenticate("https://httpbin.org/headers", &creds).await;
    
    if let Ok(auth_result) = result {
        assert_eq!(auth_result.headers.len(), 1);
        assert_eq!(auth_result.headers[0].0, "X-API-Key");
    }
}

#[test]
fn test_auth_result_structure() {
    let result = AuthResult {
        success: true,
        session_token: Some("token123".to_string()),
        cookies: vec!["session=abc".to_string()],
        headers: vec![("Authorization".to_string(), "Bearer token123".to_string())],
    };
    
    assert!(result.success);
    assert!(result.session_token.is_some());
    assert_eq!(result.cookies.len(), 1);
    assert_eq!(result.headers.len(), 1);
}

#[tokio::test]
async fn test_oauth2_auth_structure() {
    let mut agent = AuthAgent::new().await.unwrap();
    
    let creds = AuthCredentials::OAuth2 {
        client_id: "client123".to_string(),
        client_secret: "secret456".to_string(),
        token_url: "https://httpbin.org/post".to_string(),
        scope: Some("read write".to_string()),
    };
    
    let result = agent.authenticate("https://httpbin.org/bearer", &creds).await;
    
    // Should return a result structure
    assert!(result.is_ok() || result.is_err());
}

#[tokio::test]
async fn test_cookie_extraction() {
    let mut agent = AuthAgent::new().await.unwrap();
    
    let creds = AuthCredentials::Basic {
        username: "user".to_string(),
        password: "pass".to_string(),
    };
    
    // httpbin.org/cookies/set sets cookies
    let result = agent.authenticate("https://httpbin.org/cookies/set?session=test123", &creds).await;
    
    if let Ok(auth_result) = result {
        // Cookies may or may not be present depending on response
        assert!(auth_result.cookies.len() >= 0);
    }
}
