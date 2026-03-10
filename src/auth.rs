//! Authentication and authorization

use serde::{Deserialize, Serialize};

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub method: AuthMethod,
}

/// Authentication methods
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AuthMethod {
    /// No authentication (local stdio, trusted environment)
    None,
    
    /// API key authentication
    ApiKey {
        key: String,
    },
    
    /// Bearer token authentication
    Bearer {
        token: String,
    },
    
    /// Mutual TLS (client and server certificates)
    MutualTls {
        cert_path: String,
        key_path: String,
        ca_path: Option<String>,
    },
    
    /// OAuth 2.0
    OAuth {
        client_id: String,
        client_secret: String,
        token_url: String,
    },
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            method: AuthMethod::None,
        }
    }
}

impl AuthConfig {
    pub fn none() -> Self {
        Self {
            method: AuthMethod::None,
        }
    }

    pub fn api_key(key: impl Into<String>) -> Self {
        Self {
            method: AuthMethod::ApiKey { key: key.into() },
        }
    }

    pub fn bearer(token: impl Into<String>) -> Self {
        Self {
            method: AuthMethod::Bearer { token: token.into() },
        }
    }
}
