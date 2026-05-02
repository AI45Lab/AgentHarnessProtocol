//! Agent Harness Protocol (AHP) v2.4
//!
//! Universal, transport-agnostic protocol for supervising autonomous AI agents.
//!
//! # Overview
//!
//! AHP provides a framework-agnostic interface for supervising, controlling, and auditing
//! AI agents. Unlike traditional monitoring solutions tightly coupled to specific frameworks,
//! AHP works with any agent system through a standardized JSON-RPC 2.0 protocol.
//!
//! # Features
//!
//! - **Framework-agnostic** — Works with any agent system
//! - **Language-neutral** — Harness servers can be written in any language
//! - **Transport-flexible** — Supports stdio, HTTP, WebSocket, and Unix sockets;
//!   gRPC is reserved as a feature placeholder
//! - **Bidirectional** — Agents can query the harness for guidance
//! - **Secure** — Built-in authentication and encryption support
//!
//! # Quick Start
//!
//! ```no_run
//! use a3s_ahp::{AhpClient, Transport, EventType, Decision};
//!
//! # async fn example() -> anyhow::Result<()> {
//! // Create client with stdio transport
//! let client = AhpClient::new(Transport::Stdio {
//!     program: "python3".into(),
//!     args: vec!["harness.py".into()],
//! }).await?;
//!
//! // Send handshake
//! let handshake = client.handshake(Vec::new()).await?;
//! println!("Connected to: {}", handshake.harness_info.name);
//!
//! // Send pre-action event
//! let decision = client.send_event_decision(EventType::PreAction, serde_json::json!({
//!     "action_type": "tool_call",
//!     "tool_name": "bash",
//!     "arguments": { "command": "ls -la" }
//! })).await?;
//!
//! match decision {
//!     Decision::Allow { .. } => println!("Action allowed"),
//!     Decision::Block { reason, .. } => println!("Action blocked: {}", reason),
//!     _ => {}
//! }
//! # Ok(())
//! # }
//! ```

pub mod auth;
pub mod client;
pub mod error;
pub mod protocol;
pub mod server;
pub mod transport;

// Re-exports
pub use auth::{AuthConfig, AuthMethod};
pub use client::AhpClient;
pub use error::{AhpError, Result};
pub use protocol::{
    AgentInfo, AhpErrorObject, AhpEvent, AhpNotification, AhpRequest, AhpResponse, ArtifactRef,
    BatchRequest, BatchResponse, ConfirmationDecision, ConfirmationEvent, ConfirmationType,
    ContextPerceptionDecision, ContextPerceptionEvent, Decision, EventContext, EventType,
    EvidenceRef, Fact, FileContentSnippet, HandshakeRequest, HandshakeResponse, HarnessConfig,
    HarnessInfo, HeartbeatEvent, HistoryItem, IdleDecision, IdleEvent, InjectedContext,
    IntentDetectionDecision, IntentDetectionEvent, MemoryRecallDecision, MemoryRecallEvent,
    MemorySummary, PerceptionConstraints, PerceptionContext, PerceptionDomain, PerceptionFreshness,
    PerceptionIntent, PerceptionModality, PerceptionTarget, PerceptionUrgency, PlanningDecision,
    PlanningEvent, PlanningStrategy, ProjectSummary, QueryRequest, QueryResponse,
    RateLimitDecision, RateLimitEvent, RateLimitType, ReasoningDecision, ReasoningEvent,
    ReasoningType, RunLifecycleEvent, RunStatus, SessionStats, SuccessEvent, TargetHints, TaskItem,
    TaskListEvent, TaskStatus, TimeRange, VerificationCheck, VerificationEvent, VerificationStatus,
};
pub use server::AhpServer;
pub use transport::{Transport, TransportConfig};

/// Protocol version
pub const PROTOCOL_VERSION: &str = "2.4";

/// Default timeout for blocking requests (milliseconds)
pub const DEFAULT_TIMEOUT_MS: u64 = 10_000;

/// Default batch size
pub const DEFAULT_BATCH_SIZE: usize = 100;
