use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use futures_util::{StreamExt, SinkExt};

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
    
    fn to_tungstenite(&self) -> Message {
        match self {
            Self::Text(s) => Message::Text(s.clone()),
            Self::Binary(b) => Message::Binary(b.clone()),
            Self::Ping => Message::Ping(vec![]),
            Self::Pong => Message::Pong(vec![]),
            Self::Close { code, reason } => Message::Close(Some(tokio_tungstenite::tungstenite::protocol::CloseFrame {
                code: (*code).into(),
                reason: std::borrow::Cow::Owned(reason.clone()),
            })),
        }
    }
    
    fn from_tungstenite(msg: Message) -> Option<Self> {
        match msg {
            Message::Text(s) => Some(Self::Text(s)),
            Message::Binary(b) => Some(Self::Binary(b)),
            Message::Ping(_) => Some(Self::Ping),
            Message::Pong(_) => Some(Self::Pong),
            Message::Close(frame) => {
                if let Some(f) = frame {
                    Some(Self::Close {
                        code: f.code.into(),
                        reason: f.reason.to_string(),
                    })
                } else {
                    Some(Self::Close {
                        code: Self::NORMAL_CLOSURE,
                        reason: String::new(),
                    })
                }
            }
            Message::Frame(_) => None,
        }
    }
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
        
        let (ws_stream, _) = tokio::time::timeout(
            Duration::from_secs(self.config.timeout_seconds),
            connect_async(url)
        ).await??;
        
        let (write, read) = ws_stream.split();
        
        Ok(WebSocketConnection {
            url: url.to_string(),
            config: self.config.clone(),
            connected: true,
            write: Some(write),
            read: Some(read),
        })
    }
    
    pub async fn send_message(&self, connection: &mut WebSocketConnection, message: WebSocketMessage) -> Result<()> {
        if !connection.connected {
            return Err(anyhow!("WebSocket not connected"));
        }
        
        if let Some(write) = &mut connection.write {
            let msg = message.to_tungstenite();
            write.send(msg).await?;
        } else {
            return Err(anyhow!("WebSocket write stream not available"));
        }
        
        Ok(())
    }
    
    pub async fn receive_message(&self, connection: &mut WebSocketConnection) -> Result<Option<WebSocketMessage>> {
        if !connection.connected {
            return Err(anyhow!("WebSocket not connected"));
        }
        
        if let Some(read) = &mut connection.read {
            let timeout_duration = Duration::from_secs(self.config.timeout_seconds);
            
            match tokio::time::timeout(timeout_duration, read.next()).await {
                Ok(Some(Ok(msg))) => Ok(WebSocketMessage::from_tungstenite(msg)),
                Ok(Some(Err(e))) => Err(anyhow!("WebSocket error: {}", e)),
                Ok(None) => {
                    connection.connected = false;
                    Ok(None)
                }
                Err(_) => Err(anyhow!("Receive timeout")),
            }
        } else {
            Err(anyhow!("WebSocket read stream not available"))
        }
    }
    
    pub async fn close(&self, connection: &mut WebSocketConnection) -> Result<()> {
        if connection.connected {
            self.send_message(connection, WebSocketMessage::close(
                WebSocketMessage::NORMAL_CLOSURE,
                "Closing connection"
            )).await?;
            connection.connected = false;
        }
        Ok(())
    }
}

pub struct WebSocketConnection {
    pub url: String,
    pub config: WebSocketConfig,
    pub connected: bool,
    write: Option<futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
        Message
    >>,
    read: Option<futures_util::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>
    >>,
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
