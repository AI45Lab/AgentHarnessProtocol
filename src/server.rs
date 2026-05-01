//! AHP server implementation

use crate::protocol::{
    ConfirmationDecision, ConfirmationEvent, ContextPerceptionDecision, ContextPerceptionEvent,
    EventContext, HarnessConfig, HarnessInfo, IdleDecision, IdleEvent, IntentDetectionDecision,
    IntentDetectionEvent, MemoryRecallDecision, MemoryRecallEvent, PlanningDecision, PlanningEvent,
    RateLimitDecision, RateLimitEvent, ReasoningDecision, ReasoningEvent,
};
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

    /// Handle context perception with a typed payload.
    async fn handle_context_perception(
        &self,
        event: &AhpEvent,
        payload: &ContextPerceptionEvent,
    ) -> Result<ContextPerceptionDecision> {
        let _ = (event, payload);
        Err(AhpError::UnsupportedCapability(
            "Context perception not supported".to_string(),
        ))
    }

    /// Handle memory recall with a typed payload.
    async fn handle_memory_recall(
        &self,
        event: &AhpEvent,
        payload: &MemoryRecallEvent,
    ) -> Result<MemoryRecallDecision> {
        let _ = (event, payload);
        Err(AhpError::UnsupportedCapability(
            "Memory recall not supported".to_string(),
        ))
    }

    /// Handle planning with a typed payload.
    async fn handle_planning(
        &self,
        event: &AhpEvent,
        payload: &PlanningEvent,
    ) -> Result<PlanningDecision> {
        let _ = (event, payload);
        Err(AhpError::UnsupportedCapability(
            "Planning not supported".to_string(),
        ))
    }

    /// Handle reasoning with a typed payload.
    async fn handle_reasoning(
        &self,
        event: &AhpEvent,
        payload: &ReasoningEvent,
    ) -> Result<ReasoningDecision> {
        let _ = (event, payload);
        Err(AhpError::UnsupportedCapability(
            "Reasoning not supported".to_string(),
        ))
    }

    /// Handle rate limits with a typed payload.
    async fn handle_rate_limit(
        &self,
        event: &AhpEvent,
        payload: &RateLimitEvent,
    ) -> Result<RateLimitDecision> {
        let _ = (event, payload);
        Err(AhpError::UnsupportedCapability(
            "Rate limit decisions not supported".to_string(),
        ))
    }

    /// Handle confirmation requests with a typed payload.
    async fn handle_confirmation(
        &self,
        event: &AhpEvent,
        payload: &ConfirmationEvent,
    ) -> Result<ConfirmationDecision> {
        let _ = (event, payload);
        Err(AhpError::UnsupportedCapability(
            "Confirmation not supported".to_string(),
        ))
    }

    /// Handle intent detection with a typed payload.
    async fn handle_intent_detection(
        &self,
        event: &AhpEvent,
        payload: &IntentDetectionEvent,
    ) -> Result<IntentDetectionDecision> {
        let _ = (event, payload);
        Err(AhpError::UnsupportedCapability(
            "Intent detection not supported".to_string(),
        ))
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
            harness_info: Self::default_harness_info(),
            config: Self::default_config(),
        }
    }

    /// Override the harness information returned during handshake.
    pub fn with_harness_info(mut self, harness_info: HarnessInfo) -> Self {
        self.harness_info = harness_info;
        self
    }

    /// Override the harness configuration returned during handshake and used by validation.
    pub fn with_config(mut self, config: HarnessConfig) -> Self {
        self.config = config;
        self
    }

    /// Override only the advertised capability list.
    pub fn with_capabilities<I, S>(mut self, capabilities: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.harness_info.capabilities = capabilities.into_iter().map(Into::into).collect();
        self
    }

    /// Return the configured harness information.
    pub fn harness_info(&self) -> &HarnessInfo {
        &self.harness_info
    }

    /// Return the configured harness limits.
    pub fn config(&self) -> &HarnessConfig {
        &self.config
    }

    fn default_harness_info() -> HarnessInfo {
        HarnessInfo {
            name: "ahp-server".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            capabilities: vec![
                "pre_action".to_string(),
                "post_action".to_string(),
                "pre_prompt".to_string(),
                "query".to_string(),
                "batch".to_string(),
            ],
        }
    }

    fn default_config() -> HarnessConfig {
        HarnessConfig {
            timeout_ms: Some(10_000),
            batch_size: Some(100),
            max_depth: Some(10),
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
                self.validate_event_shape(&event)?;

                if event.event_type.is_blocking() {
                    return Err(AhpError::Protocol(format!(
                        "Blocking event {} must be sent as a request",
                        event.event_type
                    )));
                }

                self.handler.handle_notification(&event).await?;
                Ok(())
            }
            _ => Ok(()), // Ignore unknown notifications
        }
    }

    async fn handle_handshake(&self, request: AhpRequest) -> AhpResponse {
        match serde_json::from_value::<HandshakeRequest>(request.params) {
            Ok(handshake_req) => {
                if handshake_req.protocol_version.split('.').next()
                    != PROTOCOL_VERSION.split('.').next()
                {
                    return AhpResponse::error(
                        request.id,
                        -32000,
                        format!(
                            "Unsupported protocol version: {}",
                            handshake_req.protocol_version
                        ),
                    );
                }

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
            Ok(event) => {
                if let Err(e) = self.validate_event_shape(&event) {
                    return AhpResponse::error(request.id, -32602, format!("Invalid event: {}", e));
                }

                if !event.event_type.is_blocking() {
                    return AhpResponse::error(
                        request.id,
                        -32602,
                        format!(
                            "Fire-and-forget event {} must be sent as a notification",
                            event.event_type
                        ),
                    );
                }

                match self.handle_event_payload(&event).await {
                    Ok(value) => AhpResponse::success(request.id, value),
                    Err(e) => {
                        AhpResponse::error(request.id, -32603, format!("Handler error: {}", e))
                    }
                }
            }
            Err(e) => AhpResponse::error(request.id, -32602, format!("Invalid params: {}", e)),
        }
    }

    fn validate_event_shape(&self, event: &AhpEvent) -> Result<()> {
        if let Some(max_depth) = self.config.max_depth {
            if event.depth > max_depth {
                return Err(AhpError::Protocol(format!(
                    "Event depth {} exceeds configured max depth {}",
                    event.depth, max_depth
                )));
            }
        }

        Ok(())
    }

    async fn handle_event_payload(&self, event: &AhpEvent) -> Result<serde_json::Value> {
        match event.event_type {
            EventType::Idle => {
                let idle_event: IdleEvent = serde_json::from_value(event.payload.clone())?;
                let context = event.context.clone().unwrap_or_default();
                serde_json::to_value(self.handler.handle_idle(&idle_event, &context).await?)
                    .map_err(AhpError::from)
            }
            EventType::ContextPerception => {
                let payload: ContextPerceptionEvent =
                    serde_json::from_value(event.payload.clone())?;
                serde_json::to_value(
                    self.handler
                        .handle_context_perception(event, &payload)
                        .await?,
                )
                .map_err(AhpError::from)
            }
            EventType::MemoryRecall => {
                let payload: MemoryRecallEvent = serde_json::from_value(event.payload.clone())?;
                serde_json::to_value(self.handler.handle_memory_recall(event, &payload).await?)
                    .map_err(AhpError::from)
            }
            EventType::Planning => {
                let payload: PlanningEvent = serde_json::from_value(event.payload.clone())?;
                serde_json::to_value(self.handler.handle_planning(event, &payload).await?)
                    .map_err(AhpError::from)
            }
            EventType::Reasoning => {
                let payload: ReasoningEvent = serde_json::from_value(event.payload.clone())?;
                serde_json::to_value(self.handler.handle_reasoning(event, &payload).await?)
                    .map_err(AhpError::from)
            }
            EventType::RateLimit => {
                let payload: RateLimitEvent = serde_json::from_value(event.payload.clone())?;
                serde_json::to_value(self.handler.handle_rate_limit(event, &payload).await?)
                    .map_err(AhpError::from)
            }
            EventType::Confirmation => {
                let payload: ConfirmationEvent = serde_json::from_value(event.payload.clone())?;
                serde_json::to_value(self.handler.handle_confirmation(event, &payload).await?)
                    .map_err(AhpError::from)
            }
            EventType::IntentDetection => {
                let payload: IntentDetectionEvent = serde_json::from_value(event.payload.clone())?;
                serde_json::to_value(
                    self.handler
                        .handle_intent_detection(event, &payload)
                        .await?,
                )
                .map_err(AhpError::from)
            }
            _ => serde_json::to_value(self.handler.handle_event(event).await?)
                .map_err(AhpError::from),
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
                if let Some(max_batch_size) = self.config.batch_size {
                    if batch.events.len() > max_batch_size {
                        return AhpResponse::error(
                            request.id,
                            -32602,
                            format!(
                                "Batch size {} exceeds configured limit {}",
                                batch.events.len(),
                                max_batch_size
                            ),
                        );
                    }
                }

                let mut decisions = Vec::with_capacity(batch.events.len());

                for event in batch.events {
                    if let Err(e) = self.validate_event_shape(&event) {
                        return AhpResponse::error(
                            request.id,
                            -32602,
                            format!("Invalid event: {}", e),
                        );
                    }

                    if !event.event_type.is_batchable() {
                        return AhpResponse::error(
                            request.id,
                            -32602,
                            format!(
                                "Event type {} cannot be batched because it does not return a generic Decision",
                                event.event_type
                            ),
                        );
                    }

                    match self.handler.handle_event(&event).await {
                        Ok(decision) => decisions.push(decision),
                        Err(e) => decisions.push(Decision::Block {
                            reason: format!("Handler error: {}", e),
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

#[cfg(test)]
mod tests;
