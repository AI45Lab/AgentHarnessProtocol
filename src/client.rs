//! AHP client implementation

use crate::protocol::AgentInfo;
use crate::transport::TransportLayer;
use crate::{
    AhpError, AhpEvent, AhpNotification, AhpRequest, BatchRequest, BatchResponse, Decision,
    EventType, HandshakeRequest, HandshakeResponse, QueryRequest, QueryResponse, Result, Transport,
    TransportConfig, PROTOCOL_VERSION,
};
use std::sync::Arc;

/// AHP client - sends events to harness server
pub struct AhpClient {
    transport: Arc<dyn TransportLayer>,
    session_id: String,
    agent_id: String,
    _config: TransportConfig,
    handshake_done: std::sync::atomic::AtomicBool,
}

impl AhpClient {
    /// Create a new AHP client with the specified transport
    pub async fn new(transport: Transport) -> Result<Self> {
        let transport_layer: Arc<dyn TransportLayer> = match transport {
            #[cfg(feature = "stdio")]
            Transport::Stdio { program, args } => {
                Arc::new(crate::transport::stdio::StdioTransport::spawn(program, &args).await?)
            }

            #[cfg(feature = "http")]
            Transport::Http { url, auth } => {
                Arc::new(crate::transport::http::HttpTransport::new(url, auth)?)
            }

            #[cfg(feature = "websocket")]
            Transport::WebSocket { url, auth } => {
                Arc::new(crate::transport::websocket::WebSocketTransport::connect(url, auth).await?)
            }

            #[cfg(feature = "grpc")]
            Transport::Grpc {
                endpoint: _,
                auth: _,
            } => {
                return Err(AhpError::UnsupportedCapability(
                    "gRPC transport not yet implemented".to_string(),
                ));
            }

            #[cfg(feature = "unix-socket")]
            Transport::UnixSocket { path: _ } => {
                return Err(AhpError::UnsupportedCapability(
                    "Unix socket transport not yet implemented".to_string(),
                ));
            }

            #[allow(unreachable_patterns)]
            _ => {
                return Err(AhpError::UnsupportedCapability(
                    "Transport not enabled".to_string(),
                ))
            }
        };

        Ok(Self {
            transport: transport_layer,
            session_id: uuid::Uuid::new_v4().to_string(),
            agent_id: uuid::Uuid::new_v4().to_string(),
            _config: TransportConfig::default(),
            handshake_done: std::sync::atomic::AtomicBool::new(false),
        })
    }

