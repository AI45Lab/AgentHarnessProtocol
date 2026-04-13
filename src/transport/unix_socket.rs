//! Unix socket transport implementation

use crate::transport::TransportLayer;
use crate::{AhpError, AhpNotification, AhpRequest, AhpResponse, Result};
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::sync::{oneshot, Mutex};

/// Unix socket transport - communicates with server via Unix domain socket
pub struct UnixSocketTransport {
    writer: Arc<Mutex<tokio::io::WriteHalf<UnixStream>>>,
    _reader_task: Arc<()>, // Keeps the reader half alive
    pending_requests: Arc<Mutex<HashMap<String, oneshot::Sender<AhpResponse>>>>,
}

impl UnixSocketTransport {
    /// Connect to a Unix socket server
    pub async fn connect(path: impl AsRef<Path>) -> Result<Self> {
        let stream = UnixStream::connect(path.as_ref()).await.map_err(|e| {
            AhpError::Transport(format!(
                "Failed to connect to {}: {}",
                path.as_ref().display(),
                e
            ))
        })?;

        let pending_requests: Arc<Mutex<HashMap<String, oneshot::Sender<AhpResponse>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let pending_clone = pending_requests.clone();

        // Split the stream
        let (reader, writer) = tokio::io::split(stream);

        // Start background task to read responses
        let reader = BufReader::new(reader);
        let reader_task = tokio::spawn(async move {
            let mut reader = reader;
            loop {
                let mut line = String::new();
                match reader.read_line(&mut line).await {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        if let Ok(response) = serde_json::from_str::<AhpResponse>(&line) {
                            let mut pending_guard = pending_clone.lock().await;
                            if let Some(sender) = pending_guard.remove(&response.id) {
                                let _ = sender.send(response);
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        let transport = Self {
            writer: Arc::new(Mutex::new(writer)),
            _reader_task: Arc::new(reader_task),
            pending_requests,
        };

        Ok(transport)
    }
}

#[async_trait]
impl TransportLayer for UnixSocketTransport {
    async fn send_request(&self, request: AhpRequest) -> Result<AhpResponse> {
        let (tx, rx) = oneshot::channel();

        // Register pending request
        {
            let mut pending = self.pending_requests.lock().await;
            pending.insert(request.id.clone(), tx);
        }

        // Send request
        let mut writer = self.writer.lock().await;
        let json = serde_json::to_string(&request)?;
        writer.write_all(json.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;
        drop(writer);

        // Wait for response with timeout
        match tokio::time::timeout(std::time::Duration::from_millis(10_000), rx).await {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(_)) => Err(AhpError::ConnectionClosed),
            Err(_) => Err(AhpError::Timeout(10_000)),
        }
    }

    async fn send_notification(&self, notification: AhpNotification) -> Result<()> {
        let mut writer = self.writer.lock().await;
        let json = serde_json::to_string(&notification)?;
        writer.write_all(json.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;
        Ok(())
    }

    async fn close(&self) -> Result<()> {
        // Dropping the writer will close the connection
        Ok(())
    }
}
