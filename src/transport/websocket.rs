//! WebSocket transport implementation

use crate::transport::TransportLayer;
use crate::{
    AhpError, AhpNotification, AhpRequest, AhpResponse, AuthConfig, AuthMethod, Result,
    TransportConfig,
};
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};

/// WebSocket transport - bidirectional streaming communication
pub struct WebSocketTransport {
    write: Arc<
        Mutex<futures_util::stream::SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>,
    >,
    pending_requests: Arc<Mutex<HashMap<String, tokio::sync::oneshot::Sender<AhpResponse>>>>,
    timeout_ms: u64,
}

impl WebSocketTransport {
    /// Connect to a WebSocket server
    pub async fn connect(url: impl Into<String>, auth: Option<AuthConfig>) -> Result<Self> {
        Self::connect_with_config(url, auth, &TransportConfig::default()).await
    }

    /// Connect to a WebSocket server with explicit config.
    pub async fn connect_with_config(
        url: impl Into<String>,
        auth: Option<AuthConfig>,
        config: &TransportConfig,
    ) -> Result<Self> {
        let url_string = build_websocket_url(url.into(), auth)?;

        let (ws_stream, _) = connect_async(&url_string)
            .await
            .map_err(|e| AhpError::Transport(format!("WebSocket connection failed: {}", e)))?;

        let (write, read) = ws_stream.split();

        let transport = Self {
            write: Arc::new(Mutex::new(write)),
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
            timeout_ms: config.timeout_ms,
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

fn build_websocket_url(url: String, auth: Option<AuthConfig>) -> Result<String> {
    let mut parsed = url::Url::parse(&url)
        .map_err(|e| AhpError::Transport(format!("Invalid WebSocket URL: {}", e)))?;

    if let Some(auth_config) = auth {
        match auth_config.method {
            AuthMethod::ApiKey { key } => {
                parsed.query_pairs_mut().append_pair("api_key", &key);
            }
            AuthMethod::Bearer { token } => {
                parsed.query_pairs_mut().append_pair("token", &token);
            }
            _ => {}
        }
    }

    Ok(parsed.into())
}

#[async_trait]
impl TransportLayer for WebSocketTransport {
    async fn send_request(&self, request: AhpRequest) -> Result<AhpResponse> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let request_id = request.id.clone();
        let json = serde_json::to_string(&request)?;

        // Register pending request
        {
            let mut pending = self.pending_requests.lock().await;
            pending.insert(request_id.clone(), tx);
        }

        // Send request
        let mut write = self.write.lock().await;
        if let Err(e) = write.send(Message::Text(json)).await {
            self.pending_requests.lock().await.remove(&request_id);
            return Err(AhpError::Transport(format!("Failed to send: {}", e)));
        }
        drop(write);

        // Wait for response with timeout
        match tokio::time::timeout(std::time::Duration::from_millis(self.timeout_ms), rx).await {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(_)) => Err(AhpError::ConnectionClosed),
            Err(_) => {
                self.pending_requests.lock().await.remove(&request_id);
                Err(AhpError::Timeout(self.timeout_ms))
            }
        }
    }

    async fn send_notification(&self, notification: AhpNotification) -> Result<()> {
        let json = serde_json::to_string(&notification)?;
        let mut write = self.write.lock().await;
        write
            .send(Message::Text(json))
            .await
            .map_err(|e| AhpError::Transport(format!("Failed to send: {}", e)))?;
        Ok(())
    }

    async fn close(&self) -> Result<()> {
        let mut write = self.write.lock().await;
        write
            .send(Message::Close(None))
            .await
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

        let listener = TcpListener::bind(addr.into())
            .await
            .map_err(|e| AhpError::Transport(format!("Failed to bind: {}", e)))?;

        tracing::info!(
            "WebSocket server listening on {}",
            listener.local_addr().unwrap()
        );

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
    use tokio_tungstenite::accept_async;
    use tokio_tungstenite::tungstenite::Message as WsMessage;

    let ws_stream = accept_async(stream)
        .await
        .map_err(|e| AhpError::Transport(format!("WebSocket handshake failed: {}", e)))?;

    tracing::info!("WebSocket connection established: {}", addr);

    let (mut write, mut read) = ws_stream.split();

    while let Some(msg) = read.next().await {
        match msg {
            Ok(WsMessage::Text(text)) => {
                // Try to parse as request or notification
                if let Ok(request) = serde_json::from_str::<AhpRequest>(&text) {
                    let response = server.handle_request(request).await;
                    let json =
                        serde_json::to_string(&response).map_err(|e| AhpError::Serialization(e))?;
                    write
                        .send(WsMessage::Text(json))
                        .await
                        .map_err(|e| AhpError::Transport(format!("Failed to send: {}", e)))?;
                } else if let Ok(notification) = serde_json::from_str::<AhpNotification>(&text) {
                    let _ = server.handle_notification(notification).await;
                }
            }
            Ok(WsMessage::Close(_)) => break,
            Ok(WsMessage::Ping(data)) => {
                write
                    .send(WsMessage::Pong(data))
                    .await
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

    #[test]
    fn websocket_url_appends_and_encodes_api_key() {
        let auth = Some(AuthConfig::api_key("test-key"));
        let url = build_websocket_url("ws://localhost:8080/ahp?existing=1".to_string(), auth)
            .expect("url should build");

        assert_eq!(url, "ws://localhost:8080/ahp?existing=1&api_key=test-key");
    }

    #[test]
    fn websocket_url_percent_encodes_bearer_token() {
        let auth = Some(AuthConfig::bearer("token with space&symbols"));
        let url = build_websocket_url("ws://localhost:8080/ahp".to_string(), auth)
            .expect("url should build");

        assert_eq!(
            url,
            "ws://localhost:8080/ahp?token=token+with+space%26symbols"
        );
    }
}
