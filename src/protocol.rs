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
    /// Context perception - model needs workspace knowledge (blocking)
    ContextPerception,
    /// Operation succeeded (fire-and-forget)
    Success,
    /// Memory recall - model needs to retrieve from memory (blocking)
    MemoryRecall,
    /// Task planning/decomposition (blocking)
    Planning,
    /// Chain-of-thought reasoning (blocking)
    Reasoning,
    /// Rate limit triggered
    RateLimit,
    /// User confirmation needed
    Confirmation,
    /// Intent detection - detect user intent from prompt (blocking)
    IntentDetection,
}

impl EventType {
    /// Returns true if this event type requires a response (blocking)
    pub fn is_blocking(&self) -> bool {
        matches!(
            self,
            EventType::Handshake
                | EventType::PreAction
                | EventType::PrePrompt
                | EventType::Query
                | EventType::ContextPerception
                | EventType::MemoryRecall
                | EventType::Planning
                | EventType::Reasoning
                | EventType::Confirmation
                | EventType::IntentDetection
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
            EventType::ContextPerception => write!(f, "context_perception"),
            EventType::Success => write!(f, "success"),
            EventType::MemoryRecall => write!(f, "memory_recall"),
            EventType::Planning => write!(f, "planning"),
            EventType::Reasoning => write!(f, "reasoning"),
            EventType::RateLimit => write!(f, "rate_limit"),
            EventType::Confirmation => write!(f, "confirmation"),
            EventType::IntentDetection => write!(f, "intent_detection"),
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
    /// CPU usage percentage (0-100)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_percent: Option<f32>,
    /// Memory usage in bytes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_bytes: Option<u64>,
    /// Currently active tool count
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_tools: Option<usize>,
    /// Pending actions in queue
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending_actions: Option<usize>,
    /// Queue depth
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queue_depth: Option<usize>,
    /// Tokens used in current session
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens_used: Option<i32>,
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

// ============================================================================
// Context Perception (model needs workspace knowledge)
// ============================================================================

/// Perception intent - why the model needs to perceive context
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PerceptionIntent {
    /// Recognize: identify or classify an entity
    /// "What is this?", "What category does this belong to?"
    Recognize,
    /// Understand: understand semantics, logic, or behavior
    /// "What does this mean?", "How does this work?"
    Understand,
    /// Locate: find the location or path of a resource
    /// "Where is X?", "Who can help me find Y?"
    Locate,
    /// Retrieve: recall relevant information from accumulated knowledge
    /// "Was this discussed before?", "I remember last time..."
    Retrieve,
    /// Explore: understand the overall structure of an environment or system
    /// "How is this project/system organized?", "What are the available...?"
    Explore,
    /// Reason: infer causality or logic based on existing information
    /// "Why did this happen?", "What if we change X?"
    Reason,
    /// Validate: confirm whether an assumption or state is correct
    /// "Is this correct?", "Are there any omissions?"
    Validate,
    /// Compare: compare similarities and differences between two or more things
    /// "What's the difference between X and Y?", "Which is better?"
    Compare,
    /// Track: get current status or history of a process
    /// "How far has this task progressed?", "What decisions were made before?"
    Track,
}

impl std::fmt::Display for PerceptionIntent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PerceptionIntent::Recognize => write!(f, "recognize"),
            PerceptionIntent::Understand => write!(f, "understand"),
            PerceptionIntent::Locate => write!(f, "locate"),
            PerceptionIntent::Retrieve => write!(f, "retrieve"),
            PerceptionIntent::Explore => write!(f, "explore"),
            PerceptionIntent::Reason => write!(f, "reason"),
            PerceptionIntent::Validate => write!(f, "validate"),
            PerceptionIntent::Compare => write!(f, "compare"),
            PerceptionIntent::Track => write!(f, "track"),
        }
    }
}

/// Perception target - what the model wants to perceive
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PerceptionTarget {
    /// Entity - a specific object or concept
    Entity {
        name: String,
        /// Entity type (e.g., "function", "file", "person", "document", "config", "api")
        entity_type: String,
        /// Identifier attribute
        identifier: Option<String>,
    },
    /// Location - a path or place
    Location {
        path: String,
        /// Location type (e.g., "file", "directory", "url", "endpoint", "region")
        location_type: String,
    },
    /// Event - something that happened or will happen
    Event {
        description: String,
        /// Event type (e.g., "change", "action", "decision", "error", "meeting")
        event_type: String,
        /// Related time range
        time_range: Option<TimeRange>,
    },
    /// Relation - connections between multiple entities
    Relation {
        entities: Vec<String>,
        /// Relation type (e.g., "dependency", "ownership", "sequence", "conflict")
        relation_type: String,
    },
    /// Rule - a policy, strategy, or convention
    Rule {
        name: String,
        /// Rule type (e.g., "policy", "convention", "constraint", "requirement")
        rule_type: String,
        /// Applicable scope
        scope: Option<String>,
    },
    /// State - current condition of a system or entity
    State {
        target: String,
        aspect: String,
        include_history: bool,
    },
    /// Resource - an available capability or asset
    Resource {
        name: String,
        /// Resource type (e.g., "tool", "skill", "api", "data", "personnel")
        resource_type: String,
        constraints: Option<serde_json::Value>,
    },
    /// Pattern - a recurring phenomenon or rule
    Pattern {
        pattern: String,
        /// Pattern type (e.g., "text", "regex", "structure", "behavior")
        pattern_type: String,
    },
}

