# AHP Examples

This directory contains examples demonstrating different transport layers and usage patterns for the Agent Harness Protocol (AHP) v2.3.

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

Send multiple generic-decision events in a single request. Events that return
specialized decisions, such as `context_perception`, `memory_recall`, `planning`,
`reasoning`, `rate_limit`, `confirmation`, `intent_detection`, and `idle`, must
be sent individually.

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
- **grpc**: Requires `tonic`, `prost`; available as a feature placeholder, not included in `all-transports` until implemented
- **unix-socket**: Requires `tokio/net`

## Next Steps

- Implement gRPC transport for high-performance scenarios
- Add more authentication methods (mTLS, OAuth)
- Add metrics and observability examples
- Add integration tests

## TypeScript Examples

### HTTP Server

```bash
# Install dependencies
cd examples
npm install

# Run HTTP server
npm run http
# Or directly:
npx ts-node http_server.ts

# Test with curl
curl -X POST http://localhost:8080/ahp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": "1",
    "method": "ahp/handshake",
    "params": {
      "protocol_version": "2.0",
      "agent_info": {
        "framework": "a3s-code",
        "version": "1.0.0"
      }
    }
  }'
```

### WebSocket Server

```bash
# Run WebSocket server
npm run websocket
# Or directly:
npx ts-node websocket_server.ts

# Test with wscat
npm install -g wscat
wscat -c ws://localhost:8081/ahp
> {"jsonrpc":"2.0","id":"1","method":"ahp/handshake","params":{"protocol_version":"2.0","agent_info":{"framework":"test","version":"1.0.0"}}}
```

## Python Examples

### HTTP Server

```bash
# Install dependencies
pip install flask

# Run HTTP server
python http_server.py

# Or with custom port
python http_server.py 8080

# Test with curl
curl -X POST http://localhost:8080/ahp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": "1",
    "method": "ahp/handshake",
    "params": {
      "protocol_version": "2.0",
      "agent_info": {
        "framework": "a3s-code",
        "version": "1.0.0"
      }
    }
  }'
```

### WebSocket Server

```bash
# Install dependencies
pip install websockets

# Run WebSocket server
python websocket_server.py

# Or with custom port
python websocket_server.py 8081

# Test with Python client
python -c "
import asyncio
import websockets
import json

async def test():
    async with websockets.connect('ws://localhost:8081/ahp') as ws:
        msg = {
            'jsonrpc': '2.0',
            'id': '1',
            'method': 'ahp/handshake',
            'params': {
                'protocol_version': '2.0',
                'agent_info': {'framework': 'test', 'version': '1.0.0'}
            }
        }
        await ws.send(json.dumps(msg))
        response = await ws.recv()
        print(response)

asyncio.run(test())
"
```

## TypeScript/Python Server Features

All TypeScript and Python example servers implement:

- **Handshake**: Capability negotiation with protocol version check
- **Event Handling**: Pre-action, post-action, pre-prompt events
- **Depth-Aware Policy**: Stricter rules for nested agents (depth > 0)
- **Query Support**: Interactive decision-making with alternatives
- **Batch Processing**: Handle multiple events in a single request
- **Dangerous Command Detection**: Block destructive operations
- **Comprehensive Logging**: Track all events and decisions

## Production Deployment

For production use:

**TypeScript:**
```bash
# Compile TypeScript
npx tsc http_server.ts

# Run with PM2
npm install -g pm2
pm2 start http_server.js -i 4

# Or with Node.js cluster
node http_server.js
```

**Python:**
```bash
# Run with Gunicorn (HTTP)
pip install gunicorn
gunicorn -w 4 -b 0.0.0.0:8080 http_server:app

# Run with systemd (WebSocket)
sudo systemctl enable ahp-websocket.service
sudo systemctl start ahp-websocket.service
```
