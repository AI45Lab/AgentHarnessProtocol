# Agent Harness Protocol (AHP) v2.3

**A transport-agnostic supervision protocol for autonomous AI agents.**

AHP separates agent execution from policy enforcement. An agent emits structured
events at meaningful control points, a harness evaluates those events, and the
agent applies the returned decision before continuing.

The first principle is simple: **the component that acts should not be the only
component that decides whether the action is safe, useful, authorized, or
well-contextualized.**

## Why AHP Exists

Agent frameworks expose different hook systems, callback shapes, and transport
assumptions. That makes policies hard to reuse:

- A safety policy written for one framework usually cannot supervise another.
- Audit, approval, memory, and context logic gets duplicated per runtime.
- Operational controls become coupled to the agent implementation.

AHP defines a small shared contract between an agent and a harness:

1. The agent sends events before or after meaningful work.
2. Blocking events are JSON-RPC requests and must receive a decision.
3. Fire-and-forget events are JSON-RPC notifications and do not block execution.
4. The harness owns policy, enrichment, approval, audit, and backpressure logic.
5. The agent owns enforcement of returned decisions.

## Design Principles

- **Protocol before implementation** — JSON-RPC message shape is the contract;
  Rust is one implementation.
- **Transport independence** — stdio, HTTP, WebSocket, and Unix sockets carry
  the same protocol semantics.
- **Fail closed for control paths** — handler failures in batch decisions become
  `Block` decisions, not silent allows.
- **Explicit synchronization** — blocking events use requests; non-blocking
  telemetry uses notifications.
- **Typed decisions where shape matters** — context, memory, planning,
  reasoning, rate limit, confirmation, idle, and intent detection use specialized
  decision payloads.
- **Generic decisions where policy is enough** — action and prompt gates use the
  generic `Decision` shape.
- **Capability negotiation first** — clients handshake before sending events.
- **Bounded recursion and batching** — harness configuration can limit event
  depth and batch size.

## Current Features

- JSON-RPC 2.0 protocol messages.
- Handshake with major-version compatibility checks.
- Rust client with stdio, HTTP, WebSocket, and Unix socket transports.
- Rust server dispatch for requests and notifications.
- Optional API key and bearer-token auth for HTTP and WebSocket clients.
- Transport timeout configuration.
- Response validation for JSON-RPC version, request id, errors, and missing
  results.
- Typed harness handlers for specialized event families.
- Batch requests for generic-decision events.
- Server builder methods for advertised harness info and validation limits.
- gRPC feature placeholder reserved for future implementation.

## Protocol Version

- Protocol version: `2.3`
- Crate version: `2.3.1`
- Rust crate: `a3s-ahp`
- Repository: `https://github.com/A3S-Lab/AgentHarnessProtocol`

The crate can receive patch releases without changing the protocol version. A
protocol major-version mismatch is rejected during handshake.

## Message Model

AHP uses JSON-RPC 2.0.

### Blocking Event Request

```json
{
  "jsonrpc": "2.0",
  "id": "req-123",
  "method": "ahp/event",
  "params": {
    "event_type": "pre_action",
    "session_id": "sess-abc",
    "agent_id": "agent-xyz",
    "timestamp": "2026-05-01T00:00:00Z",
    "depth": 0,
    "payload": {
      "tool_name": "bash",
      "arguments": {
        "command": "cargo test"
      }
    }
  }
}
```

### Decision Response

```json
{
  "jsonrpc": "2.0",
  "id": "req-123",
  "result": {
    "decision": "allow"
  }
}
```

### Fire-And-Forget Notification

```json
{
  "jsonrpc": "2.0",
  "method": "ahp/event",
  "params": {
    "event_type": "post_action",
    "session_id": "sess-abc",
    "agent_id": "agent-xyz",
    "timestamp": "2026-05-01T00:00:02Z",
    "depth": 0,
    "payload": {
      "status": "ok"
    }
  }
}
```

## Methods

| Method | Direction | Purpose |
| --- | --- | --- |
| `ahp/handshake` | Agent to harness | Negotiate protocol compatibility, capabilities, and harness limits. |
| `ahp/event` | Agent to harness | Send one event as a request or notification depending on event type. |
| `ahp/query` | Agent to harness | Ask the harness for extra information. |
| `ahp/batch` | Agent to harness | Send multiple generic-decision events in one request. |

## Event Types

