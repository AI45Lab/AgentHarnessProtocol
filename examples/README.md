# AHP Examples

This directory contains examples demonstrating different transport layers and usage patterns for the Agent Harness Protocol (AHP) v2.0.

## Examples

### 1. stdio Transport (Local Process)

**Client:** `simple_client.rs`
**Server:** `simple_server.py`

The simplest transport - spawns a child process and communicates via stdin/stdout.

```bash
# Run the client (it will spawn the Python server automatically)
cargo run --example simple_client
```

### 2. HTTP Transport (Remote Server)

**Client:** `http_client.rs`
**Server:** `http_server.rs`

HTTP-based transport for remote harness servers.

```bash
# Terminal 1: Start the HTTP server
cargo run --example http_server --features http

# Terminal 2: Run the HTTP client
cargo run --example http_client --features http
```

**Features:**
- RESTful API
- Authentication support (API key, Bearer token)
- Load balancing ready
- Works with standard HTTP infrastructure

### 3. WebSocket Transport (Bidirectional Streaming)

**Client:** `websocket_client.rs`
**Server:** `websocket_server.rs`

WebSocket-based transport for low-latency bidirectional communication.

```bash
# Terminal 1: Start the WebSocket server
cargo run --example websocket_server --features websocket

# Terminal 2: Run the WebSocket client
cargo run --example websocket_client --features websocket
```

**Features:**
- Persistent connection
- Low latency
- Bidirectional streaming
- Efficient for high-frequency events
- Batch processing support

## Authentication Examples

### API Key Authentication (HTTP)

```rust
use a3s_ahp::{AhpClient, Transport, AuthConfig};

let client = AhpClient::new(Transport::Http {
    url: "http://localhost:8080/ahp".into(),
    auth: Some(AuthConfig::api_key("your-api-key")),
}).await?;
```

### Bearer Token Authentication (HTTP)

```rust
let client = AhpClient::new(Transport::Http {
    url: "http://localhost:8080/ahp".into(),
    auth: Some(AuthConfig::bearer("your-token")),
}).await?;
```

### WebSocket with Authentication

```rust
let client = AhpClient::new(Transport::WebSocket {
    url: "ws://localhost:8081/ahp".into(),
    auth: Some(AuthConfig::api_key("your-api-key")),
}).await?;
```

## Event Types

### Blocking Events (Require Response)

- `Handshake` - Initial connection and capability negotiation
- `PreAction` - Before any agent action (tool call, API request, etc.)
- `PrePrompt` - Before sending prompt to LLM
- `Query` - Agent requests guidance from harness

### Fire-and-Forget Events (Notifications)

- `PostAction` - After action completes
- `PostResponse` - After receiving LLM response
- `SessionStart` / `SessionEnd` - Session lifecycle
- `Error` - Agent encountered an error
- `Heartbeat` - Periodic keepalive

## Decision Types

The harness can return different decision types:

- `Allow` - Proceed with the action as-is
- `Block` - Cancel the action, return error to agent
- `Modify` - Proceed with modified payload
- `Defer` - Ask agent to retry after delay
- `Escalate` - Forward to human operator for approval

## Advanced Features

### Batch Processing

Send multiple events in a single request:

```rust
let events = vec![event1, event2, event3];
let batch_response = client.send_batch(events).await?;
```

### Query Support

Agent can query the harness for guidance:

```rust
let response = client.query(
    "should_i_proceed",
    serde_json::json!({
        "question": "Should I delete this file?",
        "file_path": "/workspace/important.txt"
    })
).await?;
```

## Building Examples

```bash
# Build all examples
cargo build --examples --all-features

# Build specific transport examples
cargo build --example http_client --features http
cargo build --example websocket_client --features websocket

# Run with logging
RUST_LOG=info cargo run --example http_server --features http
```

## Testing

```bash
# Run tests
cargo test --all-features

# Run specific transport tests
cargo test --features http
cargo test --features websocket
```

## Dependencies

- **stdio**: No additional dependencies (default)
- **http**: Requires `reqwest`, `axum`, `tower`, `tower-http`
- **websocket**: Requires `tokio-tungstenite`, `futures-util`
- **grpc**: Requires `tonic`, `prost` (not yet implemented)
- **unix-socket**: Requires `tokio/net` (not yet implemented)

## Next Steps

- Implement gRPC transport for high-performance scenarios
- Implement Unix socket transport for local IPC
- Add more authentication methods (mTLS, OAuth)
- Add metrics and observability examples
- Add integration tests
