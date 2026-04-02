//! AHP server implementation

use crate::protocol::{EventContext, HarnessConfig, HarnessInfo, IdleDecision, IdleEvent};
use crate::{
    AhpError, AhpEvent, AhpNotification, AhpRequest, AhpResponse, BatchRequest, BatchResponse,
    Decision, EventType, HandshakeRequest, HandshakeResponse, QueryRequest, QueryResponse, Result,
    PROTOCOL_VERSION,
};
use async_trait::async_trait;
use std::sync::Arc;

/// Event handler trait - implement this to handle events
#[async_trait]
pub trait EventHandler: Send + Sync {
    /// Handle a blocking event and return a decision
    async fn handle_event(&self, event: &AhpEvent) -> Result<Decision>;

    /// Handle a notification (fire-and-forget)
    async fn handle_notification(&self, event: &AhpEvent) -> Result<()> {
        // Default: do nothing
        let _ = event;
        Ok(())
    }

    /// Handle a query from the agent
    async fn handle_query(&self, query: &QueryRequest) -> Result<QueryResponse> {
        // Default: not supported
        let _ = query;
        Err(AhpError::UnsupportedCapability(
            "Query not supported".to_string(),
        ))
    }

    /// Handle an idle event - called when agent has been idle for threshold duration
    ///
    /// This enables background consolidation (dream system), memory cleanup, etc.
    /// Default: allow idle processing
    async fn handle_idle(
        &self,
        idle_event: &IdleEvent,
        context: &EventContext,
    ) -> Result<IdleDecision> {
        let _ = (idle_event, context);
        Ok(IdleDecision::Allow)
    }
}

/// AHP server - receives events from agents
pub struct AhpServer {
    handler: Arc<dyn EventHandler>,
    harness_info: HarnessInfo,
    config: HarnessConfig,
}

impl AhpServer {
    /// Create a new AHP server with the specified event handler
    pub fn new(handler: Arc<dyn EventHandler>) -> Self {
        Self {
            handler,
            harness_info: HarnessInfo {
                name: "ahp-server".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                capabilities: vec![
                    "pre_action".to_string(),
                    "post_action".to_string(),
                    "pre_prompt".to_string(),
                    "query".to_string(),
                    "batch".to_string(),
                ],
            },
            config: HarnessConfig {
                timeout_ms: Some(10_000),
                batch_size: Some(100),
                max_depth: Some(10),
            },
        }
    }

    /// Handle a JSON-RPC request
    pub async fn handle_request(&self, request: AhpRequest) -> AhpResponse {
        match request.method.as_str() {
            "ahp/handshake" => self.handle_handshake(request).await,
            "ahp/event" => self.handle_event_request(request).await,
            "ahp/query" => self.handle_query_request(request).await,
            "ahp/batch" => self.handle_batch_request(request).await,
            _ => AhpResponse::error(request.id, -32601, "Method not found"),
        }
    }

    /// Handle a JSON-RPC notification
    pub async fn handle_notification(&self, notification: AhpNotification) -> Result<()> {
        match notification.method.as_str() {
            "ahp/event" => {
                let event: AhpEvent = serde_json::from_value(notification.params)?;
                self.handler.handle_notification(&event).await?;
                Ok(())
            }
            _ => Ok(()), // Ignore unknown notifications
        }
    }

    async fn handle_handshake(&self, request: AhpRequest) -> AhpResponse {
        match serde_json::from_value::<HandshakeRequest>(request.params) {
            Ok(_handshake_req) => {
                let response = HandshakeResponse {
                    protocol_version: PROTOCOL_VERSION.to_string(),
                    harness_info: self.harness_info.clone(),
                    session_token: None,
                    config: Some(self.config.clone()),
                };

                match serde_json::to_value(&response) {
                    Ok(value) => AhpResponse::success(request.id, value),
                    Err(e) => {
                        AhpResponse::error(request.id, -32603, format!("Internal error: {}", e))
                    }
                }
            }
            Err(e) => AhpResponse::error(request.id, -32602, format!("Invalid params: {}", e)),
        }
    }

    async fn handle_event_request(&self, request: AhpRequest) -> AhpResponse {
        match serde_json::from_value::<AhpEvent>(request.params) {
            Ok(event) => match self.handler.handle_event(&event).await {
                Ok(decision) => match serde_json::to_value(&decision) {
                    Ok(value) => AhpResponse::success(request.id, value),
                    Err(e) => {
                        AhpResponse::error(request.id, -32603, format!("Internal error: {}", e))
                    }
                },
                Err(e) => AhpResponse::error(request.id, -32603, format!("Handler error: {}", e)),
            },
            Err(e) => AhpResponse::error(request.id, -32602, format!("Invalid params: {}", e)),
        }
    }

    async fn handle_query_request(&self, request: AhpRequest) -> AhpResponse {
        match serde_json::from_value::<QueryRequest>(request.params) {
            Ok(query) => match self.handler.handle_query(&query).await {
                Ok(response) => match serde_json::to_value(&response) {
                    Ok(value) => AhpResponse::success(request.id, value),
                    Err(e) => {
                        AhpResponse::error(request.id, -32603, format!("Internal error: {}", e))
                    }
                },
                Err(e) => AhpResponse::error(request.id, -32603, format!("Handler error: {}", e)),
            },
            Err(e) => AhpResponse::error(request.id, -32602, format!("Invalid params: {}", e)),
        }
    }

    async fn handle_batch_request(&self, request: AhpRequest) -> AhpResponse {
        match serde_json::from_value::<BatchRequest>(request.params) {
            Ok(batch) => {
                let mut decisions = Vec::new();

                for event in batch.events {
                    match self.handler.handle_event(&event).await {
                        Ok(decision) => decisions.push(decision),
                        Err(_) => decisions.push(Decision::Allow {
                            modified_payload: None,
                            metadata: None,
                        }),
                    }
                }

                let response = BatchResponse { decisions };

                match serde_json::to_value(&response) {
                    Ok(value) => AhpResponse::success(request.id, value),
                    Err(e) => {
                        AhpResponse::error(request.id, -32603, format!("Internal error: {}", e))
                    }
                }
            }
            Err(e) => AhpResponse::error(request.id, -32602, format!("Invalid params: {}", e)),
        }
    }

    /// Run the server with stdio transport (read from stdin, write to stdout)
    pub async fn run_stdio(&self) -> Result<()> {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

        let stdin = tokio::io::stdin();
        let mut stdout = tokio::io::stdout();
        let mut reader = BufReader::new(stdin);
        let mut line = String::new();

        loop {
            line.clear();
            match reader.read_line(&mut line).await {
                Ok(0) => break, // EOF
                Ok(_) => {
                    // Try to parse as request or notification
                    if let Ok(request) = serde_json::from_str::<AhpRequest>(&line) {
                        let response = self.handle_request(request).await;
                        let json = serde_json::to_string(&response)?;
                        stdout.write_all(json.as_bytes()).await?;
                        stdout.write_all(b"\n").await?;
                        stdout.flush().await?;
                    } else if let Ok(notification) = serde_json::from_str::<AhpNotification>(&line)
                    {
                        let _ = self.handle_notification(notification).await;
                    }
                }
                Err(_) => break,
            }
        }

        Ok(())
    }
}