| Event | Timing | Blocking | Decision Shape | Batchable |
| --- | --- | --- | --- | --- |
| `pre_action` | Before a tool/action executes | Yes | `Decision` | Yes |
| `post_action` | After a tool/action completes | No | Notification | Yes |
| `pre_prompt` | Before an LLM request | Yes | `Decision` | Yes |
| `post_response` | After an LLM response | No | Notification | Yes |
| `session_start` | Session begins | No | Notification | Yes |
| `session_end` | Session ends | No | Notification | Yes |
| `error` | Operation failed | No | Notification | Yes |
| `heartbeat` | Periodic liveness/status | No | Notification | Yes |
| `success` | Operation succeeded | No | Notification | Yes |
| `idle` | Agent asks whether background work should run | Yes | `IdleDecision` | No |
| `intent_detection` | Classify user intent before deeper context work | Yes | `IntentDetectionDecision` | No |
| `context_perception` | Retrieve or inject workspace context | Yes | `ContextPerceptionDecision` | No |
| `memory_recall` | Retrieve facts from memory | Yes | `MemoryRecallDecision` | No |
| `planning` | Select or modify planning strategy | Yes | `PlanningDecision` | No |
| `reasoning` | Provide reasoning hints or block reasoning | Yes | `ReasoningDecision` | No |
| `rate_limit` | Decide backpressure after a limit is hit | Yes | `RateLimitDecision` | No |
| `confirmation` | Ask for approval, rejection, or escalation | Yes | `ConfirmationDecision` | No |

`handshake` and `query` are represented in `EventType` for taxonomy purposes,
but normal clients should use the dedicated `ahp/handshake` and `ahp/query`
methods.

## Decision Shapes

### Generic `Decision`

Generic decisions are used by ordinary action and prompt gates.

| Decision | Meaning |
| --- | --- |
| `allow` | Continue, optionally with metadata or modified payload. |
| `block` | Stop and return a reason. |
| `modify` | Continue with harness-modified parameters. |
| `defer` | Retry later. |
| `escalate` | Forward to a human or external approval path. |

### Specialized Decisions

Some harness points need richer return types than a generic allow/block:

- `IdleDecision` can allow or defer idle/background work.
- `IntentDetectionDecision` returns detected intent, confidence, and target
  hints.
- `ContextPerceptionDecision` injects facts, file snippets, project summaries,
  knowledge, or suggestions.
- `MemoryRecallDecision` injects recalled facts.
- `PlanningDecision` selects a planning strategy or modifies the task.
- `ReasoningDecision` returns reasoning hints or blocks reasoning.
- `RateLimitDecision` retries, queues, or skips.
- `ConfirmationDecision` approves, rejects, or escalates.

Specialized events must be sent individually with `send_typed_event` or
equivalent JSON-RPC calls. They are intentionally excluded from batch requests
because a batch response contains `Vec<Decision>`.

## Client Lifecycle

1. Create an `AhpClient` with a transport.
2. Run `handshake` with agent capabilities.
3. Send blocking events with `send_event_decision` or `send_typed_event`.
4. Send fire-and-forget events through `send_event` for non-blocking event
   types.
5. Use `send_batch` only for generic-decision event types.
6. Close the client when done.

The Rust client validates:

- JSON-RPC version is `2.0`.
- Response id matches the request id.
- Error responses become `AhpError::Protocol`.
- Missing results are rejected.
- Events and batches require a completed handshake.
- Batch response decision count must match request event count.

## Rust Client Example

```rust
use a3s_ahp::{AhpClient, Decision, EventType, Transport};

async fn run_agent() -> a3s_ahp::Result<()> {
    let client = AhpClient::new(Transport::Stdio {
        program: "python3".into(),
        args: vec!["harness.py".into()],
    })
    .await?;

    client
        .handshake(vec![
            "pre_action".to_string(),
            "post_action".to_string(),
        ])
        .await?;

    let decision = client
        .send_event_decision(
            EventType::PreAction,
            serde_json::json!({
                "tool_name": "bash",
                "arguments": {
                    "command": "cargo test --all-features"
                }
            }),
        )
        .await?;

    match decision {
        Decision::Allow { .. } => {
            // Execute the action.
        }
        Decision::Block { reason, .. } => {
            // Surface the policy reason to the caller.
            eprintln!("blocked: {reason}");
        }
        Decision::Modify {
            modified_payload, ..
        } => {
            // Execute using modified_payload.
            println!("modified: {modified_payload}");
        }
        Decision::Defer { retry_after_ms, .. } => {
            // Retry later.
            println!("retry after {retry_after_ms}ms");
        }
        Decision::Escalate { reason, .. } => {
            // Hand off to a human approval path.
            eprintln!("escalated: {reason}");
        }
    }

    client.close().await?;
    Ok(())
}
```

## Typed Event Example

