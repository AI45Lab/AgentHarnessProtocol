//! Transport layer abstractions

use crate::{AhpRequest, AhpResponse, AhpNotification, AuthConfig, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Transport configuration
#[derive(Debug, Clone)]
pub enum Transport {
    /// stdio transport (local child process)
    Stdio {
        program: String,
        args: Vec<String>,
    },
    
    /// HTTP transport (remote harness server)
    #[cfg(feature = "http")]
    Http {
        url: String,
        auth: Option<AuthConfig>,
    },
    
    /// WebSocket transport (bidirectional streaming)
    #[cfg(feature = "websocket")]
    WebSocket {
        url: String,
        auth: Option<AuthConfig>,
    },
    
    /// gRPC transport (high-performance RPC)
    #[cfg(feature = "grpc")]
    Grpc {
        endpoint: String,
        auth: Option<AuthConfig>,
    },
    
    /// Unix socket transport (local IPC)
    #[cfg(feature = "unix-socket")]
    UnixSocket {
        path: String,
    },
}

/// Transport configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportConfig {
    pub timeout_ms: u64,
    pub max_retries: u32,
    pub retry_delay_ms: u64,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            timeout_ms: 10_000,
            max_retries: 3,
            retry_delay_ms: 1_000,
        }
    }
}

/// Transport trait - all transports must implement this
#[async_trait]
pub trait TransportLayer: Send + Sync {
    /// Send a request and wait for response
    async fn send_request(&self, request: AhpRequest) -> Result<AhpResponse>;
    
    /// Send a notification (fire-and-forget, no response expected)
    async fn send_notification(&self, notification: AhpNotification) -> Result<()>;
    
    /// Close the transport connection
    async fn close(&self) -> Result<()>;
}

// Module declarations for different transport implementations
#[cfg(feature = "stdio")]
pub mod stdio;

#[cfg(feature = "http")]
pub mod http;

#[cfg(feature = "websocket")]
pub mod websocket;

#[cfg(feature = "grpc")]
pub mod grpc;

#[cfg(feature = "unix-socket")]
pub mod unix_socket;
