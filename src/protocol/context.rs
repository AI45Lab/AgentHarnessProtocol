use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Idle event fired when the agent has been idle for a threshold duration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdleEvent {
    /// Duration of idle time in milliseconds.
    pub idle_duration_ms: u64,
    /// Reason for becoming idle.
    pub idle_reason: String,
    /// Last event type before becoming idle.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_event_type: Option<String>,
    /// Suggested action for the idle period.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_action: Option<String>,
}

/// Heartbeat event for periodic status updates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatEvent {
    /// Agent uptime in milliseconds.
    pub uptime_ms: u64,
    /// Total events processed since start.
    pub total_events_processed: u64,
    /// Current agent state.
    pub current_state: String,
    /// CPU usage percentage (0-100).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_percent: Option<f32>,
    /// Memory usage in bytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_bytes: Option<u64>,
    /// Currently active tool count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_tools: Option<usize>,
    /// Pending actions in queue.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending_actions: Option<usize>,
    /// Queue depth.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queue_depth: Option<usize>,
    /// Tokens used in current session.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens_used: Option<i32>,
}

/// Structured context passed with events for context-aware control decisions.
///
/// The client exposes capabilities it chooses. The server can use exposed
/// capabilities and should ignore unrecognized or unneeded entries.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EventContext {
    /// Recent facts or knowledge retrieved.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recent_facts: Option<Vec<Fact>>,
    /// Memory/knowledge base state summary.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_summary: Option<MemorySummary>,
    /// Session statistics.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_stats: Option<SessionStats>,
    /// Current task/goal description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_task: Option<String>,
    /// Client-defined capabilities as arbitrary key-value pairs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<HashMap<String, serde_json::Value>>,
}

/// A factual memory item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fact {
    pub content: String,
    pub source: String,
    pub confidence: f32,
}

/// Memory state summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySummary {
    pub memory_type: String,
    pub total_items: usize,
    pub recent_topics: Vec<String>,
}

/// Session statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStats {
    pub total_actions: usize,
    pub total_tokens: i32,
    pub duration_ms: u64,
    pub error_count: usize,
}

/// Decision for idle/dream events.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "lowercase")]
pub enum IdleDecision {
    /// Allow background consolidation/dream.
    Allow,
    /// Defer idle processing.
    Defer {
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
}

/// Perception intent - why the model needs to perceive context.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PerceptionIntent {
    /// Identify or classify an entity.
    Recognize,
    /// Understand semantics, logic, or behavior.
    Understand,
    /// Find the location or path of a resource.
    Locate,
    /// Recall relevant information from accumulated knowledge.
    Retrieve,
    /// Understand the overall structure of an environment or system.
    Explore,
    /// Infer causality or logic based on existing information.
    Reason,
    /// Confirm whether an assumption or state is correct.
    Validate,
    /// Compare similarities and differences.
    Compare,
    /// Get current status or history of a process.
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

/// Perception target - what the model wants to perceive.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PerceptionTarget {
    /// A specific object or concept.
    Entity {
        name: String,
        /// Entity type, such as function, file, person, document, config, or API.
        entity_type: String,
        /// Identifier attribute.
        identifier: Option<String>,
    },
    /// A path or place.
    Location {
        path: String,
        /// Location type, such as file, directory, URL, endpoint, or region.
        location_type: String,
    },
    /// Something that happened or will happen.
    Event {
        description: String,
        /// Event type, such as change, action, decision, error, or meeting.
        event_type: String,
        /// Related time range.
        time_range: Option<TimeRange>,
    },
    /// Connections between multiple entities.
    Relation {
        entities: Vec<String>,
        /// Relation type, such as dependency, ownership, sequence, or conflict.
        relation_type: String,
    },
    /// A policy, strategy, or convention.
    Rule {
        name: String,
        /// Rule type, such as policy, convention, constraint, or requirement.
        rule_type: String,
        /// Applicable scope.
        scope: Option<String>,
    },
    /// Current condition of a system or entity.
    State {
        target: String,
        aspect: String,
        include_history: bool,
    },
    /// An available capability or asset.
    Resource {
        name: String,
        /// Resource type, such as tool, skill, API, data, or personnel.
        resource_type: String,
        constraints: Option<serde_json::Value>,
    },
    /// A recurring phenomenon or rule.
    Pattern {
        pattern: String,
        /// Pattern type, such as text, regex, structure, or behavior.
        pattern_type: String,
    },
}

/// Time range for events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub from: Option<i64>,
    pub to: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relative: Option<String>,
}

/// Perception domain - the domain of the current task.
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

/// Perception modality - the form of information needed.
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

/// Quality requirement for perception.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PerceptionUrgency {
    /// Must have - cannot proceed without this context.
    Critical,
    /// Important - significantly improves response quality.
    High,
    /// Helpful but not essential.
    Normal,
    /// Can be cached or delayed.
    Low,
}

impl Default for PerceptionUrgency {
    fn default() -> Self {
        PerceptionUrgency::Normal
    }
}

/// Freshness requirement for perceived data.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PerceptionFreshness {
    /// Need realtime data, such as git status or filesystem state.
    Realtime,
    /// Need recent data, such as recent policy changes.
    Recent,
    /// Static data is sufficient, such as code structure or archives.
    Static,
}

impl Default for PerceptionFreshness {
    fn default() -> Self {
        PerceptionFreshness::Static
    }
}

/// Constraints for perception.
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

/// Current perception context.
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

/// History item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryItem {
    pub item_type: String,
    pub content: String,
    pub timestamp: i64,
}

/// Context perception event fired when the model needs workspace knowledge.
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

/// Injected context from harness.
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

/// File content snippet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileContentSnippet {
    pub path: String,
    pub snippet: String,
    pub relevance_score: f32,
}

/// Project summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSummary {
    pub project_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_files: Option<Vec<String>>,
    pub structure_description: String,
}

/// Decision for context perception events.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "lowercase")]
pub enum ContextPerceptionDecision {
    /// Provide context and continue.
    Allow {
        injected_context: InjectedContext,
        #[serde(skip_serializing_if = "Option::is_none")]
        metadata: Option<HashMap<String, serde_json::Value>>,
    },
    /// Skip context injection.
    Block {
        reason: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        metadata: Option<HashMap<String, serde_json::Value>>,
    },
    /// Request more specific context.
    Refine {
        refined_intent: Option<PerceptionIntent>,
        refined_target: Option<PerceptionTarget>,
        scope_hints: Vec<String>,
    },
}
