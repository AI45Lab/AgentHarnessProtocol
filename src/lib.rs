//! Agent Harness Protocol (AHP) — host-side implementation.
//!
//! AHP is a language-agnostic protocol for supervising AI agent sessions.
//! The host (A3S Code) forwards hook events to an external harness server
//! via newline-delimited JSON-RPC 2.0 over stdio. The harness server can be
//! implemented in any language — Python, Node.js, Go, or a shell script.
//!
//! # Protocol
//!
//! **Transport**: newline-delimited JSON-RPC 2.0 over child process stdio.
//!
//! **Blocking events** (`pre_tool_use`, `pre_prompt`): sent as requests with
//! an `id`; the host awaits the harness decision before the agent proceeds.
//!
//! **Fire-and-forget events** (all others): sent as notifications (no `id`);
//! the host does not wait for a response.
//!
//! # Wire format
//!
//! ## Request (host → server, blocking event)
//!
//! ```json
//! {
//!   "jsonrpc": "2.0",
//!   "id": 1,
//!   "method": "harness/event",
//!   "params": {
//!     "event_type": "pre_tool_use",
//!     "payload": { ... },
//!     "meta": { "depth": 0 }
//!   }
//! }
//! ```
//!
//! ## Response (server → host)
//!
//! ```json
//! {
//!   "jsonrpc": "2.0",
//!   "id": 1,
//!   "result": { "action": "continue", "reason": null, "modified": null, "retry_delay_ms": null }
//! }
//! ```
//!
//! Valid `action` values: `"continue"`, `"block"`, `"skip"`, `"retry"`.
//!
//! # Usage
//!
//! ```no_run
//! use a3s_ahp::AhpHookExecutor;
//! use a3s_code_core::{Agent, SessionOptions};
//!
//! # async fn run() -> anyhow::Result<()> {
//! let harness = AhpHookExecutor::spawn("python3", &["harness.py"]).await?;
//! let opts = SessionOptions::new().with_hook_executor(harness);
//! let session = Agent::new("agent.hcl").await?.session("/workspace", Some(opts))?;
//! # Ok(())
//! # }
//! ```

mod protocol;
mod transport;

pub use protocol::{AhpAction, AhpDecision, AhpMeta};
pub use transport::StdioTransport;

use a3s_code_core::hooks::{HookEvent, HookEventType, HookResult};
use async_trait::async_trait;
use std::sync::Arc;

/// Blocking event types — the host awaits a decision before proceeding.
fn is_blocking(event_type: HookEventType) -> bool {
    matches!(
        event_type,
        HookEventType::PreToolUse | HookEventType::PrePrompt
    )
}

fn decision_to_result(decision: AhpDecision) -> HookResult {
    match decision.action {
        AhpAction::Continue => HookResult::Continue(decision.modified),
        AhpAction::Block => HookResult::Block(
            decision
                .reason
                .unwrap_or_else(|| "blocked by AHP harness".to_string()),
        ),
        AhpAction::Skip => HookResult::Skip,
        AhpAction::Retry => HookResult::Retry(decision.retry_delay_ms.unwrap_or(1000)),
    }
}

/// AHP hook executor — delegates hook events to an external harness server.
///
/// Implements [`HookExecutor`][a3s_code_core::hooks::HookExecutor] by forwarding
/// lifecycle events to a child process via the Agent Harness Protocol
/// (JSON-RPC 2.0 over stdio).
///
/// The harness server can be written in **any language** — it just needs to
/// read newline-delimited JSON from stdin and write responses to stdout.
#[derive(Debug)]
pub struct AhpHookExecutor {
    transport: Arc<StdioTransport>,
    /// Timeout for blocking requests in milliseconds. Default: 10 000.
    timeout_ms: u64,
    /// Sub-agent nesting depth injected into every AHP message.
    ///
    /// `0` = user-facing session, `1` = first-level sub-agent, etc.
    depth: u32,
}

