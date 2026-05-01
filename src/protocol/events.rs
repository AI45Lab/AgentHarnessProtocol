use super::Fact;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Memory recall event - model needs to retrieve from memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRecallEvent {
    pub session_id: String,
    pub query: String,
    pub memory_type: String,
    pub max_results: usize,
    pub working_directory: String,
}

/// Decision for memory recall events.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "lowercase")]
pub enum MemoryRecallDecision {
    /// Allow recall and provide facts.
    Allow {
        injected_facts: Vec<Fact>,
        #[serde(skip_serializing_if = "Option::is_none")]
        metadata: Option<HashMap<String, serde_json::Value>>,
    },
    /// Block recall.
    Block {
        reason: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        metadata: Option<HashMap<String, serde_json::Value>>,
    },
}

/// Planning strategy.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanningStrategy {
    None,
    StepByStep,
    TreeOfThoughts,
    GraphPlanning,
    Custom(String),
}

/// Planning event - model is doing task planning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningEvent {
    pub session_id: String,
    pub task_description: String,
    pub available_strategies: Vec<PlanningStrategy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub constraints: Option<serde_json::Value>,
}

/// Decision for planning events.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "lowercase")]
pub enum PlanningDecision {
    /// Allow planning with strategy.
    Allow {
        selected_strategy: PlanningStrategy,
        #[serde(skip_serializing_if = "Option::is_none")]
        planning_template: Option<serde_json::Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        metadata: Option<HashMap<String, serde_json::Value>>,
    },
    /// Block planning.
    Block {
        reason: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        metadata: Option<HashMap<String, serde_json::Value>>,
    },
    /// Modify planning parameters.
    Modify {
        modified_task: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        hints: Option<Vec<String>>,
    },
}

/// Reasoning type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningType {
    ChainOfThought,
    TreeOfThoughts,
    ReAct,
    Reflexion,
    Other(String),
}

/// Reasoning event - model is doing chain-of-thought reasoning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningEvent {
    pub session_id: String,
    pub reasoning_type: ReasoningType,
    pub problem_statement: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hints: Option<Vec<String>>,
}

/// Decision for reasoning events.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "lowercase")]
pub enum ReasoningDecision {
    /// Allow reasoning with hints.
    Allow {
        #[serde(skip_serializing_if = "Option::is_none")]
        hints: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        metadata: Option<HashMap<String, serde_json::Value>>,
    },
    /// Block reasoning.
    Block {
        reason: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        metadata: Option<HashMap<String, serde_json::Value>>,
    },
}

/// Success event - operation succeeded.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessEvent {
    pub session_id: String,
    pub action_type: String,
    pub action_summary: String,
    pub duration_ms: u64,
}

/// Rate limit type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RateLimitType {
    LlmTokenLimit,
    LlmRequestLimit,
    ApiRequestLimit,
    ToolExecutionLimit,
    Custom(String),
}

/// Rate limit event - rate limit triggered.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitEvent {
    pub session_id: String,
    pub limit_type: RateLimitType,
    pub retry_after_ms: u64,
    pub current_usage: String,
}

/// Decision for rate limit events.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "lowercase")]
pub enum RateLimitDecision {
    /// Retry after delay.
    Retry {
        retry_after_ms: u64,
        #[serde(skip_serializing_if = "Option::is_none")]
        metadata: Option<HashMap<String, serde_json::Value>>,
    },
    /// Queue the request.
    Queue,
    /// Skip the action.
    Skip { reason: String },
}

/// Confirmation type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfirmationType {
    SafetyConfirm,
    UserConfirm,
    CostConfirm,
    Custom(String),
}

/// Confirmation event - user confirmation needed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfirmationEvent {
    pub session_id: String,
    pub confirmation_type: ConfirmationType,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<String>>,
}

/// Decision for confirmation events.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "lowercase")]
pub enum ConfirmationDecision {
    /// Escalate to human.
    Escalate,
    /// Approved.
    Approve,
    /// Rejected.
    Reject { reason: String },
}

/// Intent detection event fired before context perception.
///
/// The harness can use LLM classification, keyword matching, or any custom logic
/// to determine the user's intent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentDetectionEvent {
    pub session_id: String,
    pub prompt: String,
    pub workspace: String,
    /// Optional language hint auto-detected from input.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language_hint: Option<String>,
}

/// Optional hints about the detected intent target.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetHints {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
}

/// Decision for intent detection events.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "lowercase")]
pub enum IntentDetectionDecision {
    /// Intent detected successfully.
    Allow {
        /// Detected intent string.
        detected_intent: String,
        /// Confidence score 0.0 - 1.0.
        confidence: f32,
        /// Optional target hints extracted from the prompt.
        #[serde(skip_serializing_if = "Option::is_none")]
        target_hints: Option<TargetHints>,
    },
    /// Skip intent detection and use local fallback.
    Block { reason: String },
}
