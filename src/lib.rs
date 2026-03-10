//! Agent Harness Protocol (AHP) v2.0
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
//! - **Transport-flexible** — Supports stdio, HTTP, WebSocket, gRPC, Unix sockets
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
//! let handshake = client.handshake().await?;
//! println!("Connected to: {}", handshake.harness_info.name);
//!
//! // Send pre-action event
//! let decision = client.send_event(EventType::PreAction, serde_json::json!({
//!     "action_type": "tool_call",
//!     "tool_name": "bash",
//!     "arguments": { "command": "ls -la" }
//! })).await?;
//!
//! match decision {
//!     Decision::Allow => println!("Action allowed"),
//!     Decision::Block { reason } => println!("Action blocked: {}", reason),
//!     _ => {}
//! }
//! # Ok(())
//! # }
//! ```

pub mod client;
pub mod server;
pub mod protocol;
pub mod transport;
pub mod auth;
pub mod error;

// Re-exports
pub use client::AhpClient;
pub use server::AhpServer;
pub use protocol::{
    AhpEvent, AhpRequest, AhpResponse, AhpNotification,
    EventType, Decision, HandshakeRequest, HandshakeResponse,
    QueryRequest, QueryResponse, BatchRequest, BatchResponse,
};
pub use transport::{Transport, TransportConfig};
pub use auth::{AuthConfig, AuthMethod};
pub use error::{AhpError, Result};

/// Protocol version
pub const PROTOCOL_VERSION: &str = "2.0";

/// Default timeout for blocking requests (milliseconds)
pub const DEFAULT_TIMEOUT_MS: u64 = 10_000;

/// Default batch size
pub const DEFAULT_BATCH_SIZE: usize = 100;