/// Time range for events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub from: Option<i64>,
    pub to: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relative: Option<String>,
}

/// Perception domain - the domain of the current task
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PerceptionDomain {
    Coding,
    Writing,
    DataAnalysis,
    Research,
    ProjectManagement,
    Conversation,
    Operations,
    Security,
    General,
}

impl Default for PerceptionDomain {
    fn default() -> Self {
        PerceptionDomain::General
    }
}

/// Perception modality - the form of information needed
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PerceptionModality {
    Text,
    Code,
    StructuredData,
    Table,
    Chart,
    Image,
    Audio,
    Video,
    Any,
}

impl Default for PerceptionModality {
    fn default() -> Self {
        PerceptionModality::Any
    }
}

/// Quality requirement for perception
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PerceptionUrgency {
    /// Must have - cannot proceed without this context
    Critical,
    /// Important - significantly improves response quality
    High,
    /// Normal - helpful but not essential
    Normal,
    /// Low - can be cached or delayed
    Low,
}

impl Default for PerceptionUrgency {
    fn default() -> Self {
        PerceptionUrgency::Normal
    }
}

/// Freshness requirement for perceived data
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PerceptionFreshness {
    /// Need realtime data (e.g., git status, filesystem)
    Realtime,
    /// Need recent data (e.g., recent policy changes)
    Recent,
    /// Static data is sufficient (e.g., code structure, archives)
    Static,
}

impl Default for PerceptionFreshness {
    fn default() -> Self {
        PerceptionFreshness::Static
    }
}

/// Constraints for perception
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerceptionConstraints {
    #[serde(default)]
    pub max_results: Option<usize>,
    #[serde(default)]
    pub max_context_length: Option<usize>,
    #[serde(default = "default_include_sources")]
    pub include_sources: bool,
}

fn default_include_sources() -> bool {
    true
}

impl Default for PerceptionConstraints {
    fn default() -> Self {
        Self {
            max_results: None,
            max_context_length: None,
            include_sources: true,
        }
    }
}

/// Current perception context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerceptionContext {
    pub workspace: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_task: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relevant_history: Option<Vec<HistoryItem>>,
}

/// History item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryItem {
    pub item_type: String,
    pub content: String,
    pub timestamp: i64,
}

/// Context perception event - fired when model needs workspace knowledge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPerceptionEvent {
    pub session_id: String,
    pub intent: PerceptionIntent,
    pub target: PerceptionTarget,
    #[serde(default)]
    pub domain: PerceptionDomain,
    #[serde(default)]
    pub preferred_modality: PerceptionModality,
    #[serde(default)]
    pub urgency: PerceptionUrgency,
    #[serde(default)]
    pub freshness: PerceptionFreshness,
    pub context: PerceptionContext,
    #[serde(default)]
    pub constraints: PerceptionConstraints,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Injected context from harness
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InjectedContext {
    pub facts: Vec<Fact>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_contents: Option<Vec<FileContentSnippet>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_summary: Option<ProjectSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub knowledge: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestions: Option<Vec<String>>,
}

/// File content snippet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileContentSnippet {
    pub path: String,
    pub snippet: String,
    pub relevance_score: f32,
}

/// Project summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSummary {
    pub project_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_files: Option<Vec<String>>,
    pub structure_description: String,
}

/// Decision for context perception events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "lowercase")]
pub enum ContextPerceptionDecision {
    /// Provide context and continue
    Allow {
        injected_context: InjectedContext,
        #[serde(skip_serializing_if = "Option::is_none")]
        metadata: Option<HashMap<String, serde_json::Value>>,
    },
    /// Skip context injection
    Block {
        reason: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        metadata: Option<HashMap<String, serde_json::Value>>,
    },
    /// Request more specific context
    Refine {
        refined_intent: Option<PerceptionIntent>,
        refined_target: Option<PerceptionTarget>,
        scope_hints: Vec<String>,
    },
}

// ============================================================================
// Memory Recall Events
// ============================================================================

/// Memory recall event - model needs to retrieve from memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRecallEvent {
    pub session_id: String,
    pub query: String,
    pub memory_type: String,
    pub max_results: usize,
    pub working_directory: String,
}