impl AhpHookExecutor {
    /// Spawn a harness server process and return a ready executor.
    ///
    /// - `program` — executable to run (e.g. `"python3"`, `"node"`, `"./harness"`)
    /// - `args` — arguments passed to the process (e.g. `&["harness.py"]`)
    ///
    /// The child's stderr is inherited so harness log output appears in the
    /// host terminal.
    pub async fn spawn(
        program: impl AsRef<str>,
        args: &[impl AsRef<str>],
    ) -> anyhow::Result<Arc<Self>> {
        Self::spawn_with_timeout(program, args, 10_000).await
    }

    /// Spawn with an explicit blocking-request timeout in milliseconds.
    ///
    /// On timeout the executor falls back to `continue`, so the agent is never
    /// permanently stalled by an unresponsive harness.
    pub async fn spawn_with_timeout(
        program: impl AsRef<str>,
        args: &[impl AsRef<str>],
        timeout_ms: u64,
    ) -> anyhow::Result<Arc<Self>> {
        let transport = StdioTransport::spawn(program, args).await?;
        Ok(Arc::new(Self {
            transport: Arc::new(transport),
            timeout_ms,
            depth: 0,
        }))
    }

    /// Return a clone of this executor with the sub-agent nesting depth incremented by one.
    ///
    /// Used internally by the task tool for sub-agent propagation so the harness
    /// server receives a monotonically increasing `meta.depth` value.
    pub fn with_incremented_depth(&self) -> Arc<Self> {
        Arc::new(Self {
            transport: self.transport.clone(),
            timeout_ms: self.timeout_ms,
            depth: self.depth + 1,
        })
    }
}

#[async_trait]
impl a3s_code_core::hooks::HookExecutor for AhpHookExecutor {
    async fn fire(&self, event: &HookEvent) -> HookResult {
        let meta = AhpMeta::new(self.depth);
        if is_blocking(event.event_type()) {
            let decision = self.transport.send_request(event, self.timeout_ms, &meta).await;
            decision_to_result(decision)
        } else {
            // Spawn so fire-and-forget notifications never delay the agent loop.
            let transport = self.transport.clone();
            let event = event.clone();
            tokio::spawn(async move {
                transport.send_notification(&event, &meta).await;
            });
            HookResult::continue_()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_blocking() {
        assert!(is_blocking(HookEventType::PreToolUse));
        assert!(is_blocking(HookEventType::PrePrompt));
        assert!(!is_blocking(HookEventType::PostToolUse));
        assert!(!is_blocking(HookEventType::SessionStart));
        assert!(!is_blocking(HookEventType::GenerateEnd));
        assert!(!is_blocking(HookEventType::OnError));
    }

    #[test]
    fn test_decision_to_result_continue() {
        let d = AhpDecision::default();
        assert!(decision_to_result(d).is_continue());
    }

    #[test]
    fn test_decision_to_result_block() {
        let d = AhpDecision {
            action: AhpAction::Block,
            reason: Some("dangerous".into()),
            modified: None,
            retry_delay_ms: None,
        };
        if let HookResult::Block(msg) = decision_to_result(d) {
            assert_eq!(msg, "dangerous");
        } else {
            panic!("expected Block");
        }
    }

    #[test]
    fn test_decision_to_result_block_no_reason() {
        let d = AhpDecision {
            action: AhpAction::Block,
            reason: None,
            modified: None,
            retry_delay_ms: None,
        };
        if let HookResult::Block(msg) = decision_to_result(d) {
            assert_eq!(msg, "blocked by AHP harness");
        } else {
            panic!("expected Block");
        }
    }

    #[test]
    fn test_decision_to_result_skip() {
        let d = AhpDecision {
            action: AhpAction::Skip,
            reason: None,
            modified: None,
            retry_delay_ms: None,
        };
        assert!(matches!(decision_to_result(d), HookResult::Skip));
    }

    #[test]
    fn test_decision_to_result_retry_default_delay() {
        let d = AhpDecision {
            action: AhpAction::Retry,
            reason: None,
            modified: None,
            retry_delay_ms: None,
        };
        assert!(matches!(decision_to_result(d), HookResult::Retry(1000)));
    }

    #[test]
    fn test_decision_to_result_retry_custom_delay() {
        let d = AhpDecision {
            action: AhpAction::Retry,
            reason: None,
            modified: None,
            retry_delay_ms: Some(500),
        };
        assert!(matches!(decision_to_result(d), HookResult::Retry(500)));
    }
}
