use super::*;
use async_trait::async_trait;
use std::sync::Arc;

struct FailingHandler;

#[async_trait]
impl EventHandler for FailingHandler {
    async fn handle_event(&self, _event: &AhpEvent) -> Result<Decision> {
        Err(AhpError::Protocol("policy engine unavailable".to_string()))
    }
}

struct ContextHandler;

#[async_trait]
impl EventHandler for ContextHandler {
    async fn handle_event(&self, _event: &AhpEvent) -> Result<Decision> {
        Err(AhpError::UnsupportedCapability(
            "generic event handling not supported".to_string(),
        ))
    }

    async fn handle_context_perception(
        &self,
        _event: &AhpEvent,
        payload: &ContextPerceptionEvent,
    ) -> Result<ContextPerceptionDecision> {
        Ok(ContextPerceptionDecision::Allow {
            injected_context: crate::InjectedContext {
                facts: vec![crate::Fact {
                    content: format!("context for {}", payload.context.workspace),
                    source: "typed-handler".to_string(),
                    confidence: 1.0,
                }],
                file_contents: None,
                project_summary: None,
                knowledge: None,
                suggestions: None,
            },
            metadata: None,
        })
    }
}

fn test_event() -> AhpEvent {
    AhpEvent {
        event_type: EventType::PreAction,
        session_id: "session-1".to_string(),
        agent_id: "agent-1".to_string(),
        timestamp: "2026-05-01T00:00:00Z".to_string(),
        depth: 0,
        payload: serde_json::json!({ "tool_name": "bash" }),
        context: None,
        metadata: None,
    }
}

fn request(method: &str, params: serde_json::Value) -> AhpRequest {
    AhpRequest {
        jsonrpc: "2.0".to_string(),
        id: "request-1".to_string(),
        method: method.to_string(),
        params,
    }
}

fn context_perception_event() -> AhpEvent {
    let payload = crate::ContextPerceptionEvent {
        session_id: "session-1".to_string(),
        intent: crate::PerceptionIntent::Understand,
        target: crate::PerceptionTarget::Location {
            path: ".".to_string(),
            location_type: "workspace".to_string(),
        },
        domain: crate::PerceptionDomain::Coding,
        preferred_modality: crate::PerceptionModality::Code,
        urgency: crate::PerceptionUrgency::Normal,
        freshness: crate::PerceptionFreshness::Static,
        context: crate::PerceptionContext {
            workspace: "/workspace".to_string(),
            current_task: Some("inspect architecture".to_string()),
            query: Some("server design".to_string()),
            relevant_history: None,
        },
        constraints: crate::PerceptionConstraints::default(),
        metadata: None,
    };

    AhpEvent {
        event_type: EventType::ContextPerception,
        session_id: "session-1".to_string(),
        agent_id: "agent-1".to_string(),
        timestamp: "2026-05-01T00:00:00Z".to_string(),
        depth: 0,
        payload: serde_json::to_value(payload).expect("context event serializes"),
        context: None,
        metadata: None,
    }
}

#[test]
fn server_builder_overrides_harness_info_and_config() {
    let harness_info = HarnessInfo {
        name: "custom-harness".to_string(),
        version: "1.0.0".to_string(),
        capabilities: vec!["pre_action".to_string()],
    };

    let server = AhpServer::new(Arc::new(FailingHandler))
        .with_harness_info(harness_info)
        .with_capabilities(["pre_action", "custom"])
        .with_config(HarnessConfig {
            timeout_ms: Some(2500),
            batch_size: Some(2),
            max_depth: Some(1),
        });

    assert_eq!(server.harness_info().name, "custom-harness");
    assert_eq!(
        server.harness_info().capabilities,
        vec!["pre_action".to_string(), "custom".to_string()]
    );
    assert_eq!(server.config().timeout_ms, Some(2500));
    assert_eq!(server.config().batch_size, Some(2));
    assert_eq!(server.config().max_depth, Some(1));
}

