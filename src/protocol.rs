//! AHP protocol message definitions

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// JSON-RPC 2.0 request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AhpRequest {
    pub jsonrpc: String,
    pub id: String,
    pub method: String,
    pub params: serde_json::Value,
}

/// JSON-RPC 2.0 response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AhpResponse {
    pub jsonrpc: String,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<AhpErrorObject>,
}

/// JSON-RPC 2.0 notification (no id, no response expected)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AhpNotification {
    pub jsonrpc: String,
    pub method: String,
    pub params: serde_json::Value,
}

/// JSON-RPC 2.0 error object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AhpErrorObject {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// AHP event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AhpEvent {
    pub event_type: EventType,
    pub session_id: String,
    pub agent_id: String,
    pub timestamp: String,
    pub depth: u32,
    pub payload: serde_json::Value,
    /// Structured context for context-aware decisions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<EventContext>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Event types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    Handshake,
    PreAction,
    PostAction,
    PrePrompt,
    PostResponse,
    SessionStart,
    SessionEnd,
    Error,
    Query,
    Heartbeat,
    Idle,
}

impl EventType {
    /// Returns true if this event type requires a response (blocking)
    pub fn is_blocking(&self) -> bool {
        matches!(
            self,
            EventType::Handshake | EventType::PreAction | EventType::PrePrompt | EventType::Query
        )
    }
}

impl std::fmt::Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventType::Handshake => write!(f, "handshake"),
            EventType::PreAction => write!(f, "pre_action"),
            EventType::PostAction => write!(f, "post_action"),
            EventType::PrePrompt => write!(f, "pre_prompt"),
            EventType::PostResponse => write!(f, "post_response"),
            EventType::SessionStart => write!(f, "session_start"),
            EventType::SessionEnd => write!(f, "session_end"),
            EventType::Error => write!(f, "error"),
            EventType::Query => write!(f, "query"),
            EventType::Heartbeat => write!(f, "heartbeat"),
            EventType::Idle => write!(f, "idle"),
        }
    }
}

/// Decision types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "lowercase")]
pub enum Decision {
    Allow {
        #[serde(skip_serializing_if = "Option::is_none")]
        modified_payload: Option<serde_json::Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        metadata: Option<HashMap<String, serde_json::Value>>,
    },
    Block {
        reason: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        metadata: Option<HashMap<String, serde_json::Value>>,
    },
    Modify {
        modified_payload: serde_json::Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        metadata: Option<HashMap<String, serde_json::Value>>,
    },
    Defer {
        retry_after_ms: u64,
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
    Escalate {
        reason: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        escalation_target: Option<String>,
    },
}

/// Handshake request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeRequest {
    pub protocol_version: String,
    pub agent_info: AgentInfo,
    pub session_id: String,
    pub agent_id: String,
}

/// Agent information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub framework: String,
    pub version: String,
    pub capabilities: Vec<String>,
}

/// Handshake response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeResponse {
    pub protocol_version: String,
    pub harness_info: HarnessInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<HarnessConfig>,
}

/// Harness information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessInfo {
    pub name: String,
    pub version: String,
    pub capabilities: Vec<String>,
}

/// Harness configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_size: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_depth: Option<u32>,
}

/// Query request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryRequest {
    pub session_id: String,
    pub agent_id: String,
    pub query_type: String,
    pub payload: serde_json::Value,
}

/// Query response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResponse {
    pub answer: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alternatives: Option<Vec<String>>,
}

/// Batch request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchRequest {
    pub events: Vec<AhpEvent>,
}

/// Batch response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResponse {
    pub decisions: Vec<Decision>,
}

impl AhpRequest {
    pub fn new(method: impl Into<String>, params: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: uuid::Uuid::new_v4().to_string(),
            method: method.into(),
            params,
        }
    }
}

impl AhpNotification {
    pub fn new(method: impl Into<String>, params: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method: method.into(),
            params,
        }
    }
}

impl AhpResponse {
    pub fn success(id: impl Into<String>, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: id.into(),
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: impl Into<String>, code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: id.into(),
            result: None,
            error: Some(AhpErrorObject {
                code,
                message: message.into(),
                data: None,
            }),
        }
    }
}

// ============================================================================
// Idle & Heartbeat Event Types
// ============================================================================

/// Idle event - fired when the agent has been idle for a threshold duration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdleEvent {
    /// Duration of idle time in milliseconds
    pub idle_duration_ms: u64,
    /// Reason for becoming idle
    pub idle_reason: String,
    /// Last event type before becoming idle
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_event_type: Option<String>,
    /// Suggested action for the idle period (e.g., "dream", "consolidate")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_action: Option<String>,
}

/// Heartbeat event - periodic status update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatEvent {
    /// Agent uptime in milliseconds
    pub uptime_ms: u64,
    /// Total events processed since start
    pub total_events_processed: u64,
    /// Current agent state
    pub current_state: String,
}

// ============================================================================
// Event Context (rich context for control decisions)
// ============================================================================

/// Structured context passed with events for context-aware control decisions.
///
/// The client自主 (client) exposes capabilities it chooses - the server can use
/// any capabilities that are exposed. Unrecognized or unneeded capabilities
/// should be ignored.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventContext {
    /// Recent facts or knowledge retrieved
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recent_facts: Option<Vec<Fact>>,
    /// Memory/knowledge base state summary
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_summary: Option<MemorySummary>,
    /// Session statistics
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_stats: Option<SessionStats>,
    /// Current task/goal description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_task: Option<String>,
    /// Client自主 exposes its own capabilities as arbitrary key-value pairs.
    /// Servers can use these to interact with the agent (e.g., memory search,
    /// session control, cross-session queries).
    ///
    /// Example capabilities:
    /// - `memory_search`: URL or endpoint for searching memory
    /// - `session_control`: URL or endpoint for controlling the session
    /// - `cross_session`: URL or endpoint for cross-session queries
    /// - `custom`: Any client-specific capabilities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<std::collections::HashMap<String, serde_json::Value>>,
}

/// A factual memory item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fact {
    pub content: String,
    pub source: String,
    pub confidence: f32,
}

/// Memory state summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySummary {
    pub memory_type: String,
    pub total_items: usize,
    pub recent_topics: Vec<String>,
}

/// Session statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStats {
    pub total_actions: usize,
    pub total_tokens: i32,
    pub duration_ms: u64,
    pub error_count: usize,
}

// ============================================================================
// Idle Decision (response to idle events)
// ============================================================================

/// Decision for idle/dream events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "lowercase")]
pub enum IdleDecision {
    /// Allow background consolidation/dream
    Allow,
    /// Defer idle processing
    Defer {
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
}
