use strike_security::tools::websocket::{WebSocketTester, WebSocketConfig, WebSocketMessage, MessageType};
use std::time::Duration;

#[tokio::test]
async fn test_websocket_config_default() {
    let config = WebSocketConfig::default();
    
    assert_eq!(config.timeout, Duration::from_secs(30));
    assert_eq!(config.max_message_size, 1024 * 1024);
}

#[tokio::test]
async fn test_websocket_tester_creation() {
    let config = WebSocketConfig {
        url: "ws://localhost:8080/ws".to_string(),
        timeout: Duration::from_secs(10),
        max_message_size: 1024,
    };
    
    let tester = WebSocketTester::new(config);
    assert!(true); // Tester created successfully
}

#[tokio::test]
async fn test_websocket_connect_valid_url() {
    let config = WebSocketConfig {
        url: "ws://echo.websocket.org".to_string(),
        ..Default::default()
    };
    
    let tester = WebSocketTester::new(config);
    let result = tester.connect().await;
    
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_websocket_connect_invalid_url() {
    let config = WebSocketConfig {
        url: "http://invalid.com".to_string(),
        ..Default::default()
    };
    
    let tester = WebSocketTester::new(config);
    let result = tester.connect().await;
    
    assert!(result.is_err());
}

#[tokio::test]
async fn test_websocket_send_message() {
    let config = WebSocketConfig {
        url: "ws://localhost:8080/ws".to_string(),
        ..Default::default()
    };
    
    let tester = WebSocketTester::new(config);
    let message = WebSocketMessage {
        payload: "test message".to_string(),
        message_type: MessageType::Text,
    };
    
    let result = tester.send_message(&message).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_websocket_send_oversized_message() {
    let config = WebSocketConfig {
        url: "ws://localhost:8080/ws".to_string(),
        max_message_size: 10,
        ..Default::default()
    };
    
    let tester = WebSocketTester::new(config);
    let message = WebSocketMessage {
        payload: "this message is too long".to_string(),
        message_type: MessageType::Text,
    };
    
    let result = tester.send_message(&message).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_websocket_receive_message() {
    let config = WebSocketConfig {
        url: "ws://localhost:8080/ws".to_string(),
        ..Default::default()
    };
    
    let tester = WebSocketTester::new(config);
    let result = tester.receive_message().await;
    
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_websocket_send_and_receive() {
    let config = WebSocketConfig {
        url: "ws://localhost:8080/ws".to_string(),
        ..Default::default()
    };
    
    let tester = WebSocketTester::new(config);
    let message = WebSocketMessage {
        payload: "echo test".to_string(),
        message_type: MessageType::Text,
    };
    
    let result = tester.send_and_receive(&message).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_websocket_injection_test() {
    let config = WebSocketConfig {
        url: "ws://localhost:8080/ws".to_string(),
        ..Default::default()
    };
    
    let tester = WebSocketTester::new(config);
    let result = tester.test_injection("<script>alert('xss')</script>").await;
    
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_websocket_close() {
    let config = WebSocketConfig {
        url: "ws://localhost:8080/ws".to_string(),
        ..Default::default()
    };
    
    let tester = WebSocketTester::new(config);
    let result = tester.close().await;
    
    assert!(result.is_ok());
}

#[test]
fn test_message_type_equality() {
    assert_eq!(MessageType::Text, MessageType::Text);
    assert_eq!(MessageType::Binary, MessageType::Binary);
    assert_ne!(MessageType::Text, MessageType::Binary);
}

#[test]
fn test_websocket_message_creation() {
    let msg = WebSocketMessage {
        payload: "test".to_string(),
        message_type: MessageType::Text,
    };
    
    assert_eq!(msg.payload, "test");
    assert_eq!(msg.message_type, MessageType::Text);
}

#[tokio::test]
async fn test_websocket_multiple_messages() {
    let config = WebSocketConfig {
        url: "ws://localhost:8080/ws".to_string(),
        ..Default::default()
    };
    
    let tester = WebSocketTester::new(config);
    
    for i in 0..5 {
        let message = WebSocketMessage {
            payload: format!("message {}", i),
            message_type: MessageType::Text,
        };
        
        let result = tester.send_message(&message).await;
        assert!(result.is_ok());
    }
}
