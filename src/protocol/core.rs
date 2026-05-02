use super::EventContext;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// AHP event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AhpEvent {
    pub event_type: EventType,
    pub session_id: String,
    pub agent_id: String,
    pub timestamp: String,
    pub depth: u32,
    pub payload: serde_json::Value,
    /// Structured context for context-aware decisions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<EventContext>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Event types.
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
    /// Agent is idle and asks whether background work should run.
    Idle,
    /// Context perception - model needs workspace knowledge (blocking).
    ContextPerception,
    /// Operation succeeded (fire-and-forget).
    Success,
    /// Memory recall - model needs to retrieve from memory (blocking).
    MemoryRecall,
    /// Task planning/decomposition (blocking).
    Planning,
    /// Chain-of-thought reasoning (blocking).
    Reasoning,
    /// Rate limit triggered and requires backpressure decision.
    RateLimit,
    /// User confirmation needed.
    Confirmation,
    /// Intent detection - detect user intent from prompt (blocking).
    IntentDetection,
    /// Durable run lifecycle transition (fire-and-forget).
    RunLifecycle,
    /// Authoritative task-list snapshot for a run (fire-and-forget).
    TaskList,
    /// Verification status snapshot for a run (fire-and-forget).
    Verification,
}

impl EventType {
    /// Returns true if this event type requires a response.
    pub fn is_blocking(&self) -> bool {
        matches!(
            self,
            EventType::Handshake
                | EventType::PreAction
                | EventType::PrePrompt
                | EventType::Query
                | EventType::Idle
                | EventType::ContextPerception
                | EventType::MemoryRecall
                | EventType::Planning
                | EventType::Reasoning
                | EventType::RateLimit
                | EventType::Confirmation
                | EventType::IntentDetection
        )
    }

    /// Returns true if this event returns a specialized decision type.
    pub fn uses_specialized_decision(&self) -> bool {
        matches!(
            self,
            EventType::Idle
                | EventType::ContextPerception
                | EventType::MemoryRecall
                | EventType::Planning
                | EventType::Reasoning
                | EventType::RateLimit
                | EventType::Confirmation
                | EventType::IntentDetection
        )
    }

    /// Returns true if this event can be included in a `BatchRequest`.
    pub fn is_batchable(&self) -> bool {
        !matches!(self, EventType::Handshake | EventType::Query)
            && !self.uses_specialized_decision()
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
            EventType::ContextPerception => write!(f, "context_perception"),
            EventType::Success => write!(f, "success"),
            EventType::MemoryRecall => write!(f, "memory_recall"),
            EventType::Planning => write!(f, "planning"),
            EventType::Reasoning => write!(f, "reasoning"),
            EventType::RateLimit => write!(f, "rate_limit"),
            EventType::Confirmation => write!(f, "confirmation"),
            EventType::IntentDetection => write!(f, "intent_detection"),
            EventType::RunLifecycle => write!(f, "run_lifecycle"),
            EventType::TaskList => write!(f, "task_list"),
            EventType::Verification => write!(f, "verification"),
        }
    }
}

/// Generic decision types used by the baseline AHP event flow.
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

/// Handshake request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeRequest {
    pub protocol_version: String,
    pub agent_info: AgentInfo,
    pub session_id: String,
    pub agent_id: String,
}

/// Agent information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub framework: String,
    pub version: String,
    pub capabilities: Vec<String>,
}

/// Handshake response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeResponse {
    pub protocol_version: String,
    pub harness_info: HarnessInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<HarnessConfig>,
}

/// Harness information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessInfo {
    pub name: String,
    pub version: String,
    pub capabilities: Vec<String>,
}

/// Harness configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_size: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_depth: Option<u32>,
}

/// Query request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryRequest {
    pub session_id: String,
    pub agent_id: String,
    pub query_type: String,
    pub payload: serde_json::Value,
}

/// Query response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResponse {
    pub answer: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alternatives: Option<Vec<String>>,
}

/// Batch request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchRequest {
    pub events: Vec<AhpEvent>,
}

/// Batch response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResponse {
    pub decisions: Vec<Decision>,
}
