//! Stdio-based AHP transport.
//!
//! Spawns an external harness server process and communicates via
//! newline-delimited JSON-RPC 2.0 over its stdin/stdout.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::{oneshot, Mutex};

use a3s_code_core::hooks::HookEvent;

use crate::protocol::{AhpDecision, AhpMeta, AhpNotification, AhpRequest, AhpResponse, build_params};

type PendingMap = Arc<Mutex<HashMap<u64, oneshot::Sender<AhpDecision>>>>;

/// Stdio-based AHP transport.
///
/// Spawns an external process and communicates via newline-delimited
/// JSON-RPC 2.0 messages over its stdin/stdout.
pub struct StdioTransport {
    stdin: Arc<Mutex<ChildStdin>>,
    pending: PendingMap,
    next_id: Arc<AtomicU64>,
    /// Held alive so the child process is not orphaned on drop.
    _child: Mutex<Child>,
}

impl std::fmt::Debug for StdioTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StdioTransport").finish()
    }
}

impl StdioTransport {
    /// Spawn an external harness server process.
    ///
    /// The child's stderr is inherited so harness log output appears in the
    /// host terminal, making debugging straightforward.
    pub async fn spawn(program: impl AsRef<str>, args: &[impl AsRef<str>]) -> anyhow::Result<Self> {
        let program = program.as_ref();
        let mut cmd = Command::new(program);
        for arg in args {
            cmd.arg(arg.as_ref());
        }
        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::inherit());

        let mut child = cmd.spawn().map_err(|e| {
            anyhow::anyhow!("AHP: failed to spawn harness server '{}': {}", program, e)
        })?;

        let stdin = child.stdin.take().expect("stdin configured as piped");
        let stdout = child.stdout.take().expect("stdout configured as piped");

        let pending: PendingMap = Arc::new(Mutex::new(HashMap::new()));

        // Background reader: parse lines from child stdout and wake pending waiters.
        let pending_reader = pending.clone();
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = tokio::io::AsyncBufReadExt::lines(reader);
            while let Ok(Some(line)) = lines.next_line().await {
                if line.trim().is_empty() {
                    continue;
                }
                match serde_json::from_str::<AhpResponse>(&line) {
                    Ok(resp) => {
                        let mut map = pending_reader.lock().await;
                        if let Some(tx) = map.remove(&resp.id) {
                            let _ = tx.send(resp.result);
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            "AHP: could not parse harness response: {} — line: {}",
                            e,
                            &line[..line.len().min(200)]
                        );
                    }
                }
            }
            tracing::debug!("AHP: harness stdout closed");
        });

        Ok(Self {
            stdin: Arc::new(Mutex::new(stdin)),
            pending,
            next_id: Arc::new(AtomicU64::new(1)),
            _child: Mutex::new(child),
        })
    }

    /// Send a blocking request and await the harness decision.
    ///
    /// On timeout or transport failure, returns `AhpDecision::default()` (continue)
    /// so the agent is never permanently stalled by an unresponsive harness.
    pub async fn send_request(&self, event: &HookEvent, timeout_ms: u64, meta: &AhpMeta) -> AhpDecision {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = oneshot::channel();

        // Register the waiter before writing to avoid a response-before-registration race.
        self.pending.lock().await.insert(id, tx);

        let params = match build_params(event, meta) {
            Some(p) => p,
            None => {
                tracing::error!("AHP: serialization error for request id={}", id);
                self.pending.lock().await.remove(&id);
                return AhpDecision::default();
            }
        };
        let req = AhpRequest::new(id, params);
        let mut line = match serde_json::to_string(&req) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("AHP: serialization error for request id={}: {}", id, e);
                self.pending.lock().await.remove(&id);
                return AhpDecision::default();
            }
        };
        line.push('\n');

        {
            let mut stdin = self.stdin.lock().await;
            if let Err(e) = stdin.write_all(line.as_bytes()).await {
                tracing::error!("AHP: write to harness stdin failed: {}", e);
                self.pending.lock().await.remove(&id);
                return AhpDecision::default();
            }
            let _ = stdin.flush().await;
        }

        let timeout = std::time::Duration::from_millis(timeout_ms);
        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(decision)) => decision,
            Ok(Err(_)) => {
                tracing::warn!("AHP: harness response channel dropped for id={}", id);
                AhpDecision::default()
            }
            Err(_) => {
                tracing::warn!(
                    "AHP: harness response timeout ({}ms) for id={}",
                    timeout_ms,
                    id
                );
                self.pending.lock().await.remove(&id);
                AhpDecision::default()
            }
        }
    }

    /// Send a fire-and-forget notification (no response expected).
    pub async fn send_notification(&self, event: &HookEvent, meta: &AhpMeta) {
        let params = match build_params(event, meta) {
            Some(p) => p,
            None => {
                tracing::error!("AHP: serialization error for notification");
                return;
            }
        };
        let notif = AhpNotification::new(params);
        let mut line = match serde_json::to_string(&notif) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("AHP: serialization error for notification: {}", e);
                return;
            }
        };
        line.push('\n');

        let mut stdin = self.stdin.lock().await;
        if let Err(e) = stdin.write_all(line.as_bytes()).await {
            tracing::error!("AHP: write notification to harness stdin failed: {}", e);
        }
        let _ = stdin.flush().await;
    }
}
