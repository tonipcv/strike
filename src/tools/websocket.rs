use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketConfig {
    pub timeout_seconds: u64,
    pub max_message_size: usize,
    pub auto_reconnect: bool,
    pub ping_interval_seconds: u64,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: 30,
            max_message_size: 1024 * 1024, // 1MB
            auto_reconnect: true,
            ping_interval_seconds: 30,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WebSocketMessage {
    Text(String),
    Binary(Vec<u8>),
    Ping,
    Pong,
    Close { code: u16, reason: String },
}

impl WebSocketMessage {
    pub const NORMAL_CLOSURE: u16 = 1000;
    pub const GOING_AWAY: u16 = 1001;
    pub const PROTOCOL_ERROR: u16 = 1002;
    pub const UNSUPPORTED_DATA: u16 = 1003;

    pub fn text<S: Into<String>>(s: S) -> Self { Self::Text(s.into()) }
    pub fn binary(b: Vec<u8>) -> Self { Self::Binary(b) }
    pub fn ping() -> Self { Self::Ping }
    pub fn pong() -> Self { Self::Pong }
    pub fn close<S: Into<String>>(code: u16, reason: S) -> Self { Self::Close { code, reason: reason.into() } }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum MessageType {
    Text,
    Binary,
    Ping,
    Pong,
    Close,
}

pub struct WebSocketTester {
    config: WebSocketConfig,
}

impl WebSocketTester {
    pub fn new(config: WebSocketConfig) -> Result<Self> {
        Ok(Self { config })
    }
    
    pub fn validate_url(&self, url: &str) -> Result<()> {
        if !url.starts_with("ws://") && !url.starts_with("wss://") {
            anyhow::bail!("Invalid WebSocket URL scheme");
        }
        Ok(())
    }
    
    pub async fn connect(&self, url: &str) -> Result<WebSocketConnection> {
        self.validate_url(url)?;
        
        Ok(WebSocketConnection {
            url: url.to_string(),
            config: self.config.clone(),
            connected: false,
        })
    }
    
    pub async fn send_message(&self, _connection: &mut WebSocketConnection, _message: WebSocketMessage) -> Result<()> {
        Ok(())
    }
    
    pub async fn receive_message(&self, _connection: &mut WebSocketConnection) -> Result<Option<WebSocketMessage>> {
        Ok(None)
    }
    
    pub async fn close(&self, _connection: &mut WebSocketConnection) -> Result<()> {
        Ok(())
    }
}

pub struct WebSocketConnection {
    pub url: String,
    pub config: WebSocketConfig,
    pub connected: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_websocket_config_creation() {
        let config = WebSocketConfig::default();
        assert_eq!(config.timeout_seconds, 30);
    }
    
    #[test]
    fn test_websocket_message_types() {
        let text = WebSocketMessage::text("test");
        let binary = WebSocketMessage::binary(vec![1, 2, 3]);
        let ping = WebSocketMessage::ping();
        let pong = WebSocketMessage::pong();
        let close = WebSocketMessage::close(1000, "bye");
        
        match text {
            WebSocketMessage::Text(_) => assert!(true),
            _ => panic!("Expected text"),
        }
        
        match binary {
            WebSocketMessage::Binary(_) => assert!(true),
            _ => panic!("Expected binary"),
        }
    }
}
