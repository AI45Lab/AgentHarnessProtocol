//! stdio transport implementation

use crate::transport::TransportLayer;
use crate::{AhpError, AhpNotification, AhpRequest, AhpResponse, Result, TransportConfig};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;

/// stdio transport - communicates with child process via stdin/stdout
pub struct StdioTransport {
    stdin: Arc<Mutex<ChildStdin>>,
    stdout: Arc<Mutex<BufReader<ChildStdout>>>,
    _child: Arc<Mutex<Child>>,
    pending_requests: Arc<Mutex<HashMap<String, tokio::sync::oneshot::Sender<AhpResponse>>>>,
    timeout_ms: u64,
}

impl StdioTransport {
    /// Spawn a child process and create a stdio transport
    pub async fn spawn(program: impl AsRef<str>, args: &[impl AsRef<str>]) -> Result<Self> {
        Self::spawn_with_config(program, args, &TransportConfig::default()).await
    }

    /// Spawn a child process and create a stdio transport with explicit config.
    pub async fn spawn_with_config(
        program: impl AsRef<str>,
        args: &[impl AsRef<str>],
        config: &TransportConfig,
    ) -> Result<Self> {
        let mut cmd = Command::new(program.as_ref());
        for arg in args {
            cmd.arg(arg.as_ref());
        }

        cmd.stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit());

        let mut child = cmd
            .spawn()
            .map_err(|e| AhpError::Transport(format!("Failed to spawn process: {}", e)))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| AhpError::Transport("Failed to capture stdin".to_string()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| AhpError::Transport("Failed to capture stdout".to_string()))?;

        let transport = Self {
            stdin: Arc::new(Mutex::new(stdin)),
            stdout: Arc::new(Mutex::new(BufReader::new(stdout))),
            _child: Arc::new(Mutex::new(child)),
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
            timeout_ms: config.timeout_ms,
        };

        // Start background task to read responses
        transport.start_reader();

        Ok(transport)
    }

    fn start_reader(&self) {
        let stdout = self.stdout.clone();
        let pending = self.pending_requests.clone();

        tokio::spawn(async move {
            loop {
                let mut stdout_guard = stdout.lock().await;
                let mut line = String::new();

                match stdout_guard.read_line(&mut line).await {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        drop(stdout_guard); // Release lock before processing

                        if let Ok(response) = serde_json::from_str::<AhpResponse>(&line) {
                            let mut pending_guard = pending.lock().await;
                            if let Some(sender) = pending_guard.remove(&response.id) {
                                let _ = sender.send(response);
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        });
    }
}

#[async_trait]
impl TransportLayer for StdioTransport {
    async fn send_request(&self, request: AhpRequest) -> Result<AhpResponse> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let request_id = request.id.clone();
        let json = serde_json::to_string(&request)?;

        // Register pending request
        {
            let mut pending = self.pending_requests.lock().await;
            pending.insert(request_id.clone(), tx);
        }

        // Send request
        let mut stdin = self.stdin.lock().await;
        if let Err(e) = stdin.write_all(json.as_bytes()).await {
            self.pending_requests.lock().await.remove(&request_id);
            return Err(e.into());
        }
        if let Err(e) = stdin.write_all(b"\n").await {
            self.pending_requests.lock().await.remove(&request_id);
            return Err(e.into());
        }
        if let Err(e) = stdin.flush().await {
            self.pending_requests.lock().await.remove(&request_id);
            return Err(e.into());
        }
        drop(stdin);

        // Wait for response with timeout
        match tokio::time::timeout(std::time::Duration::from_millis(self.timeout_ms), rx).await {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(_)) => Err(AhpError::ConnectionClosed),
            Err(_) => {
                self.pending_requests.lock().await.remove(&request_id);
                Err(AhpError::Timeout(self.timeout_ms))
            }
        }
    }

    async fn send_notification(&self, notification: AhpNotification) -> Result<()> {
        let mut stdin = self.stdin.lock().await;
        let json = serde_json::to_string(&notification)?;
        stdin.write_all(json.as_bytes()).await?;
        stdin.write_all(b"\n").await?;
        stdin.flush().await?;
        Ok(())
    }

    async fn close(&self) -> Result<()> {
        // Child process will be killed when dropped
        Ok(())
    }
}