#[tokio::test]
async fn batch_handler_errors_fail_closed() {
    let server = AhpServer::new(Arc::new(FailingHandler));
    let response = server
        .handle_request(request(
            "ahp/batch",
            serde_json::to_value(BatchRequest {
                events: vec![test_event()],
            })
            .unwrap(),
        ))
        .await;

    let result = response.result.expect("batch response has result");
    let batch: BatchResponse = serde_json::from_value(result).unwrap();

    assert_eq!(batch.decisions.len(), 1);
    match &batch.decisions[0] {
        Decision::Block { reason, metadata } => {
            assert!(reason.contains("policy engine unavailable"));
            assert!(metadata.is_none());
        }
        decision => panic!("expected fail-closed block decision, got {decision:?}"),
    }
}

#[tokio::test]
async fn context_perception_uses_typed_handler() {
    let server = AhpServer::new(Arc::new(ContextHandler));
    let response = server
        .handle_request(request(
            "ahp/event",
            serde_json::to_value(context_perception_event()).unwrap(),
        ))
        .await;

    let result = response.result.expect("context perception returns result");
    let decision: ContextPerceptionDecision = serde_json::from_value(result).unwrap();

    match decision {
        ContextPerceptionDecision::Allow {
            injected_context, ..
        } => {
            assert_eq!(injected_context.facts.len(), 1);
            assert_eq!(injected_context.facts[0].content, "context for /workspace");
            assert_eq!(injected_context.facts[0].source, "typed-handler");
        }
        decision => panic!("expected typed context allow decision, got {decision:?}"),
    }
}

#[tokio::test]
async fn batch_rejects_specialized_decision_events() {
    let server = AhpServer::new(Arc::new(ContextHandler));
    let response = server
        .handle_request(request(
            "ahp/batch",
            serde_json::to_value(BatchRequest {
                events: vec![context_perception_event()],
            })
            .unwrap(),
        ))
        .await;

    let error = response.error.expect("batch should reject typed event");
    assert_eq!(error.code, -32602);
    assert!(error.message.contains("cannot be batched"));
}

#[tokio::test]
async fn request_rejects_fire_and_forget_events() {
    let server = AhpServer::new(Arc::new(FailingHandler));
    let mut event = test_event();
    event.event_type = EventType::PostAction;

    let response = server
        .handle_request(request("ahp/event", serde_json::to_value(event).unwrap()))
        .await;

    let error = response
        .error
        .expect("request should reject notification event");
    assert_eq!(error.code, -32602);
    assert!(error.message.contains("must be sent as a notification"));
}

#[tokio::test]
async fn notification_rejects_blocking_events() {
    let server = AhpServer::new(Arc::new(FailingHandler));
    let result = server
        .handle_notification(AhpNotification::new(
            "ahp/event",
            serde_json::to_value(test_event()).unwrap(),
        ))
        .await;

    let error = result.expect_err("notification should reject blocking event");
    assert!(error.to_string().contains("must be sent as a request"));
}

#[tokio::test]
async fn request_rejects_events_over_max_depth() {
    let server = AhpServer::new(Arc::new(FailingHandler)).with_config(HarnessConfig {
        timeout_ms: Some(10_000),
        batch_size: Some(100),
        max_depth: Some(1),
    });
    let mut event = test_event();
    event.depth = 2;

    let response = server
        .handle_request(request("ahp/event", serde_json::to_value(event).unwrap()))
        .await;

    let error = response.error.expect("request should reject deep event");
    assert_eq!(error.code, -32602);
    assert!(error.message.contains("exceeds configured max depth 1"));
}

#[tokio::test]
async fn batch_rejects_events_over_max_depth() {
    let server = AhpServer::new(Arc::new(FailingHandler)).with_config(HarnessConfig {
        timeout_ms: Some(10_000),
        batch_size: Some(100),
        max_depth: Some(1),
    });
    let mut event = test_event();
    event.depth = 2;

    let response = server
        .handle_request(request(
            "ahp/batch",
            serde_json::to_value(BatchRequest {
                events: vec![event],
            })
            .unwrap(),
        ))
        .await;

    let error = response.error.expect("batch should reject deep event");
    assert_eq!(error.code, -32602);
    assert!(error.message.contains("exceeds configured max depth 1"));
}