    /// Create a new AHP client with a pre-configured transport layer (for testing).
    ///
    /// This bypasses transport selection logic and uses the provided transport directly.
    /// No handshake is performed - the caller is responsible for setting up the transport.
    pub fn new_for_testing(transport: Arc<dyn TransportLayer>) -> Self {
        Self {
            transport,
            session_id: uuid::Uuid::new_v4().to_string(),
            agent_id: uuid::Uuid::new_v4().to_string(),
            _config: TransportConfig::default(),
            handshake_done: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// Perform handshake with harness server
    ///
    /// # Arguments
    ///
    /// * `capabilities` - List of capability strings the agent supports
    ///
    pub async fn handshake(&self, capabilities: Vec<String>) -> Result<HandshakeResponse> {
        let request = HandshakeRequest {
            protocol_version: PROTOCOL_VERSION.to_string(),
            agent_info: AgentInfo {
                framework: "a3s-ahp".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                capabilities,
            },
            session_id: self.session_id.clone(),
            agent_id: self.agent_id.clone(),
        };

        let ahp_request = AhpRequest::new("ahp/handshake", serde_json::to_value(&request)?);
        let response = self.transport.send_request(ahp_request).await?;

        if let Some(error) = response.error {
            return Err(AhpError::Protocol(format!(
                "Handshake failed: {}",
                error.message
            )));
        }

        let handshake_response: HandshakeResponse = serde_json::from_value(
            response
                .result
                .ok_or_else(|| AhpError::Protocol("Missing result".to_string()))?,
        )?;

        self.handshake_done
            .store(true, std::sync::atomic::Ordering::Relaxed);

        Ok(handshake_response)
    }

    /// Send an event and wait for decision (blocking events only).
    ///
    /// Returns the raw JSON decision payload. Callers should use the event type
    /// to deserialize into the appropriate specialized decision type.
    pub async fn send_event(
        &self,
        event_type: EventType,
        payload: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let event = AhpEvent {
            event_type,
            session_id: self.session_id.clone(),
            agent_id: self.agent_id.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            depth: 0,
            payload,
            context: None,
            metadata: None,
        };

        if event_type.is_blocking() {
            let ahp_request = AhpRequest::new("ahp/event", serde_json::to_value(&event)?);
            let response = self.transport.send_request(ahp_request).await?;

            if let Some(error) = response.error {
                return Err(AhpError::Protocol(format!(
                    "Event failed: {}",
                    error.message
                )));
            }

            Ok(response
                .result
                .ok_or_else(|| AhpError::Protocol("Missing result".to_string()))?)
        } else {
            // Fire-and-forget notification
            let notification = AhpNotification::new("ahp/event", serde_json::to_value(&event)?);
            self.transport.send_notification(notification).await?;

            // Return default allow decision for notifications
            Ok(serde_json::json!({"decision": "allow"}))
        }
    }

    /// Send a complete event (with context) and wait for decision.
    ///
    /// Unlike `send_event` which creates a new AhpEvent, this method accepts
    /// the full event with pre-built context and metadata.
    pub async fn send_event_full(&self, event: &AhpEvent) -> Result<Decision> {
        if event.event_type.is_blocking() {
            let ahp_request = AhpRequest::new("ahp/event", serde_json::to_value(event)?);
            let response = self.transport.send_request(ahp_request).await?;

            if let Some(error) = response.error {
                return Err(AhpError::Protocol(format!(
                    "Event failed: {}",
                    error.message
                )));
            }

            let decision: Decision = serde_json::from_value(
                response
                    .result
                    .ok_or_else(|| AhpError::Protocol("Missing result".to_string()))?,
            )?;

            Ok(decision)
        } else {
            // Fire-and-forget notification
            let notification = AhpNotification::new("ahp/event", serde_json::to_value(event)?);
            self.transport.send_notification(notification).await?;

            // Return default allow decision for notifications
            Ok(Decision::Allow {
                modified_payload: None,
                metadata: None,
            })
        }
    }

    /// Send a query to the harness
    pub async fn query(
        &self,
        query_type: impl Into<String>,
        payload: serde_json::Value,
    ) -> Result<QueryResponse> {
        let query = QueryRequest {
            session_id: self.session_id.clone(),
            agent_id: self.agent_id.clone(),
            query_type: query_type.into(),
            payload,
        };

        let ahp_request = AhpRequest::new("ahp/query", serde_json::to_value(&query)?);
        let response = self.transport.send_request(ahp_request).await?;

        if let Some(error) = response.error {
            return Err(AhpError::Protocol(format!(
                "Query failed: {}",
                error.message
            )));
        }

        let query_response: QueryResponse = serde_json::from_value(
            response
                .result
                .ok_or_else(|| AhpError::Protocol("Missing result".to_string()))?,
        )?;

        Ok(query_response)
    }

    /// Send a batch of events
    pub async fn send_batch(&self, events: Vec<AhpEvent>) -> Result<BatchResponse> {
        let batch = BatchRequest { events };

        let ahp_request = AhpRequest::new("ahp/batch", serde_json::to_value(&batch)?);
        let response = self.transport.send_request(ahp_request).await?;

        if let Some(error) = response.error {
            return Err(AhpError::Protocol(format!(
                "Batch failed: {}",
                error.message
            )));
        }

        let batch_response: BatchResponse = serde_json::from_value(
            response
                .result
                .ok_or_else(|| AhpError::Protocol("Missing result".to_string()))?,
        )?;

        Ok(batch_response)
    }

    /// Close the client connection
    pub async fn close(&self) -> Result<()> {
        self.transport.close().await
    }
}
