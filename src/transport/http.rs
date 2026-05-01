//! HTTP transport implementation

use crate::transport::TransportLayer;
use crate::{
    AhpError, AhpNotification, AhpRequest, AhpResponse, AuthConfig, AuthMethod, Result,
    TransportConfig,
};
use async_trait::async_trait;
use reqwest::{header, Client};
use std::sync::Arc;

/// HTTP transport - communicates with remote harness server via HTTP
pub struct HttpTransport {
    client: Client,
    url: String,
    #[allow(dead_code)]
    auth: Option<AuthConfig>,
}

impl HttpTransport {
    /// Create a new HTTP transport
    pub fn new(url: impl Into<String>, auth: Option<AuthConfig>) -> Result<Self> {
        Self::new_with_config(url, auth, &TransportConfig::default())
    }

    /// Create a new HTTP transport with explicit config.
    pub fn new_with_config(
        url: impl Into<String>,
        auth: Option<AuthConfig>,
        config: &TransportConfig,
    ) -> Result<Self> {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );

        // Add authentication headers
        if let Some(ref auth_config) = auth {
            match &auth_config.method {
                AuthMethod::ApiKey { key } => {
                    headers.insert(
                        "X-API-Key",
                        key.parse()
                            .map_err(|_| AhpError::AuthFailed("Invalid API key".to_string()))?,
                    );
                }
                AuthMethod::Bearer { token } => {
                    let auth_value = format!("Bearer {}", token);
                    headers.insert(
                        header::AUTHORIZATION,
                        auth_value.parse().map_err(|_| {
                            AhpError::AuthFailed("Invalid bearer token".to_string())
                        })?,
                    );
                }
                _ => {}
            }
        }

        let client = Client::builder()
            .default_headers(headers)
            .timeout(std::time::Duration::from_millis(config.timeout_ms))
            .build()
            .map_err(|e| AhpError::Transport(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            client,
            url: url.into(),
            auth,
        })
    }
}

#[async_trait]
impl TransportLayer for HttpTransport {
    async fn send_request(&self, request: AhpRequest) -> Result<AhpResponse> {
        let response = self
            .client
            .post(&self.url)
            .json(&request)
            .send()
            .await
            .map_err(|e| AhpError::Transport(format!("HTTP request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(AhpError::Transport(format!(
                "HTTP error: {} - {}",
                response.status(),
                response.text().await.unwrap_or_default()
            )));
        }

        let ahp_response: AhpResponse = response
            .json()
            .await
            .map_err(|e| AhpError::Protocol(format!("Failed to parse response: {}", e)))?;

        Ok(ahp_response)
    }

    async fn send_notification(&self, notification: AhpNotification) -> Result<()> {
        // For HTTP, notifications are sent as POST requests but we don't wait for response
        let _ = self
            .client
            .post(&self.url)
            .json(&notification)
            .send()
            .await
            .map_err(|e| AhpError::Transport(format!("HTTP notification failed: {}", e)))?;

        Ok(())
    }

    async fn close(&self) -> Result<()> {
        // HTTP client doesn't need explicit cleanup
        Ok(())
    }
}

/// HTTP server handler
pub struct HttpServer {
    server: Arc<crate::AhpServer>,
}

impl HttpServer {
    /// Create a new HTTP server
    pub fn new(server: Arc<crate::AhpServer>) -> Self {
        Self { server }
    }

    /// Run the HTTP server on the specified address
    #[cfg(feature = "http")]
    pub async fn run(self, addr: impl Into<std::net::SocketAddr>) -> Result<()> {
        use axum::{routing::post, Router};
        use tower_http::trace::TraceLayer;

        let app = Router::new()
            .route("/ahp", post(handle_request))
            .layer(TraceLayer::new_for_http())
            .with_state(self.server);

        let listener = tokio::net::TcpListener::bind(addr.into())
            .await
            .map_err(|e| AhpError::Transport(format!("Failed to bind: {}", e)))?;

        axum::serve(listener, app)
            .await
            .map_err(|e| AhpError::Transport(format!("Server error: {}", e)))?;

        Ok(())
    }
}

#[cfg(feature = "http")]
async fn handle_request(
    axum::extract::State(server): axum::extract::State<Arc<crate::AhpServer>>,
    axum::extract::Json(message): axum::extract::Json<serde_json::Value>,
) -> axum::response::Response {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    if let Ok(request) = serde_json::from_value::<AhpRequest>(message.clone()) {
        let response = server.handle_request(request).await;
        return axum::response::Json(response).into_response();
    }

    if let Ok(notification) = serde_json::from_value::<AhpNotification>(message) {
        return match server.handle_notification(notification).await {
            Ok(()) => StatusCode::NO_CONTENT.into_response(),
            Err(e) => axum::response::Json(AhpResponse::error(
                "",
                -32603,
                format!("Handler error: {}", e),
            ))
            .into_response(),
        };
    }

    axum::response::Json(AhpResponse::error("", -32600, "Invalid JSON-RPC message")).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::EventHandler;
    use crate::{AhpEvent, Decision};
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_http_transport_creation() {
        let transport = HttpTransport::new("http://localhost:8080/ahp", None);
        assert!(transport.is_ok());
    }

    #[test]
    fn test_http_transport_with_api_key() {
        let auth = Some(AuthConfig::api_key("test-key"));
        let transport = HttpTransport::new("http://localhost:8080/ahp", auth);
        assert!(transport.is_ok());
    }

    #[test]
    fn test_http_transport_with_bearer() {
        let auth = Some(AuthConfig::bearer("test-token"));
        let transport = HttpTransport::new("http://localhost:8080/ahp", auth);
        assert!(transport.is_ok());
    }

    struct CountingHandler {
        notifications: AtomicUsize,
    }

    #[async_trait]
    impl EventHandler for CountingHandler {
        async fn handle_event(&self, _event: &AhpEvent) -> Result<Decision> {
            Ok(Decision::Allow {
                modified_payload: None,
                metadata: None,
            })
        }

        async fn handle_notification(&self, _event: &AhpEvent) -> Result<()> {
            self.notifications.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
    }

    #[tokio::test]
    async fn http_handler_accepts_notifications() {
        let handler = Arc::new(CountingHandler {
            notifications: AtomicUsize::new(0),
        });
        let server = Arc::new(crate::AhpServer::new(handler.clone()));
        let event = AhpEvent {
            event_type: crate::EventType::PostAction,
            session_id: "session-1".to_string(),
            agent_id: "agent-1".to_string(),
            timestamp: "2026-05-01T00:00:00Z".to_string(),
            depth: 0,
            payload: serde_json::json!({"status": "ok"}),
            context: None,
            metadata: None,
        };
        let notification = AhpNotification::new("ahp/event", serde_json::to_value(event).unwrap());

        let response = handle_request(
            axum::extract::State(server),
            axum::extract::Json(serde_json::to_value(notification).unwrap()),
        )
        .await;

        assert_eq!(response.status(), axum::http::StatusCode::NO_CONTENT);
        assert_eq!(handler.notifications.load(Ordering::Relaxed), 1);
    }
}
