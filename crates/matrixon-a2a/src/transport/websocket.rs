//! WebSocket Transport Implementation
//! 
//! This module implements WebSocket transport for A2A protocol.
//! 
//! Author: arkSong <arksong2018@gmail.com>
//! Version: 0.1.0
//! License: MIT

use crate::error::Error;
use crate::message::Message;
use tracing::{info, instrument};
use std::time::Instant;
use async_trait::async_trait;
use tokio::sync::RwLock;
use tokio_tungstenite::{
    connect_async,
    tungstenite::protocol::Message as WsMessage,
    MaybeTlsStream,
    WebSocketStream
};
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use url::Url;
use std::sync::Arc;
use std::time::Duration;

/// WebSocket transport configuration
#[derive(Debug, Clone)]
pub struct WebSocketConfig {
    pub url: String,
    pub reconnect_interval: Duration,
    pub max_reconnect_attempts: u32,
    pub ping_interval: Duration,
    pub pong_timeout: Duration,
}

/// WebSocket transport implementation
#[derive(Debug)]
pub struct WebSocketTransport {
    /// WebSocket connection
    connection: Arc<RwLock<Option<WebSocketStream<MaybeTlsStream<TcpStream>>>>>,
    /// Remote URL
    url: String,
}

impl WebSocketTransport {
    /// Create a new WebSocket transport from URL string
    #[instrument(level = "debug")]
    pub fn new(url: String) -> Self {
        Self {
            connection: Arc::new(RwLock::new(None)),
            url,
        }
    }

    /// Create a new WebSocket transport from TransportConfig
    #[instrument(level = "debug")]
    pub fn from_config(config: super::TransportConfig) -> Self {
        let scheme = match config.tls {
            Some(_) => "wss",
            None => "ws"
        };
        let url = format!("{}://{}:{}", scheme, config.host, config.port);
        Self::new(url)
    }

    /// Connect to remote WebSocket server
    #[instrument(level = "debug", skip(self))]
    pub async fn connect(&self) -> Result<(), Error> {
        let start = Instant::now();
        info!("🔧 Connecting to WebSocket server");

        let url = Url::parse(&self.url).map_err(|e| {
            Error::Connection(format!("Invalid WebSocket URL: {}", e))
        })?;

        let (ws_stream, _) = connect_async(url).await.map_err(|e| {
            Error::Connection(format!("WebSocket connection failed: {}", e))
        })?;

        let mut connection = self.connection.write().await;
        *connection = Some(ws_stream);

        info!("✅ WebSocket connected in {:?}", start.elapsed());
        Ok(())
    }

    /// Disconnect from remote WebSocket server
    #[instrument(level = "debug", skip(self))]
    pub async fn disconnect(&self) -> Result<(), Error> {
        let start = Instant::now();
        info!("🔧 Disconnecting from WebSocket server");

        let mut connection = self.connection.write().await;
        if let Some(mut stream) = connection.take() {
            stream.close(None).await.map_err(|e| {
                Error::Connection(format!("WebSocket disconnection failed: {}", e))
            })?;
        }

        info!("✅ WebSocket disconnected in {:?}", start.elapsed());
        Ok(())
    }

    /// Send message over WebSocket
    #[instrument(level = "debug", skip(self, message))]
    pub async fn send(&self, message: Message) -> Result<(), Error> {
        let start = Instant::now();
        info!("🔧 Sending WebSocket message");

        let mut connection = self.connection.write().await;
        let connection = connection.as_mut().ok_or_else(|| {
            Error::Connection("WebSocket connection not established".to_string())
        })?;

        let ws_message = WsMessage::Text(serde_json::to_string(&message).map_err(|e| {
            Error::Serialization(e)
        })?.to_string());

        connection
            .send(ws_message)
            .await
            .map_err(|e| Error::Connection(e.to_string()))?;

        info!("✅ WebSocket message sent in {:?}", start.elapsed());
        Ok(())
    }

    /// Receive message from WebSocket
    #[instrument(level = "debug", skip(self))]
    pub async fn receive(&self) -> Result<Message, Error> {
        let start = Instant::now();
        info!("🔧 Receiving WebSocket message");

        let mut connection = self.connection.write().await;
        let connection = connection.as_mut().ok_or_else(|| {
            Error::Connection("WebSocket connection not established".to_string())
        })?;

        let message = connection
            .next()
            .await
            .ok_or_else(|| Error::Connection("WebSocket stream ended".to_string()))?
            .map_err(|e| Error::Connection(e.to_string()))?;

        let message = match message {
            WsMessage::Text(text) => serde_json::from_str(&text).map_err(|e| {
                Error::Serialization(e)
            })?,
            _ => return Err(Error::Connection("Invalid WebSocket message type".to_string())),
        };

        info!("✅ WebSocket message received in {:?}", start.elapsed());
        Ok(message)
    }
}

#[async_trait]
impl super::Transport for WebSocketTransport {
    #[instrument(level = "debug", skip(self))]
    async fn start(&self) -> Result<(), Error> {
        self.connect().await
    }

    #[instrument(level = "debug", skip(self))]
    async fn stop(&self) -> Result<(), Error> {
        self.disconnect().await
    }

    #[instrument(level = "debug", skip(self, message))]
    async fn send(&self, message: Message) -> Result<(), Error> {
        self.send(message).await
    }

    #[instrument(level = "debug", skip(self))]
    async fn receive(&self) -> Result<Message, Error> {
        self.receive().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;
    use tokio_tungstenite::accept_async;
    use crate::message::MessageType;

    #[tokio::test]
    async fn test_websocket_connection() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            let _ws_stream = accept_async(socket).await.unwrap();
        });

        let transport = WebSocketTransport::new(format!("ws://{}", addr));
        assert!(transport.connect().await.is_ok());
        assert!(transport.disconnect().await.is_ok());
    }

    #[tokio::test]
    async fn test_websocket_message_exchange() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            let mut ws_stream = accept_async(socket).await.unwrap();

            let message = ws_stream.next().await.unwrap().unwrap();
            ws_stream.send(message).await.unwrap();
        });

        let transport = WebSocketTransport::new(format!("ws://{}", addr));
        transport.connect().await.unwrap();

        let test_message = Message::new(
            MessageType::Data,
            "sender".to_string(),
            "receiver".to_string(),
            serde_json::json!({"data": "test"}),
            serde_json::json!({}),
        );

        assert!(transport.send(test_message.clone()).await.is_ok());
        let received = transport.receive().await.unwrap();
        assert_eq!(received.id(), test_message.id());

        transport.disconnect().await.unwrap();
    }

    #[tokio::test]
    async fn test_websocket_transport() {
        let transport = WebSocketTransport::new("ws://localhost:8080".to_string());
        let test_message = Message::new(
            MessageType::Data,
            "sender".to_string(),
            "receiver".to_string(),
            serde_json::json!({"data": "test"}),
            serde_json::json!({}),
        );

        let result = transport.send(test_message).await;
        assert!(result.is_err()); // Should fail since no server is running
    }
}
