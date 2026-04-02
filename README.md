# Agent Harness Protocol (AHP) v2.1.0

**A universal, transport-agnostic protocol for supervising autonomous AI agents**

## Table of Contents

1. [Overview](#overview)
2. [Design Principles](#design-principles)
3. [Architecture](#architecture)
4. [Protocol Specification](#protocol-specification)
5. [Transport Layers](#transport-layers)
6. [Security Model](#security-model)
7. [Implementation Guide](#implementation-guide)
8. [Reference Implementations](#reference-implementations)
9. [Migration from v1.x](#migration-from-v1x)

---

## Overview

The Agent Harness Protocol (AHP) is a **framework-agnostic, language-neutral protocol** for supervising, controlling, and auditing autonomous AI agents. Unlike traditional monitoring solutions that are tightly coupled to specific frameworks, AHP provides a universal interface that works with any agent system — from coding assistants to autonomous research agents to robotic control systems.

### What is a "Harness"?

In the context of AI agents, a **harness** is an external supervision layer that:

- **Intercepts** agent actions before execution (pre-execution hooks)
- **Observes** agent behavior and outputs (post-execution hooks)
- **Enforces** policies and constraints (blocking, rate-limiting, resource quotas)
- **Audits** all agent activities for compliance and debugging
- **Modifies** agent behavior dynamically (prompt injection, parameter tuning)

The harness sits **between** the agent and its environment, acting as a transparent proxy that can inspect, modify, or block any interaction.

### Why AHP?

**Problem:** Existing agent frameworks (LangChain, AutoGPT, A3S Code, etc.) each have their own proprietary hooks, callbacks, or plugin systems. This creates:

- **Vendor lock-in** — Policies written for one framework don't work with another
- **Duplication** — The same safety rules must be reimplemented for each framework
- **Limited interoperability** — Cross-framework orchestration is nearly impossible
- **Fragmented tooling** — Monitoring, debugging, and compliance tools are framework-specific

**Solution:** AHP defines a **universal protocol** that any agent framework can implement. Once a framework supports AHP, it gains access to a shared ecosystem of:

- **Policy engines** (compliance, safety, resource management)
- **Monitoring tools** (observability, tracing, alerting)
- **Audit systems** (logging, replay, forensics)
- **Testing harnesses** (simulation, fuzzing, regression testing)

### Key Features

- **Framework-agnostic** — Works with any agent system (coding agents, research agents, robotics, etc.)
- **Language-neutral** — Harness servers can be written in any language (Python, Go, Rust, Java, etc.)
- **Transport-flexible** — Supports stdio, HTTP, WebSocket, gRPC, Unix sockets, and custom transports
- **Bidirectional** — Agents can query the harness for guidance, not just receive commands
- **Stateful** — Harness can maintain session state, track history, and make context-aware decisions
- **Extensible** — New event types, actions, and metadata can be added without breaking compatibility
- **Secure** — Built-in authentication, encryption, and sandboxing support
- **Observable** — Full tracing, metrics, and logging for all interactions

---

## Design Principles

AHP v2.1 is built on the following first principles:

### 1. **Separation of Concerns**

The protocol strictly separates:

- **Agent logic** (what the agent wants to do)
- **Policy enforcement** (what the agent is allowed to do)
- **Execution environment** (how actions are performed)

This separation enables:
- Independent development and testing of each layer
- Reusable policy engines across different agent types
- Flexible deployment models (local, remote, distributed)

### 2. **Protocol-First Design**

AHP is defined as a **protocol specification**, not a library or framework. This means:

- The protocol is documented independently of any implementation
- Multiple implementations can coexist and interoperate
- Implementations can be verified for compliance via test suites
- The protocol can evolve without breaking existing implementations

### 3. **Transport Agnosticism**

The protocol is **transport-layer independent**. The same message format works over:

- **stdio** (for local child processes)
- **HTTP/SSE** (for web-based harnesses)
- **WebSocket** (for bidirectional streaming)
- **gRPC** (for high-performance RPC)
- **Unix sockets** (for local IPC)
- **Custom transports** (ZeroMQ, NATS, Kafka, etc.)

This flexibility allows deployment in diverse environments:
- Local development (stdio)
- Cloud services (HTTP/gRPC)
- Edge devices (Unix sockets)
- Distributed systems (message queues)

### 4. **Extensibility Without Breaking Changes**

The protocol uses a **capability negotiation** mechanism:

1. Agent and harness exchange capability declarations at connection time
2. Each side declares which event types, actions, and features it supports
3. Unknown capabilities are ignored (forward compatibility)
4. New features can be added without breaking old implementations

### 5. **Security by Default**

Security is not an afterthought:

- **Authentication** — Mutual TLS, API keys, or custom auth mechanisms
- **Authorization** — Fine-grained permissions per event type
- **Encryption** — TLS for network transports, encrypted stdio for local processes
- **Sandboxing** — Harness can enforce resource limits, network isolation, filesystem restrictions
- **Audit trail** — All interactions are logged with cryptographic signatures

### 6. **Performance and Scalability**

The protocol is designed for high-throughput scenarios:

- **Batching** — Multiple events can be sent in a single message
- **Pipelining** — Requests can be sent before previous responses arrive
- **Async notifications** — Fire-and-forget events don't block the agent
- **Streaming** — Large payloads (logs, traces) can be streamed incrementally
- **Caching** — Harness can cache decisions for repeated patterns

---

## Architecture

### System Model

```
┌─────────────────────────────────────────────────────────────┐
│                        Agent Host                            │
│  ┌────────────────────────────────────────────────────────┐ │
│  │  Agent Runtime (any framework: A3S Code, LangChain, …) │ │
│  │  ┌──────────────────────────────────────────────────┐  │ │
│  │  │  AHP Client (implements protocol)                 │  │ │
│  │  │  • Intercepts agent actions                       │  │ │
│  │  │  • Sends events to harness                        │  │ │
│  │  │  • Enforces harness decisions                     │  │ │
│  │  └──────────────────────────────────────────────────┘  │ │
│  └────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
                            │
                            │ AHP Protocol
                            │ (stdio / HTTP / WebSocket / gRPC / …)
                            │
┌─────────────────────────────────────────────────────────────┐
│                      Harness Server                          │
│  ┌────────────────────────────────────────────────────────┐ │
│  │  AHP Server (implements protocol)                      │ │
│  │  • Receives events from agents                         │ │
│  │  • Applies policies and rules                          │ │
│  │  • Returns decisions (allow/block/modify)              │ │
│  │  • Logs and audits all interactions                    │ │
│  └────────────────────────────────────────────────────────┘ │
│  ┌────────────────────────────────────────────────────────┐ │
│  │  Policy Engine                                         │ │
│  │  • Rule-based policies (regex, AST analysis, …)       │ │
│  │  │  • LLM-based policies (semantic analysis)            │ │
│  │  • Stateful policies (rate limiting, quotas, …)       │ │
│  └────────────────────────────────────────────────────────┘ │
│  ┌────────────────────────────────────────────────────────┐ │
│  │  Audit & Observability                                 │ │
│  │  • Event logging (structured logs, traces)             │ │
│  │  • Metrics (Prometheus, StatsD, …)                     │ │
│  │  • Alerting (PagerDuty, Slack, …)                      │ │
│  └────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

### Event Flow

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              Agent (A3S Code, etc.)                          │
│  ┌─────────┐    ┌─────────┐    ┌─────────┐    ┌─────────┐    ┌─────────┐  │
│  │ Session │───▶│ PrePrompt│───▶│ LLM Call│───▶│PostResp │───▶│   ...   │  │
│  │  Start  │    │         │    │         │    │         │    │         │  │
│  └────┬────┘    └────┬────┘    └────┬────┘    └────┬────┘    └────┬────┘  │
│       │               │               │               │               │       │
│       ▼               ▼               ▼               ▼               ▼       │
│  ┌─────────────────────────────────────────────────────────────────────┐     │
│  │                      AhpHookExecutor                                 │     │
│  │  • Maps hook events to AHP events                                   │     │
│  │  • Sends events to harness (blocking or fire-and-forget)            │     │
│  │  • Enforces decisions (Allow/Block/Modify/Defer)                   │     │
│  │  • Tracks idle state & fires idle events                           │     │
│  └─────────────────────────────────────────────────────────────────────┘     │
└─────────────────────────────────────────────────────────────────────────────┘
                                     │
                                     ▼
                         ┌───────────────────────┐
                         │   AHP Transport       │
                         │   (stdio/http/ws/...) │
                         └───────────────────────┘
                                     │
                                     ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                           Harness Server                                     │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐      │
│  │  Policy 1   │  │  Policy 2   │  │  Policy N   │  │  LLM-based  │      │
│  │  (regex)    │  │  (allowlist)│  │  (rate)     │  │  (semantic) │      │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘      │
│         └──────────────────┴──────────────────┴────────────────┘            │
│                                    │                                         │
│                                    ▼                                         │
│                          ┌─────────────────┐                               │
│                          │  Decision Engine │                               │
│                          │  Allow / Block   │                               │
│                          │  Modify / Defer  │                               │
│                          │  Escalate        │                               │
│                          └─────────────────┘                               │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Idle Detection Flow

```
Agent Idle                    AHP Client                         Harness
   │                              │                                  │
   │  [No activity for N seconds] │                                  │
   │─────────────────────────────▶│                                  │
   │                              │  IdleEvent {                      │
   │                              │    idle_duration_ms: 10000,       │
   │                              │    idle_reason: "no_activity",    │
   │                              │    suggested_action: "dream"      │
   │                              │  } + EventContext {               │
   │                              │    capabilities: {...}            │
   │                              │  }                               │
   │                              │─────────────────────────────────▶│
   │                              │                                  │
   │                              │    IdleDecision::Allow           │
   │                              │◀─────────────────────────────────│
   │                              │                                  │
   │  [Background consolidation]  │  [Continue to dream]             │
   │◀─────────────────────────────│                                  │
```

### Context Propagation

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              Agent Runtime                                   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ AhpHookExecutor                                                      │   │
│  │                                                                       │   │
│  │  capabilities: {                     EventContext {                   │   │
│  │    memory_search: {...},      ▶      recent_facts: [...],            │   │
│  │    session_info: {...},      ▶      memory_summary: {...},           │   │
│  │    cross_session: {...},     ▶      session_stats: {...},           │   │
│  │    custom_ai_tool: {...}     ▶      current_task: "implement X",   │   │
│  │  }                               capabilities: {...}                 │   │
│  │                                     }                                 │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                      │                                      │
│                                      ▼                                      │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                           AhpEvent                                    │   │
│  │  {                                                                    │   │
│  │    event_type: "pre_action",                                         │   │
│  │    session_id: "...",                                                │   │
│  │    agent_id: "...",                                                   │   │
│  │    timestamp: "...",                                                  │   │
│  │    depth: 0,                                                         │   │
│  │    payload: {...},                                                    │   │
│  │    context: { EventContext }  ◀── All驾驭 points receive this       │   │
│  │  }                                                                    │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Component Roles

**Agent Host:**
- Runs the AI agent (LLM-based, rule-based, or hybrid)
- Embeds an AHP client that intercepts agent actions
- Enforces harness decisions before executing actions
- Can be any language/framework that implements the AHP client interface

**Harness Server:**
- Receives events from one or more agent hosts
- Applies policies to decide whether to allow, block, or modify actions
- Maintains state across multiple agents and sessions
- Provides observability and audit capabilities
- Can be deployed locally (same machine) or remotely (cloud service)

**Policy Engine:**
- Pluggable component that evaluates events against rules
- Can use static rules (regex, allowlists), dynamic rules (LLM-based), or hybrid approaches
- Supports multiple policy languages (Rego, CEL, Python, custom DSLs)

**Audit & Observability:**
- Records all events and decisions for compliance and debugging
- Exports metrics for monitoring (latency, block rate, error rate)
- Integrates with existing observability stacks (Prometheus, Grafana, ELK, etc.)

---

## Protocol Specification

### Message Format

AHP uses **JSON-RPC 2.0** as the base message format. All messages are JSON objects with the following structure:

```json
{
  "jsonrpc": "2.0",
  "id": "unique-request-id",
  "method": "ahp/event",
  "params": {
    "event_type": "pre_action",
    "session_id": "session-uuid",
    "agent_id": "agent-uuid",
    "timestamp": "2026-03-10T12:34:56.789Z",
    "depth": 0,
    "payload": { ... },
    "metadata": { ... }
  }
}
```

**Fields:**

- `jsonrpc`: Always `"2.0"` (JSON-RPC version)
- `id`: Unique identifier for request-response pairing. Omitted for notifications (fire-and-forget).
- `method`: Always `"ahp/event"` for agent events. Other methods: `"ahp/handshake"`, `"ahp/query"`, `"ahp/control"`.
- `params`: Event-specific parameters (see below)

### Event Types

AHP defines a core set of event types. Implementations can extend this list via capability negotiation.

#### Core Event Types

| Event Type | Direction | Blocking | Description |
|------------|-----------|----------|-------------|
| `handshake` | Agent → Harness | Yes | Initial connection, capability negotiation |
| `pre_action` | Agent → Harness | Yes | Before any agent action (tool call, API request, etc.) |
| `post_action` | Agent → Harness | No | After action completes (success or failure) |
| `pre_prompt` | Agent → Harness | Yes | Before sending prompt to LLM |
| `post_response` | Agent → Harness | No | After receiving LLM response |
| `session_start` | Agent → Harness | No | New agent session begins |
| `session_end` | Agent → Harness | No | Agent session ends |
| `error` | Agent → Harness | No | Agent encountered an error |
| `query` | Agent → Harness | Yes | Agent requests guidance from harness |
| `heartbeat` | Agent → Harness | No | Periodic keepalive signal |

#### Extended Event Types (Optional)

Frameworks can define custom event types:

- `pre_tool_use` / `post_tool_use` (for tool-based agents)
- `pre_code_execution` / `post_code_execution` (for coding agents)
- `pre_file_write` / `post_file_write` (for filesystem operations)
- `pre_network_request` / `post_network_request` (for network operations)
- `pre_subprocess_spawn` / `post_subprocess_spawn` (for process management)

### Event Payload Structure

Each event type has a specific payload structure. Here are the core types:

#### `handshake`

```json
{
  "method": "ahp/handshake",
  "params": {
    "protocol_version": "2.1",
    "agent_info": {
      "framework": "a3s-code",
      "version": "1.3.1",
      "capabilities": ["pre_action", "post_action", "pre_prompt", "query"]
    },
    "session_id": "session-uuid",
    "agent_id": "agent-uuid"
  }
}
```

**Response:**

```json
{
  "jsonrpc": "2.0",
  "id": "handshake-1",
  "result": {
    "protocol_version": "2.1",
    "harness_info": {
      "name": "compliance-harness",
      "version": "1.0.0",
      "capabilities": ["pre_action", "post_action", "pre_prompt", "query", "batch"]
    },
    "session_token": "auth-token-xyz",
    "config": {
      "timeout_ms": 10000,
      "batch_size": 100
    }
  }
}
```

#### `pre_action`

```json
{
  "method": "ahp/event",
  "id": "req-123",
  "params": {
    "event_type": "pre_action",
    "session_id": "session-uuid",
    "agent_id": "agent-uuid",
    "timestamp": "2026-03-10T12:34:56.789Z",
    "depth": 0,
    "payload": {
      "action_type": "tool_call",
      "tool_name": "bash",
      "arguments": {
        "command": "ls -la /etc"
      },
      "context": {
        "working_directory": "/workspace",
        "environment": { "USER": "agent" },
        "recent_actions": ["read_file", "write_file"]
      }
    },
    "metadata": {
      "trace_id": "trace-xyz",
      "parent_span_id": "span-abc"
    }
  }
}
```

**Response:**

```json
{
  "jsonrpc": "2.0",
  "id": "req-123",
  "result": {
    "decision": "allow",
    "reason": null,
    "modified_payload": null,
    "metadata": {
      "policy_version": "v1.2.3",
      "rules_applied": ["allow_read_only_commands"]
    }
  }
}
```

**Decision Types:**

- `allow` — Proceed with the action as-is
- `block` — Cancel the action, return error to agent
- `modify` — Proceed with modified payload (see `modified_payload`)
- `defer` — Ask agent to retry after delay (see `retry_after_ms`)
- `escalate` — Forward to human operator for approval

#### `post_action`

```json
{
  "method": "ahp/event",
  "params": {
    "event_type": "post_action",
    "session_id": "session-uuid",
    "agent_id": "agent-uuid",
    "timestamp": "2026-03-10T12:34:57.123Z",
    "depth": 0,
    "payload": {
      "action_type": "tool_call",
      "tool_name": "bash",
      "arguments": {
        "command": "ls -la /etc"
      },
      "result": {
        "status": "success",
        "output": "total 1234\ndrwxr-xr-x  10 root  wheel  320 Mar  9 12:00 .\n...",
        "exit_code": 0,
        "duration_ms": 45
      }
    },
    "metadata": {
      "trace_id": "trace-xyz",
      "span_id": "span-def"
    }
  }
}
```

No response expected (notification).

#### `query`

Agents can query the harness for guidance:

```json
{
  "method": "ahp/query",
  "id": "query-456",
  "params": {
    "session_id": "session-uuid",
    "agent_id": "agent-uuid",
    "query_type": "should_i_proceed",
    "payload": {
      "question": "Should I delete this file?",
      "file_path": "/workspace/important.txt",
      "context": {
        "file_size": 1024,
        "last_modified": "2026-03-09T10:00:00Z"
      }
    }
  }
}
```

**Response:**

```json
{
  "jsonrpc": "2.0",
  "id": "query-456",
  "result": {
    "answer": "no",
    "reason": "File is marked as important in project metadata",
    "alternatives": ["Move to trash", "Create backup first"]
  }
}
```

### Batching

For high-throughput scenarios, multiple events can be batched:

```json
{
  "jsonrpc": "2.0",
  "method": "ahp/batch",
  "id": "batch-789",
  "params": {
    "events": [
      { "event_type": "pre_action", "payload": { ... } },
      { "event_type": "pre_action", "payload": { ... } },
      { "event_type": "pre_action", "payload": { ... } }
    ]
  }
}
```

**Response:**

```json
{
  "jsonrpc": "2.0",
  "id": "batch-789",
  "result": {
    "decisions": [
      { "decision": "allow" },
      { "decision": "block", "reason": "Dangerous command" },
      { "decision": "allow" }
    ]
  }
}
```

---

## Transport Layers

AHP is transport-agnostic. The protocol defines the message format, not how messages are delivered.

### 1. stdio Transport (Default)

**Use case:** Local harness server spawned as child process

**Mechanism:**
- Agent spawns harness as subprocess
- Messages sent as newline-delimited JSON over stdin/stdout
- Harness logs to stderr (not mixed with protocol messages)

**Pros:**
- Simple, no network configuration
- Process isolation
- Works on any OS

**Cons:**
- Single harness per agent
- No remote deployment
- Limited scalability

**Example:**

```bash
# Agent spawns harness
harness_process = subprocess.Popen(
    ["python3", "harness.py"],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE
)

# Send event
event = {"jsonrpc": "2.0", "id": "1", "method": "ahp/event", ...}
harness_process.stdin.write(json.dumps(event).encode() + b"\n")
harness_process.stdin.flush()

# Read response
response_line = harness_process.stdout.readline()
response = json.loads(response_line)
```

### 2. HTTP/SSE Transport

**Use case:** Remote harness server, web-based deployment

**Mechanism:**
- Agent sends events via HTTP POST to harness endpoint
- Harness returns decisions in HTTP response
- Long-lived connections use Server-Sent Events (SSE) for notifications

**Pros:**
- Remote deployment (cloud, edge)
- Load balancing, horizontal scaling
- Standard web infrastructure (nginx, CDN, etc.)

**Cons:**
- Higher latency than local transports
- Requires network configuration

**Example:**

```python
import requests

# Send event
event = {"jsonrpc": "2.0", "id": "1", "method": "ahp/event", ...}
response = requests.post(
    "https://harness.example.com/ahp",
    json=event,
    headers={"Authorization": "Bearer token-xyz"}
)
decision = response.json()
```

### 3. WebSocket Transport

**Use case:** Bidirectional streaming, low-latency remote harness

**Mechanism:**
- Agent opens WebSocket connection to harness
- Both sides can send messages at any time
- Supports multiplexing multiple sessions over one connection

**Pros:**
- Low latency (persistent connection)
- Bidirectional (harness can push updates to agent)
- Efficient for high-frequency events

**Cons:**
- More complex than HTTP
- Requires WebSocket-capable infrastructure

**Example:**

```javascript
const ws = new WebSocket('wss://harness.example.com/ahp');

ws.on('open', () => {
  const event = { jsonrpc: '2.0', id: '1', method: 'ahp/event', ... };
  ws.send(JSON.stringify(event));
});

ws.on('message', (data) => {
  const response = JSON.parse(data);
  console.log('Decision:', response.result.decision);
});
```

### 4. gRPC Transport

**Use case:** High-performance, strongly-typed RPC

**Mechanism:**
- Agent and harness communicate via gRPC
- Protocol Buffers define message schemas
- Supports streaming, bidirectional communication

**Pros:**
- Highest performance (binary protocol)
- Strong typing (compile-time validation)
- Built-in load balancing, retries, timeouts

**Cons:**
- Requires protobuf definitions
- More complex setup

**Example protobuf:**

```protobuf
service AhpService {
  rpc SendEvent(AhpEvent) returns (AhpDecision);
  rpc StreamEvents(stream AhpEvent) returns (stream AhpDecision);
}

message AhpEvent {
  string event_type = 1;
  string session_id = 2;
  google.protobuf.Struct payload = 3;
}

message AhpDecision {
  string decision = 1;
  string reason = 2;
}
```

### 5. Unix Socket Transport

**Use case:** Local IPC with lower overhead than stdio

**Mechanism:**
- Agent and harness communicate via Unix domain socket
- Same message format as stdio, but over socket

**Pros:**
- Lower overhead than stdio (no process spawning)
- Persistent connection (no reconnect cost)
- Works for multiple agents sharing one harness

**Cons:**
- Unix/Linux only (no Windows support)
- Requires socket file management

---

## Security Model

### Authentication

AHP supports multiple authentication mechanisms:

1. **No auth** (local stdio, trusted environment)
2. **API key** (HTTP header: `Authorization: Bearer <key>`)
3. **Mutual TLS** (client and server certificates)
4. **OAuth 2.0** (for web-based harnesses)
5. **Custom** (framework-specific auth plugins)

### Authorization

Fine-grained permissions per event type:

```json
{
  "permissions": {
    "pre_action": ["read", "write"],
    "post_action": ["read"],
    "query": ["read"]
  }
}
```

### Encryption

- **TLS 1.3** for network transports (HTTP, WebSocket, gRPC)
- **Encrypted stdio** for local transports (optional, using libsodium)

### Sandboxing

Harness can enforce resource limits:

```json
{
  "sandbox": {
    "max_memory_mb": 1024,
    "max_cpu_percent": 50,
    "allowed_network": ["10.0.0.0/8"],
    "allowed_filesystem": ["/workspace"],
    "max_subprocess_depth": 3
  }
}
```

### Audit Trail

All events are logged with:

- **Timestamp** (ISO 8601)
- **Session ID** (UUID)
- **Agent ID** (UUID)
- **Event type**
- **Payload** (full or redacted)
- **Decision** (allow/block/modify)
- **Signature** (HMAC-SHA256 or Ed25519)

Logs can be exported to:
- **Local files** (JSON, CSV)
- **Syslog** (RFC 5424)
- **Cloud logging** (CloudWatch, Stackdriver, Azure Monitor)
- **SIEM** (Splunk, ELK, Datadog)

---

## Implementation Guide

### For Agent Framework Authors

To add AHP support to your agent framework:

1. **Embed an AHP client** in your agent runtime
2. **Intercept agent actions** at key decision points
3. **Send events** to the harness before/after actions
4. **Enforce decisions** returned by the harness
5. **Handle errors** gracefully (timeout, connection loss)

**Minimal implementation:**

```python
class AhpClient:
    def __init__(self, transport):
        self.transport = transport  # stdio, HTTP, WebSocket, etc.
        self.session_id = str(uuid.uuid4())

    def send_event(self, event_type, payload):
        event = {
            "jsonrpc": "2.0",
            "id": str(uuid.uuid4()),
            "method": "ahp/event",
            "params": {
                "event_type": event_type,
                "session_id": self.session_id,
                "payload": payload
            }
        }
        response = self.transport.send(event)
        return response["result"]["decision"]

    def pre_action(self, action_type, arguments):
        decision = self.send_event("pre_action", {
            "action_type": action_type,
            "arguments": arguments
        })
        if decision == "block":
            raise PermissionError("Action blocked by harness")
        return decision
```

### For Harness Server Authors

To build a harness server:

1. **Choose a transport** (stdio, HTTP, WebSocket, gRPC)
2. **Implement the protocol** (parse events, return decisions)
3. **Define policies** (rules, allowlists, LLM-based analysis)
4. **Add observability** (logging, metrics, tracing)
5. **Test compliance** (use AHP test suite)

**Minimal implementation:**

```python
import json, sys

def handle_event(event):
    event_type = event["params"]["event_type"]
    payload = event["params"]["payload"]

    if event_type == "pre_action":
        # Apply policy
        if is_dangerous(payload):
            return {"decision": "block", "reason": "Dangerous action"}
        return {"decision": "allow"}

    # Default: allow
    return {"decision": "allow"}

# Main loop (stdio transport)
for line in sys.stdin:
    event = json.loads(line)
    req_id = event.get("id")

    if req_id:  # Request (blocking)
        result = handle_event(event)
        response = {"jsonrpc": "2.0", "id": req_id, "result": result}
        print(json.dumps(response), flush=True)
    else:  # Notification (fire-and-forget)
        handle_event(event)
```

---

## Reference Implementations

### Python Harness Server

See `examples/ahp_server.py` for a full-featured Python implementation with:

- Regex-based command blocking
- Depth-aware policies (stricter for sub-agents)
- Sensitive data detection
- Structured logging to stderr
- Graceful error handling

### TypeScript Harness Server

See `examples/ahp_server.ts` for a TypeScript implementation with:

- Type-safe event handling
- Async/await support
- WebSocket transport option
- Prometheus metrics export

### Go Harness Server

See `examples/ahp_server.go` for a high-performance Go implementation with:

- gRPC transport
- Concurrent event processing
- Redis-backed state management
- OpenTelemetry tracing

### Rust AHP Client

See `src/lib.rs` for the Rust client implementation used by A3S Code:

- Async/await with Tokio
- Multiple transport support
- Automatic reconnection
- Depth propagation for sub-agents

---

## Migration from v1.x

AHP v2.1 is **backward compatible** with v1.x and v2.0 for the stdio transport. Key changes:

| v1.x | v2.0 | Notes |
|------|------|-------|
| `harness/event` method | `ahp/event` method | Old method still supported |
| `action` field | `decision` field | Old field still supported |
| No handshake | `ahp/handshake` required | Optional for stdio, required for network transports |
| No batching | `ahp/batch` method | New feature |
| No query support | `ahp/query` method | New feature |

**Migration steps:**

1. Update harness server to handle `ahp/event` (or keep `harness/event` for compatibility)
2. Add `ahp/handshake` handler (optional for stdio)
3. Update decision field from `action` to `decision` (or support both)
4. Test with AHP v2.0 test suite

---

## License

MIT License. See LICENSE file for details.

## Contributing

Contributions welcome! Please see CONTRIBUTING.md for guidelines.

## Community

- **GitHub:** https://github.com/A3S-Lab/ahp
- **Discord:** https://discord.gg/a3s-lab
- **Docs:** https://docs.a3s.dev/ahp

---

**Version:** 2.1.0
**Last Updated:** 2026-04-02
**Status:** Stable

## Changelog

### v2.1.0 (2026-04-02)

**Protocol Changes (from v2.0):**
- Added `idle` event type for idle detection and dream system support
- Added `IdleDecision` response type for idle events (`Allow`, `Defer`)
- Added `EventContext` with client-exposes capabilities pattern
- Added `HeartbeatEvent` for periodic agent status updates

### v2.0.1 (2026-04-02)

**New Features:**
- **Idle Detection**: Support for `idle` event type to detect when agent is idle, enabling background consolidation/dream systems
- **EventContext**: Structured context passed with events including `recent_facts`, `memory_summary`, `session_stats`, `current_task`
- **Client-Exposed Capabilities**: `EventContext` supports dynamic `capabilities` field where clients self-report their abilities (memory_search, session_info, cross_session, etc.)
- **IdleDecision**: New decision type for idle events (`Allow`, `Defer`)
- **HeartbeatEvent**: Periodic status update structure for agent liveness

**Event Types Added:**
- `idle` - Agent idle detection (fire-and-forget notification)
- `heartbeat` - Periodic keepalive (already in spec, now formalized)

**EventContext Fields:**
| Field | Type | Description |
|-------|------|-------------|
| `recent_facts` | `Vec<Fact>` | Recent facts/knowledge retrieved |
| `memory_summary` | `MemorySummary` | Memory/knowledge base state |
| `session_stats` | `SessionStats` | Session statistics |
| `current_task` | `String` | Current task/goal description |
| `capabilities` | `HashMap` | Client self-reported capabilities |
