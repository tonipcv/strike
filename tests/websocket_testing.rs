use strike_security::tools::websocket::{WebSocketTester, WebSocketMessage, WebSocketConfig};

#[tokio::test]
async fn test_websocket_tester_creation() {
    let config = WebSocketConfig::default();
    let tester = WebSocketTester::new(config);
    assert!(tester.is_ok());
}

#[tokio::test]
async fn test_websocket_config_defaults() {
    let config = WebSocketConfig::default();
    
    assert_eq!(config.timeout_seconds, 30);
    assert_eq!(config.max_message_size, 1_048_576);
    assert!(config.auto_reconnect);
    assert_eq!(config.ping_interval_seconds, 30);
}

#[tokio::test]
async fn test_websocket_config_custom() {
    let config = WebSocketConfig {
        timeout_seconds: 60,
        max_message_size: 2_097_152,
        auto_reconnect: false,
        ping_interval_seconds: 60,
    };
    
    assert_eq!(config.timeout_seconds, 60);
    assert_eq!(config.max_message_size, 2_097_152);
    assert!(!config.auto_reconnect);
}

#[test]
fn test_websocket_message_text() {
    let msg = WebSocketMessage::text("Hello WebSocket");
    
    match msg {
        WebSocketMessage::Text(content) => assert_eq!(content, "Hello WebSocket"),
        _ => panic!("Expected text message"),
    }
}

#[test]
fn test_websocket_message_binary() {
    let data = vec![1, 2, 3, 4, 5];
    let msg = WebSocketMessage::binary(data.clone());
    
    match msg {
        WebSocketMessage::Binary(content) => assert_eq!(content, data),
        _ => panic!("Expected binary message"),
    }
}

#[test]
fn test_websocket_message_ping() {
    let msg = WebSocketMessage::ping();
    
    match msg {
        WebSocketMessage::Ping => assert!(true),
        _ => panic!("Expected ping message"),
    }
}

#[test]
fn test_websocket_message_pong() {
    let msg = WebSocketMessage::pong();
    
    match msg {
        WebSocketMessage::Pong => assert!(true),
        _ => panic!("Expected pong message"),
    }
}

#[test]
fn test_websocket_message_close() {
    let msg = WebSocketMessage::close(1000, "Normal closure");
    
    match msg {
        WebSocketMessage::Close { code, reason } => {
            assert_eq!(code, 1000);
            assert_eq!(reason, "Normal closure");
        },
        _ => panic!("Expected close message"),
    }
}

#[tokio::test]
async fn test_websocket_url_validation() {
    let config = WebSocketConfig::default();
    let tester = WebSocketTester::new(config).unwrap();
    
    assert!(tester.validate_url("ws://localhost:8080/ws").is_ok());
    assert!(tester.validate_url("wss://example.com/socket").is_ok());
    assert!(tester.validate_url("http://example.com").is_err());
    assert!(tester.validate_url("invalid").is_err());
}

#[test]
fn test_websocket_close_codes() {
    assert_eq!(WebSocketMessage::NORMAL_CLOSURE, 1000);
    assert_eq!(WebSocketMessage::GOING_AWAY, 1001);
    assert_eq!(WebSocketMessage::PROTOCOL_ERROR, 1002);
    assert_eq!(WebSocketMessage::UNSUPPORTED_DATA, 1003);
}

#[tokio::test]
async fn test_websocket_message_serialization() {
    let msg = WebSocketMessage::text("{\"type\":\"test\",\"data\":\"value\"}");
    
    let json_str = match msg {
        WebSocketMessage::Text(content) => content,
        _ => panic!("Expected text message"),
    };
    
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    assert_eq!(parsed["type"], "test");
    assert_eq!(parsed["data"], "value");
}

#[tokio::test]
async fn test_websocket_large_message() {
    let large_data = vec![0u8; 1_000_000];
    let msg = WebSocketMessage::binary(large_data.clone());
    
    match msg {
        WebSocketMessage::Binary(content) => assert_eq!(content.len(), 1_000_000),
        _ => panic!("Expected binary message"),
    }
}

#[tokio::test]
async fn test_websocket_empty_message() {
    let msg = WebSocketMessage::text("");
    
    match msg {
        WebSocketMessage::Text(content) => assert_eq!(content, ""),
        _ => panic!("Expected text message"),
    }
}

#[tokio::test]
async fn test_websocket_unicode_message() {
    let msg = WebSocketMessage::text("Hello 世界 🌍");
    
    match msg {
        WebSocketMessage::Text(content) => assert!(content.contains("世界")),
        _ => panic!("Expected text message"),
    }
}

#[tokio::test]
async fn test_websocket_config_validation() {
    let config = WebSocketConfig {
        timeout_seconds: 0,
        max_message_size: 0,
        auto_reconnect: true,
        ping_interval_seconds: 0,
    };
    
    assert_eq!(config.timeout_seconds, 0);
    assert_eq!(config.max_message_size, 0);
}

#[tokio::test]
async fn test_websocket_multiple_messages() {
    let messages = vec![
        WebSocketMessage::text("Message 1"),
        WebSocketMessage::text("Message 2"),
        WebSocketMessage::text("Message 3"),
    ];
    
    assert_eq!(messages.len(), 3);
}
