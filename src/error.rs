//! Error types for AHP

use thiserror::Error;

pub type Result<T> = std::result::Result<T, AhpError>;

#[derive(Error, Debug)]
pub enum AhpError {
    #[error("Transport error: {0}")]
    Transport(String),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Authentication failed: {0}")]
    AuthFailed(String),

    #[error("Timeout after {0}ms")]
    Timeout(u64),

    #[error("Invalid event type: {0}")]
    InvalidEventType(String),

    #[error("Invalid decision: {0}")]
    InvalidDecision(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Connection closed")]
    ConnectionClosed,

    #[error("Capability not supported: {0}")]
    UnsupportedCapability(String),

    #[error("{0}")]
    Other(String),
}

impl From<String> for AhpError {
    fn from(s: String) -> Self {
        AhpError::Other(s)
    }
}

impl From<&str> for AhpError {
    fn from(s: &str) -> Self {
        AhpError::Other(s.to_string())
    }
}

impl From<anyhow::Error> for AhpError {
    fn from(e: anyhow::Error) -> Self {
        AhpError::Other(e.to_string())
    }
}