```rust
use a3s_ahp::{AhpClient, ContextPerceptionDecision, EventType};

async fn inject_context(client: &AhpClient) -> a3s_ahp::Result<()> {
    let decision: ContextPerceptionDecision = client
        .send_typed_event(
            EventType::ContextPerception,
            serde_json::json!({
                "session_id": "session-1",
                "intent": "understand",
                "target": {
                    "location": {
                        "path": ".",
                        "location_type": "workspace"
                    }
                },
                "context": {
                    "workspace": "/repo",
                    "query": "How is the protocol structured?"
                }
            }),
        )
        .await?;

    match decision {
        ContextPerceptionDecision::Allow {
            injected_context, ..
        } => {
            println!("facts: {}", injected_context.facts.len());
        }
        ContextPerceptionDecision::Block { reason, .. } => {
            eprintln!("context blocked: {reason}");
        }
        ContextPerceptionDecision::Refine { scope_hints, .. } => {
            println!("refine with hints: {scope_hints:?}");
        }
    }

    Ok(())
}
```

## Server Example

```rust
use a3s_ahp::{
    AhpEvent, AhpServer, Decision, EventHandler, HarnessConfig, Result,
};
use async_trait::async_trait;
use std::sync::Arc;

struct PolicyHarness;

#[async_trait]
impl EventHandler for PolicyHarness {
    async fn handle_event(&self, event: &AhpEvent) -> Result<Decision> {
        if event.payload["tool_name"] == "rm" {
            return Ok(Decision::Block {
                reason: "destructive command requires approval".to_string(),
                metadata: None,
            });
        }

        Ok(Decision::Allow {
            modified_payload: None,
            metadata: None,
        })
    }
}

async fn run_harness() -> Result<()> {
    let server = AhpServer::new(Arc::new(PolicyHarness))
        .with_capabilities(["pre_action", "post_action", "batch"])
        .with_config(HarnessConfig {
            timeout_ms: Some(10_000),
            batch_size: Some(100),
            max_depth: Some(10),
        });

    server.run_stdio().await
}
```

`AhpServer` validates event depth, rejects blocking events sent as
notifications, rejects fire-and-forget events sent as requests, and rejects
batch entries that require specialized decision payloads.

## Transports

| Transport | Feature | Status | Notes |
| --- | --- | --- | --- |
| stdio | `stdio` | Implemented | Default feature; useful for local subprocess harnesses. |
| HTTP | `http` | Implemented | Supports API key and bearer auth. |
| WebSocket | `websocket` | Implemented | Supports API key and bearer auth via URL query parameters. |
| Unix socket | `unix-socket` | Implemented | Local IPC on Unix platforms. |
| gRPC | `grpc` | Reserved | Feature placeholder; not included in `all-transports`. |

Feature examples:

```bash
cargo add a3s-ahp
cargo add a3s-ahp --features http
cargo add a3s-ahp --features all-transports
```

## Transport Configuration

```rust
use a3s_ahp::{AhpClient, Transport, TransportConfig};

async fn connect() -> a3s_ahp::Result<AhpClient> {
    AhpClient::new_with_config(
        Transport::Http {
            url: "https://harness.example.com/ahp".to_string(),
            auth: None,
        },
        TransportConfig {
            timeout_ms: Some(5_000),
        },
    )
    .await
}
```

The same timeout configuration is applied consistently across implemented
transports where request/response waiting is involved.

## Authentication

```rust
use a3s_ahp::{AuthConfig, Transport};

let http = Transport::Http {
    url: "https://harness.example.com/ahp".to_string(),
    auth: Some(AuthConfig::bearer("token")),
};

let websocket = Transport::WebSocket {
    url: "wss://harness.example.com/ahp".to_string(),
    auth: Some(AuthConfig::api_key("key")),
};
```

## Batching Rules

Batching exists to amortize transport overhead for homogeneous generic policy
checks. It is not a multiplexing mechanism for every event type.

Rules:

- `ahp/batch` returns `BatchResponse { decisions: Vec<Decision> }`.
- Event order is preserved.
- The number of returned decisions must equal the number of submitted events.
- Server-side handler failures become `Decision::Block`.
- Specialized decision events are rejected.
- `handshake` and `query` are rejected in batches.
- Batch size can be limited by `HarnessConfig.batch_size`.

## Depth And Recursion

Agents can emit AHP events while handling another AHP decision. The `depth`
field makes that recursion visible. Harnesses can advertise and enforce
`HarnessConfig.max_depth` to prevent uncontrolled loops.

## Repository Layout

```text
ahp/
├── src/
│   ├── lib.rs
│   ├── auth.rs
│   ├── client.rs
│   ├── error.rs
│   ├── protocol.rs
│   ├── protocol/
│   │   ├── core.rs
│   │   ├── context.rs
│   │   ├── events.rs
│   │   └── json_rpc.rs
│   ├── server.rs
│   ├── server/
│   │   └── tests.rs
│   └── transport/
│       ├── http.rs
│       ├── stdio.rs
│       ├── unix_socket.rs
│       └── websocket.rs
├── examples/
└── Cargo.toml
```

## Development

Run checks from this crate directory:

```bash
cargo fmt --all -- --check
cargo check --all-features
cargo check --no-default-features
cargo check --features all-transports
cargo test --all-features
```

## License

MIT
