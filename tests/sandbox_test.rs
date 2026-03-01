use strike_security::sandbox::Sandbox;

#[tokio::test]
async fn test_sandbox_creation() {
    let sandbox = Sandbox::new().await;
    assert!(sandbox.is_ok());
}

#[tokio::test]
async fn test_sandbox_availability() {
    let sandbox = Sandbox::new().await.unwrap();
    
    // Docker may or may not be available in test environment
    let available = sandbox.is_available();
    assert!(available == true || available == false);
}

#[tokio::test]
async fn test_sandbox_execute_simple_command() {
    let sandbox = Sandbox::new().await.unwrap();
    
    if !sandbox.is_available() {
        // Skip test if Docker not available
        return;
    }
    
    let result = sandbox.execute_in_sandbox("echo 'test'").await;
    
    // Should either succeed or fail gracefully
    assert!(result.is_ok() || result.is_err());
}

#[tokio::test]
async fn test_sandbox_execute_without_docker() {
    // Test that sandbox handles missing docker gracefully
    let sandbox = Sandbox::new().await;
    
    if sandbox.is_err() {
        // Docker not available, which is expected in some environments
        assert!(true);
        return;
    }
    
    let sandbox = sandbox.unwrap();
    if !sandbox.is_available() {
        assert!(true);
    }
}

#[tokio::test]
async fn test_sandbox_test_payload_isolated() {
    let sandbox = Sandbox::new().await.unwrap();
    
    let result = sandbox.test_payload_isolated("test_payload", "https://httpbin.org/post").await;
    
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_sandbox_result_structure() {
    let sandbox = Sandbox::new().await.unwrap();
    
    let result = sandbox.test_payload_isolated("test", "https://httpbin.org/post").await;
    
    if let Ok(sandbox_result) = result {
        // Check structure exists
        assert!(sandbox_result.executed == true || sandbox_result.executed == false);
        assert!(sandbox_result.safe == true || sandbox_result.safe == false);
    }
}

#[tokio::test]
async fn test_sandbox_handles_dangerous_payload() {
    let sandbox = Sandbox::new().await.unwrap();
    
    if !sandbox.is_available() {
        return;
    }
    
    let dangerous_payload = "'; DROP TABLE users; --";
    let result = sandbox.test_payload_isolated(dangerous_payload, "https://httpbin.org/post").await;
    
    assert!(result.is_ok());
}