/// Decision for memory recall events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "lowercase")]
pub enum MemoryRecallDecision {
    /// Allow recall and provide facts
    Allow {
        injected_facts: Vec<Fact>,
        #[serde(skip_serializing_if = "Option::is_none")]
        metadata: Option<HashMap<String, serde_json::Value>>,
    },
    /// Block recall
    Block {
        reason: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        metadata: Option<HashMap<String, serde_json::Value>>,
    },
}

// ============================================================================
// Planning Events
// ============================================================================

/// Planning strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanningStrategy {
    None,
    StepByStep,
    TreeOfThoughts,
    GraphPlanning,
    Custom(String),
}

/// Planning event - model is doing task planning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningEvent {
    pub session_id: String,
    pub task_description: String,
    pub available_strategies: Vec<PlanningStrategy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub constraints: Option<serde_json::Value>,
}

/// Decision for planning events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "lowercase")]
pub enum PlanningDecision {
    /// Allow planning with strategy
    Allow {
        selected_strategy: PlanningStrategy,
        #[serde(skip_serializing_if = "Option::is_none")]
        planning_template: Option<serde_json::Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        metadata: Option<HashMap<String, serde_json::Value>>,
    },
    /// Block planning
    Block {
        reason: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        metadata: Option<HashMap<String, serde_json::Value>>,
    },
    /// Modify planning parameters
    Modify {
        modified_task: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        hints: Option<Vec<String>>,
    },
}

// ============================================================================
// Reasoning Events
// ============================================================================

/// Reasoning type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningType {
    ChainOfThought,
    TreeOfThoughts,
    ReAct,
    Reflexion,
    Other(String),
}

/// Reasoning event - model is doing chain-of-thought reasoning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningEvent {
    pub session_id: String,
    pub reasoning_type: ReasoningType,
    pub problem_statement: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hints: Option<Vec<String>>,
}

/// Decision for reasoning events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "lowercase")]
pub enum ReasoningDecision {
    /// Allow reasoning with hints
    Allow {
        #[serde(skip_serializing_if = "Option::is_none")]
        hints: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        metadata: Option<HashMap<String, serde_json::Value>>,
    },
    /// Block reasoning
    Block {
        reason: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        metadata: Option<HashMap<String, serde_json::Value>>,
    },
}

// ============================================================================
// Success Event
// ============================================================================

/// Success event - operation succeeded
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessEvent {
    pub session_id: String,
    pub action_type: String,
    pub action_summary: String,
    pub duration_ms: u64,
}

// ============================================================================
// Rate Limit Event
// ============================================================================

/// Rate limit type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RateLimitType {
    LlmTokenLimit,
    LlmRequestLimit,
    ApiRequestLimit,
    ToolExecutionLimit,
    Custom(String),
}

/// Rate limit event - rate limit triggered
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitEvent {
    pub session_id: String,
    pub limit_type: RateLimitType,
    pub retry_after_ms: u64,
    pub current_usage: String,
}

/// Decision for rate limit events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "lowercase")]
pub enum RateLimitDecision {
    /// Retry after delay
    Retry {
        retry_after_ms: u64,
        #[serde(skip_serializing_if = "Option::is_none")]
        metadata: Option<HashMap<String, serde_json::Value>>,
    },
    /// Queue the request
    Queue,
    /// Skip the action
    Skip { reason: String },
}

// ============================================================================
// Confirmation Event
// ============================================================================

/// Confirmation type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfirmationType {
    SafetyConfirm,
    UserConfirm,
    CostConfirm,
    Custom(String),
}

/// Confirmation event - user confirmation needed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfirmationEvent {
    pub session_id: String,
    pub confirmation_type: ConfirmationType,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<String>>,
}

/// Decision for confirmation events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "lowercase")]
pub enum ConfirmationDecision {
    /// Escalate to human
    Escalate,
    /// Approved
    Approve,
    /// Rejected
    Reject { reason: String },
}

// ============================================================================
// Intent Detection Event
// ============================================================================

/// Intent detection event - fired before context perception to detect user intent
///
/// The harness can use LLM classification, keyword matching, or any custom logic
/// to determine the user's intent. This allows multi-language support and more
/// sophisticated intent recognition than local keyword matching.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentDetectionEvent {
    pub session_id: String,
    pub prompt: String,
    pub workspace: String,
    /// Optional language hint auto-detected from input
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language_hint: Option<String>,
}

/// Optional hints about the detected intent target
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetHints {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
}

/// Decision for intent detection events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "lowercase")]
pub enum IntentDetectionDecision {
    /// Intent detected successfully
    Allow {
        /// Detected intent string: "locate" | "understand" | "retrieve" | "explore" | "reason" | "validate" | "compare" | "track"
        detected_intent: String,
        /// Confidence score 0.0 - 1.0
        confidence: f32,
        /// Optional target hints extracted from the prompt
        #[serde(skip_serializing_if = "Option::is_none")]
        target_hints: Option<TargetHints>,
    },
    /// Skip intent detection - use local fallback
    Block { reason: String },
}
