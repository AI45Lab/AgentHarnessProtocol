//! AHP JSON-RPC 2.0 protocol types.
//!
//! The Agent Harness Protocol uses newline-delimited JSON-RPC 2.0 messages
//! over stdio. Blocking events (`pre_tool_use`, `pre_prompt`) are sent as
//! requests with an `id`; all other events are sent as notifications (no `id`).
//!
//! ## Wire format
//!
//! ```json
//! {
//!   "jsonrpc": "2.0",
//!   "id": 1,
//!   "method": "harness/event",
//!   "params": {
//!     "event_type": "pre_tool_use",
//!     "payload": { ... },
//!     "meta": { "depth": 2 }
//!   }
//! }
//! ```
//!
//! `meta.depth` is the sub-agent nesting depth:
//! `0` = user session, `1` = first-level sub-agent, etc.

use a3s_code_core::hooks::HookEvent;
use serde::{Deserialize, Serialize};

// ============================================================================
// Meta
// ============================================================================

/// Per-message metadata injected by the AHP host into every request/notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AhpMeta {
    /// Sub-agent nesting depth.
    ///
    /// `0` = user-facing session, `1` = first-level sub-agent, etc.
    /// The harness server can use this to apply stricter policies for deeply
    /// nested agents, or to track session lineage across the call tree.
    pub depth: u32,
}

impl AhpMeta {
    pub fn new(depth: u32) -> Self {
        Self { depth }
    }
}

// ============================================================================
// Params construction
// ============================================================================

/// Build the `params` object for an AHP message.
///
/// Merges the serialized `HookEvent` (which produces `event_type` + `payload`
/// fields) with the `meta` object into a single flat JSON object.
///
/// Returns `None` only if serialisation of the event itself fails.
pub fn build_params(event: &HookEvent, meta: &AhpMeta) -> Option<serde_json::Value> {
    let mut params = serde_json::to_value(event).ok()?;
    if let serde_json::Value::Object(ref mut map) = params {
        map.insert(
            "meta".to_string(),
            serde_json::to_value(meta).unwrap_or(serde_json::Value::Null),
        );
    }
    Some(params)
}

// ============================================================================
// Outgoing messages (host → server)
// ============================================================================

/// JSON-RPC request — sent for blocking events that require a decision.
#[derive(Serialize)]
pub struct AhpRequest {
    pub jsonrpc: &'static str,
    pub id: u64,
    pub method: &'static str,
    pub params: serde_json::Value,
}

impl AhpRequest {
    pub fn new(id: u64, params: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            method: "harness/event",
            params,
        }
    }
}

/// JSON-RPC notification — sent for fire-and-forget events.
#[derive(Serialize)]
pub struct AhpNotification {
    pub jsonrpc: &'static str,
    pub method: &'static str,
    pub params: serde_json::Value,
}

impl AhpNotification {
    pub fn new(params: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0",
            method: "harness/event",
            params,
        }
    }
}

// ============================================================================
// Incoming messages (server → host)
// ============================================================================

/// Action decided by the harness server.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AhpAction {
    Continue,
    Block,
    Skip,
    Retry,
}

/// Decision returned by the harness server for blocking events.
#[derive(Debug, Clone, Deserialize)]
pub struct AhpDecision {
    pub action: AhpAction,
    pub reason: Option<String>,
    pub modified: Option<serde_json::Value>,
    pub retry_delay_ms: Option<u64>,
}

impl Default for AhpDecision {
    fn default() -> Self {
        Self {
            action: AhpAction::Continue,
            reason: None,
            modified: None,
            retry_delay_ms: None,
        }
    }
}

/// Incoming JSON-RPC response from the harness server.
#[derive(Deserialize)]
pub struct AhpResponse {
    pub id: u64,
    pub result: AhpDecision,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use a3s_code_core::hooks::{HookEvent, PreToolUseEvent};

    fn sample_event() -> HookEvent {
        HookEvent::PreToolUse(PreToolUseEvent {
            session_id: "s1".into(),
            tool: "Bash".into(),
            args: serde_json::json!({"command": "ls"}),
            working_directory: "/workspace".into(),
            recent_tools: vec![],
        })
    }

    #[test]
    fn test_build_params_contains_event_fields() {
        let event = sample_event();
        let meta = AhpMeta::new(0);
        let params = build_params(&event, &meta).unwrap();
        assert_eq!(params["event_type"], "pre_tool_use");
        assert!(params["payload"].is_object());
    }

    #[test]
    fn test_build_params_contains_meta() {
        let event = sample_event();
        let meta = AhpMeta::new(3);
        let params = build_params(&event, &meta).unwrap();
        assert_eq!(params["meta"]["depth"], 3);
    }

    #[test]
    fn test_request_serialization() {
        let event = sample_event();
        let meta = AhpMeta::new(1);
        let params = build_params(&event, &meta).unwrap();
        let req = AhpRequest::new(42, params);
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"id\":42"));
        assert!(json.contains("\"method\":\"harness/event\""));
        assert!(json.contains("pre_tool_use"));
        assert!(json.contains("\"depth\":1"));
    }

    #[test]
    fn test_notification_has_no_id() {
        let event = sample_event();
        let meta = AhpMeta::new(0);
        let params = build_params(&event, &meta).unwrap();
        let notif = AhpNotification::new(params);
        let json = serde_json::to_string(&notif).unwrap();
        assert!(!json.contains("\"id\""));
        assert!(json.contains("\"method\":\"harness/event\""));
        assert!(json.contains("\"depth\":0"));
    }

    #[test]
    fn test_decision_default_is_continue() {
        let d = AhpDecision::default();
        assert_eq!(d.action, AhpAction::Continue);
        assert!(d.reason.is_none());
        assert!(d.modified.is_none());
    }

    #[test]
    fn test_ahp_response_deserialization() {
        let raw = r#"{"jsonrpc":"2.0","id":1,"result":{"action":"block","reason":"dangerous","modified":null,"retry_delay_ms":null}}"#;
        let resp: AhpResponse = serde_json::from_str(raw).unwrap();
        assert_eq!(resp.id, 1);
        assert_eq!(resp.result.action, AhpAction::Block);
        assert_eq!(resp.result.reason.as_deref(), Some("dangerous"));
    }
}
