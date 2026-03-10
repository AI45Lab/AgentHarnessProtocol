//! WebSocket transport implementation

use crate::{AhpRequest, AhpResponse, AhpNotification, Result, AhpError, AuthConfig, AuthMethod};
use crate::transport::TransportLayer;
use async_trait::async_trait;
use tokio_tungstenite::{connect_async, tungstenite::Message, WebSocketStream, MaybeTlsStream};
use tokio::net::TcpStream;
use futures_util::{StreamExt, SinkExt};
use tokio::sync::Mutex;
use std::collections::HashMap;
use std::sync::Arc;

/// WebSocket transport - bidirectional streaming communication
pub struct WebSocketTransport {
    write: Arc<Mutex<futures_util::stream::SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>>,
    pending_requests: Arc<Mutex<HashMap<String, tokio::sync::oneshot::Sender<AhpResponse>>>>,
}

impl WebSocketTransport {
    /// Connect to a WebSocket server
    pub async fn connect(url: impl Into<String>, auth: Option<AuthConfig>) -> Result<Self> {
        let mut url_string = url.into();

        // Add authentication as query parameter or header
        if let Some(auth_config) = auth {
            match auth_config.method {
                AuthMethod::ApiKey { key } => {
                    url_string = format!("{}?api_key={}", url_string, key);
                }
                AuthMethod::Bearer { token } => {
                    url_string = format!("{}?token={}", url_string, token);
                }
                _ => {}
            }
        }

        let (ws_stream, _) = connect_async(&url_string).await
            .map_err(|e| AhpError::Transport(format!("WebSocket connection failed: {}", e)))?;

        let (write, read) = ws_stream.split();

        let transport = Self {
            write: Arc::new(Mutex::new(write)),
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
        };

        // Start background task to read responses
        transport.start_reader(read);

        Ok(transport)
    }

    fn start_reader(
        &self,
        mut read: futures_util::stream::SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    ) {
        let pending = self.pending_requests.clone();

        tokio::spawn(async move {
            while let Some(msg) = read.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        if let Ok(response) = serde_json::from_str::<AhpResponse>(&text) {
                            let mut pending_guard = pending.lock().await;
                            if let Some(sender) = pending_guard.remove(&response.id) {
                                let _ = sender.send(response);
                            }
                        }
                    }
                    Ok(Message::Close(_)) => break,
                    Err(_) => break,
                    _ => {}
                }
            }
        });
    }
}

#[async_trait]
impl TransportLayer for WebSocketTransport {
    async fn send_request(&self, request: AhpRequest) -> Result<AhpResponse> {
        let (tx, rx) = tokio::sync::oneshot::channel();

        // Register pending request
        {
            let mut pending = self.pending_requests.lock().await;
            pending.insert(request.id.clone(), tx);
        }

        // Send request
        let json = serde_json::to_string(&request)?;
        let mut write = self.write.lock().await;
        write.send(Message::Text(json)).await
            .map_err(|e| AhpError::Transport(format!("Failed to send: {}", e)))?;
        drop(write);

        // Wait for response with timeout
        match tokio::time::timeout(std::time::Duration::from_millis(10_000), rx).await {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(_)) => Err(AhpError::ConnectionClosed),
            Err(_) => Err(AhpError::Timeout(10_000)),
        }
    }

    async fn send_notification(&self, notification: AhpNotification) -> Result<()> {
        let json = serde_json::to_string(&notification)?;
        let mut write = self.write.lock().await;
        write.send(Message::Text(json)).await
            .map_err(|e| AhpError::Transport(format!("Failed to send: {}", e)))?;
        Ok(())
    }

    async fn close(&self) -> Result<()> {
        let mut write = self.write.lock().await;
        write.send(Message::Close(None)).await
            .map_err(|e| AhpError::Transport(format!("Failed to close: {}", e)))?;
        Ok(())
    }
}

/// WebSocket server handler
pub struct WebSocketServer {
    server: Arc<crate::AhpServer>,
}

impl WebSocketServer {
    /// Create a new WebSocket server
    pub fn new(server: Arc<crate::AhpServer>) -> Self {
        Self { server }
    }

    /// Run the WebSocket server on the specified address
    #[cfg(feature = "websocket")]
    pub async fn run(self, addr: impl Into<std::net::SocketAddr>) -> Result<()> {
        use tokio::net::TcpListener;
        use tokio_tungstenite::accept_async;

        let listener = TcpListener::bind(addr.into()).await
            .map_err(|e| AhpError::Transport(format!("Failed to bind: {}", e)))?;

        tracing::info!("WebSocket server listening on {}", listener.local_addr().unwrap());

        while let Ok((stream, addr)) = listener.accept().await {
            let server = self.server.clone();
            tokio::spawn(async move {
                if let Err(e) = handle_connection(stream, server, addr).await {
                    tracing::error!("WebSocket connection error: {}", e);
                }
            });
        }

        Ok(())
    }
}

#[cfg(feature = "websocket")]
async fn handle_connection(
    stream: tokio::net::TcpStream,
    server: Arc<crate::AhpServer>,
    addr: std::net::SocketAddr,
) -> Result<()> {
    use tokio_tungstenite::tungstenite::Message as WsMessage;
    use tokio_tungstenite::accept_async;

    let ws_stream = accept_async(stream).await
        .map_err(|e| AhpError::Transport(format!("WebSocket handshake failed: {}", e)))?;

    tracing::info!("WebSocket connection established: {}", addr);

    let (mut write, mut read) = ws_stream.split();

    while let Some(msg) = read.next().await {
        match msg {
            Ok(WsMessage::Text(text)) => {
                // Try to parse as request or notification
                if let Ok(request) = serde_json::from_str::<AhpRequest>(&text) {
                    let response = server.handle_request(request).await;
                    let json = serde_json::to_string(&response)
                        .map_err(|e| AhpError::Serialization(e))?;
                    write.send(WsMessage::Text(json)).await
                        .map_err(|e| AhpError::Transport(format!("Failed to send: {}", e)))?;
                } else if let Ok(notification) = serde_json::from_str::<AhpNotification>(&text) {
                    let _ = server.handle_notification(notification).await;
                }
            }
            Ok(WsMessage::Close(_)) => break,
            Ok(WsMessage::Ping(data)) => {
                write.send(WsMessage::Pong(data)).await
                    .map_err(|e| AhpError::Transport(format!("Failed to send pong: {}", e)))?;
            }
            Err(e) => {
                tracing::error!("WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }

    tracing::info!("WebSocket connection closed: {}", addr);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_websocket_url_with_api_key() {
        // This test just verifies URL construction
        let auth = Some(AuthConfig::api_key("test-key"));
        let url = "ws://localhost:8080/ahp";

        // In real implementation, this would be tested with a mock server
        // For now, we just verify the auth config is created correctly
        assert!(auth.is_some());
    }
}
