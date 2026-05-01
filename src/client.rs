//! AHP client implementation

use crate::protocol::AgentInfo;
use crate::transport::TransportLayer;
use crate::{
    AhpError, AhpEvent, AhpNotification, AhpRequest, BatchRequest, BatchResponse, Decision,
    EventType, HandshakeRequest, HandshakeResponse, QueryRequest, QueryResponse, Result, Transport,
    TransportConfig, PROTOCOL_VERSION,
};
use serde::de::DeserializeOwned;
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
        Self::new_with_config(transport, TransportConfig::default()).await
    }

    /// Create a new AHP client with the specified transport and transport config.
    pub async fn new_with_config(transport: Transport, config: TransportConfig) -> Result<Self> {
        #[cfg(not(any(
            feature = "stdio",
            feature = "http",
            feature = "websocket",
            feature = "grpc",
            feature = "unix-socket"
        )))]
        {
            let _ = (transport, config);
            return Err(AhpError::UnsupportedCapability(
                "No transport features enabled".to_string(),
            ));
        }

        #[cfg(any(
            feature = "stdio",
            feature = "http",
            feature = "websocket",
            feature = "grpc",
            feature = "unix-socket"
        ))]
        {
            let transport_layer: Arc<dyn TransportLayer> = match transport {
                #[cfg(feature = "stdio")]
                Transport::Stdio { program, args } => Arc::new(
                    crate::transport::stdio::StdioTransport::spawn_with_config(
                        program, &args, &config,
                    )
                    .await?,
                ),

                #[cfg(feature = "http")]
                Transport::Http { url, auth } => Arc::new(
                    crate::transport::http::HttpTransport::new_with_config(url, auth, &config)?,
                ),

                #[cfg(feature = "websocket")]
                Transport::WebSocket { url, auth } => Arc::new(
                    crate::transport::websocket::WebSocketTransport::connect_with_config(
                        url, auth, &config,
                    )
                    .await?,
                ),

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
                Transport::UnixSocket { path } => Arc::new(
                    crate::transport::unix_socket::UnixSocketTransport::connect_with_config(
                        path, &config,
                    )
                    .await?,
                ),

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
                _config: config,
                handshake_done: std::sync::atomic::AtomicBool::new(false),
            })
        }
    }

    /// Create a new AHP client with a pre-configured transport layer (for testing).
    ///
    /// This bypasses transport selection logic and uses the provided transport directly.
    /// Test clients are treated as handshaken so unit tests can focus on transport behavior.
    pub fn new_for_testing(transport: Arc<dyn TransportLayer>) -> Self {
        Self {
            transport,
            session_id: uuid::Uuid::new_v4().to_string(),
            agent_id: uuid::Uuid::new_v4().to_string(),
            _config: TransportConfig::default(),
            handshake_done: std::sync::atomic::AtomicBool::new(true),
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

        let result = self
            .send_rpc_request(
                "ahp/handshake",
                serde_json::to_value(&request)?,
                "Handshake",
            )
            .await?;
        let handshake_response: HandshakeResponse = serde_json::from_value(result)?;

        self.handshake_done
            .store(true, std::sync::atomic::Ordering::Release);

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
        self.ensure_handshake()?;

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
            self.send_rpc_request("ahp/event", serde_json::to_value(&event)?, "Event")
                .await
        } else {
            // Fire-and-forget notification
            let notification = AhpNotification::new("ahp/event", serde_json::to_value(&event)?);
            self.transport.send_notification(notification).await?;

            // Return default allow decision for notifications
            Ok(serde_json::json!({"decision": "allow"}))
        }
    }

    /// Send an event and deserialize the response as the generic AHP decision.
    ///
    /// Use `send_typed_event` for harness points that return specialized decision
    /// shapes such as `ContextPerceptionDecision`.
    pub async fn send_event_decision(
        &self,
        event_type: EventType,
        payload: serde_json::Value,
    ) -> Result<Decision> {
        let value = self.send_event(event_type, payload).await?;
        Ok(serde_json::from_value(value)?)
    }

    /// Send an event and deserialize the response into a caller-selected type.
    pub async fn send_typed_event<T>(
        &self,
        event_type: EventType,
        payload: serde_json::Value,
    ) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let value = self.send_event(event_type, payload).await?;
        Ok(serde_json::from_value(value)?)
    }

    /// Send a complete event (with context) and wait for decision.
    ///
    /// Unlike `send_event` which creates a new AhpEvent, this method accepts
    /// the full event with pre-built context and metadata.
    pub async fn send_event_full(&self, event: &AhpEvent) -> Result<Decision> {
        self.ensure_handshake()?;

        if event.event_type.is_blocking() {
            let result = self
                .send_rpc_request("ahp/event", serde_json::to_value(event)?, "Event")
                .await?;
            let decision: Decision = serde_json::from_value(result)?;

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
        self.ensure_handshake()?;

        let query = QueryRequest {
            session_id: self.session_id.clone(),
            agent_id: self.agent_id.clone(),
            query_type: query_type.into(),
            payload,
        };

        let result = self
            .send_rpc_request("ahp/query", serde_json::to_value(&query)?, "Query")
            .await?;
        let query_response: QueryResponse = serde_json::from_value(result)?;

        Ok(query_response)
    }

    /// Send a batch of events
    pub async fn send_batch(&self, events: Vec<AhpEvent>) -> Result<BatchResponse> {
        self.ensure_handshake()?;

        if let Some(event) = events.iter().find(|event| !event.event_type.is_batchable()) {
            return Err(AhpError::Protocol(format!(
                "Batch failed: event type {} cannot be batched because it does not return a generic Decision",
                event.event_type
            )));
        }

        let event_count = events.len();
        let batch = BatchRequest { events };

        let result = self
            .send_rpc_request("ahp/batch", serde_json::to_value(&batch)?, "Batch")
            .await?;
        let batch_response: BatchResponse = serde_json::from_value(result)?;

        if batch_response.decisions.len() != event_count {
            return Err(AhpError::Protocol(format!(
                "Batch failed: decision count mismatch, expected {}, got {}",
                event_count,
                batch_response.decisions.len()
            )));
        }

        Ok(batch_response)
    }

    /// Close the client connection
    pub async fn close(&self) -> Result<()> {
        self.transport.close().await
    }

    fn ensure_handshake(&self) -> Result<()> {
        if self
            .handshake_done
            .load(std::sync::atomic::Ordering::Acquire)
        {
            Ok(())
        } else {
            Err(AhpError::Protocol(
                "Handshake must complete before sending AHP operations".to_string(),
            ))
        }
    }

    async fn send_rpc_request(
        &self,
        method: impl Into<String>,
        params: serde_json::Value,
        operation: &str,
    ) -> Result<serde_json::Value> {
        let request = AhpRequest::new(method, params);
        let request_id = request.id.clone();
        let response = self.transport.send_request(request).await?;

        if response.jsonrpc != "2.0" {
            return Err(AhpError::Protocol(format!(
                "{} failed: invalid JSON-RPC version {}",
                operation, response.jsonrpc
            )));
        }

        if response.id != request_id {
            return Err(AhpError::Protocol(format!(
                "{} failed: response id mismatch, expected {}, got {}",
                operation, request_id, response.id
            )));
        }

        if let Some(error) = response.error {
            return Err(AhpError::Protocol(format!(
                "{} failed: {}",
                operation, error.message
            )));
        }

        response
            .result
            .ok_or_else(|| AhpError::Protocol(format!("{} failed: missing result", operation)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{
        AhpResponse, BatchResponse, ContextPerceptionDecision, Fact, InjectedContext,
    };
    use async_trait::async_trait;

    struct StaticTransport {
        response: AhpResponse,
        echo_request_id: bool,
    }

    #[async_trait]
    impl TransportLayer for StaticTransport {
        async fn send_request(&self, request: AhpRequest) -> Result<AhpResponse> {
            let mut response = self.response.clone();
            if self.echo_request_id {
                response.id = request.id;
            }
            Ok(response)
        }

        async fn send_notification(&self, _notification: AhpNotification) -> Result<()> {
            Ok(())
        }

        async fn close(&self) -> Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn send_typed_event_preserves_specialized_decision_payload() {
        let decision = ContextPerceptionDecision::Allow {
            injected_context: InjectedContext {
                facts: vec![Fact {
                    content: "workspace uses Rust".to_string(),
                    source: "test".to_string(),
                    confidence: 0.9,
                }],
                file_contents: None,
                project_summary: None,
                knowledge: None,
                suggestions: None,
            },
            metadata: None,
        };
        let transport = Arc::new(StaticTransport {
            response: AhpResponse::success("placeholder", serde_json::to_value(decision).unwrap()),
            echo_request_id: true,
        });
        let client = AhpClient::new_for_testing(transport);

        let response: ContextPerceptionDecision = client
            .send_typed_event(EventType::ContextPerception, serde_json::json!({}))
            .await
            .expect("typed decision should deserialize");

        match response {
            ContextPerceptionDecision::Allow {
                injected_context, ..
            } => {
                assert_eq!(injected_context.facts[0].content, "workspace uses Rust");
            }
            other => panic!("expected allow decision, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn send_event_rejects_mismatched_response_id() {
        let transport = Arc::new(StaticTransport {
            response: AhpResponse::success("wrong-id", serde_json::json!({"decision": "allow"})),
            echo_request_id: false,
        });
        let client = AhpClient::new_for_testing(transport);

        let error = client
            .send_event(EventType::PreAction, serde_json::json!({}))
            .await
            .expect_err("mismatched response id should fail");

        assert!(error.to_string().contains("response id mismatch"));
    }

    #[tokio::test]
    async fn send_event_requires_handshake() {
        let transport = Arc::new(StaticTransport {
            response: AhpResponse::success("placeholder", serde_json::json!({"decision": "allow"})),
            echo_request_id: true,
        });
        let client = AhpClient {
            transport,
            session_id: "session-1".to_string(),
            agent_id: "agent-1".to_string(),
            _config: TransportConfig::default(),
            handshake_done: std::sync::atomic::AtomicBool::new(false),
        };

        let error = client
            .send_event(EventType::PreAction, serde_json::json!({}))
            .await
            .expect_err("event should fail before handshake");

        assert!(error.to_string().contains("Handshake must complete"));
    }

    #[tokio::test]
    async fn send_batch_rejects_decision_count_mismatch() {
        let transport = Arc::new(StaticTransport {
            response: AhpResponse::success(
                "placeholder",
                serde_json::to_value(BatchResponse { decisions: vec![] }).unwrap(),
            ),
            echo_request_id: true,
        });
        let client = AhpClient::new_for_testing(transport);
        let event = AhpEvent {
            event_type: EventType::PreAction,
            session_id: "session-1".to_string(),
            agent_id: "agent-1".to_string(),
            timestamp: "2026-05-01T00:00:00Z".to_string(),
            depth: 0,
            payload: serde_json::json!({}),
            context: None,
            metadata: None,
        };

        let error = client
            .send_batch(vec![event])
            .await
            .expect_err("batch decision count mismatch should fail");

        assert!(error.to_string().contains("decision count mismatch"));
    }

    #[tokio::test]
    async fn send_batch_rejects_specialized_decision_events() {
        let transport = Arc::new(StaticTransport {
            response: AhpResponse::success(
                "placeholder",
                serde_json::to_value(BatchResponse { decisions: vec![] }).unwrap(),
            ),
            echo_request_id: true,
        });
        let client = AhpClient::new_for_testing(transport);
        let event = AhpEvent {
            event_type: EventType::ContextPerception,
            session_id: "session-1".to_string(),
            agent_id: "agent-1".to_string(),
            timestamp: "2026-05-01T00:00:00Z".to_string(),
            depth: 0,
            payload: serde_json::json!({}),
            context: None,
            metadata: None,
        };

        let error = client
            .send_batch(vec![event])
            .await
            .expect_err("specialized decision event should not be batchable");

        assert!(error.to_string().contains("cannot be batched"));
    }
}
