# Agent Harness Protocol (AHP) v2.3

**Universal supervision protocol for autonomous AI agents**

## The Problem

Every AI agent framework (Claude Code, Codex, OpenClaw, LangChain, AutoGPT, A3S Code, CrewAI...) has its own hooks/callbacks system. Policies written for one framework don't work with others. This creates:

- **Vendor lock-in** — Safety rules are non-transferable
- **Duplicated effort** — Same policies reimplemented per framework
- **No interoperability** — Agents can't be composed across frameworks

## The Solution

AHP defines **one protocol** that any agent framework can implement. Once an agent supports AHP, it can use any AHP-compatible supervisor (harness).

```
┌─────────────────────────────────────────────────────────────┐
│                      Agent Framework                         │
│   (Claude Code, Codex, OpenClaw, LangChain, AutoGPT,       │
│    A3S Code, CrewAI, any other)                            │
│                          │                                   │
│                          ▼                                   │
│   ┌─────────────────────────────────────────────────────┐   │
│   │  AHP Client                                           │   │
│   │  • Intercepts agent actions                          │   │
│   │  • Sends events to harness                           │   │
│   │  • Enforces harness decisions                        │   │
│   └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                     AHP Harness                             │
│   (Policy engine, safety rules, audit logging, etc.)        │
│                                                             │
│   Receives events → Applies policies → Returns decisions     │
│   Allow / Block / Modify / Defer / Escalate                │
└─────────────────────────────────────────────────────────────┘
```

## Core Concepts

### Agent ↔ Harness Communication

1. **Agent sends events** to harness at key decision points
2. **Harness responds** with decisions (allow, block, modify...)
3. **Agent enforces** the decision before proceeding

### Event Types

| Event | When | Blocking |
|-------|------|----------|
| `pre_action` | Before any agent action | Yes |
| `post_action` | After action completes | No |
| `pre_prompt` | Before LLM call | Yes |
| `post_response` | After LLM response | No |
| `intent_detection` | Detect user intent from prompt | Yes |
| `context_perception` | Model needs workspace knowledge | Yes |
| `memory_recall` | Model retrieves from memory | Yes |
| `planning` | Task decomposition | Yes |
| `reasoning` | CoT/ToT reasoning | Yes |
| `idle` | Agent is idle and asks whether background work should run | Yes |
| `heartbeat` | Periodic status | No |
| `success` | Operation succeeded | No |
| `error` | Operation failed | No |
| `rate_limit` | Rate limit hit and needs backpressure decision | Yes |
| `confirmation` | Human approval needed | Yes |

### Decision Types

| Decision | Meaning |
|----------|---------|
| `Allow` | Proceed (optionally with modified payload) |
| `Block` | Cancel, return error to agent |
| `Modify` | Proceed with harness-modified parameters |
| `Defer` | Retry after specified delay |
| `Escalate` | Forward to human operator |

Batch requests only support events that return the generic `Decision` shape.
Harness points with specialized decision payloads must be sent individually.

## Harness Points (驾驭点)

AHP v2.3 introduces **harness points** — structured hooks that intercept agent operations at specific moments.

### Event Flow Diagram

```
                        ┌─────────────────────────────────────────┐
                        │              Agent Loop                  │
                        └─────────────────────────────────────────┘
                                              │
                    ┌─────────────────────────┼─────────────────────────┐
                    │                         │                         │
                    ▼                         ▼                         ▼
            ┌───────────────┐        ┌───────────────┐        ┌───────────────┐
            │   Perceive    │        │   Remember    │        │    Plan       │
            │               │        │               │        │               │
            │ PreContext    │        │ PreMemory     │        │ PrePlanning   │
            │ Perception    │        │ Recall        │        │               │
            └───────┬───────┘        └───────┬───────┘        └───────┬───────┘
                    │                         │                         │
                    ▼                         ▼                         ▼
            ┌───────────────┐        ┌───────────────┐        ┌───────────────┐
            │    Think     │        │    Act        │        │   Observe     │
            │               │        │               │        │               │
            │ PreReasoning  │        │ PreToolUse    │        │ OnSuccess     │
            │ PostReasoning │        │ PostToolUse   │        │ OnError       │
            └───────────────┘        └───────┬───────┘        └───────────────┘
                                              │
                                              ▼
                                    ┌───────────────────┐
                                    │   Confirm if      │
                                    │   needed (block)  │
                                    │                   │
                                    │ OnConfirmation    │
                                    └───────────────────┘
```

### Harness Points by Category

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        BLOCKING HARNESS POINTS                               │
│                  (Agent waits for harness decision before proceeding)         │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│   ┌──────────────┐    ┌──────────────┐    ┌──────────────┐    ┌───────────┐ │
│   │   Perceive   │───▶│   Remember   │───▶│    Plan     │───▶│   Think   │ │
│   │              │    │              │    │             │    │           │ │
│   │ PreContext   │    │ PreMemory    │    │ PrePlanning │    │PreReason- │ │
│   │ Perception   │    │ Recall       │    │             │    │   ing     │ │
│   └──────┬───────┘    └──────┬───────┘    └──────┬───────┘    └─────┬─────┘ │
│          │                   │                   │                   │       │
│          ▼                   ▼                   ▼                   ▼       │
│   ┌──────────────┐    ┌──────────────┐    ┌──────────────┐    ┌───────────┐ │
│   │   Allow      │    │   Allow      │    │   Allow      │    │  Allow    │ │
│   │   +Inject   │    │   +Recall    │    │   +Plan     │    │  +Reason  │ │
│   │   Context   │    │   Memory     │    │   Subtasks  │    │   Hints   │ │
│   ├──────────────┤    ├──────────────┤    ├──────────────┤    ├───────────┤ │
│   │   Block      │    │   Block      │    │   Block      │    │   Block   │ │
│   │   (skip)     │    │   (empty)    │    │   (abort)   │    │   (skip)  │ │
│   └──────────────┘    └──────────────┘    └──────────────┘    └───────────┘ │
│                                                                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│   ┌──────────────┐    ┌──────────────┐    ┌──────────────┐                 │
│   │     Act      │───▶│   Confirm    │───▶│   Prompt     │                 │
│   │              │    │              │    │              │                 │
│   │ PreToolUse   │    │ OnConfirma-  │    │ PrePrompt    │                 │
│   │              │    │   tion       │    │              │                 │
│   └──────┬───────┘    └──────┬───────┘    └──────┬───────┘                 │
│          │                   │                   │                          │
│          ▼                   ▼                   ▼                          │
│   ┌──────────────┐    ┌──────────────┐    ┌──────────────┐                 │
│   │   Allow      │    │   Allow      │    │   Allow      │                 │
│   │   +Modify    │    │   +User      │    │   +Inject    │                 │
│   │   Args       │    │   Input      │    │   System     │                 │
│   ├──────────────┤    ├──────────────┤    ├──────────────┤                 │
│   │   Block      │    │   Block      │    │   Block      │                 │
│   │   (reject)   │    │   (cancel)   │    │   (override) │                 │
│   └──────────────┘    └──────────────┘    └──────────────┘                 │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│                     FIRE-AND-FORGET EVENTS                                  │
│                    (Agent continues immediately, no wait)                    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│   OnSuccess ──▶ Record to audit log, update metrics, trigger workflows      │
│                                                                              │
│   OnError   ──▶ Record to audit log, increment error counters, alert        │
│                                                                              │
│   OnRate    ──▶ Record to audit log, apply backpressure, alert              │
│   Limit                                                                │
│                                                                              │
│   Post      ──▶ Record to audit log, update session stats                   │
│   ToolUse                                                        │
│                                                                              │
│   Post      ──▶ Record to audit log, store reasoning trace                  │
│   Reasoning                                                       │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Context Perception (上下文感知)

When the model needs to understand its workspace, `context_perception` fires. This is the most nuanced harness point.

### Perception Intent Matrix (四象限)

```
                         TARGET TYPE
                    ┌─────────────┬─────────────┐
                    │   ENTITY    │  LOCATION   │
              ┌─────┼─────────────┼─────────────┤
    INTENT    │RECO-│  "What is   │  "Where is  │
              │GNIZE│   X?"       │   Y?"       │
              ├─────┼─────────────┼─────────────┤
              │UNDER│  "What does │  "What is   │
              │STAND│   X do?"    │   at Y?"    │
              ├─────┼─────────────┼─────────────┤
              │EXPL-│  "How does │  "What      │
              │ORE  │   X work?"  │   exists    │
              │     │             │   around Y?"│
              ├─────┼─────────────┼─────────────┤
              │RETR-│  "Find all  │  "Find all  │
              │IEVE │   X"        │   things at │
              │     │             │   Y"        │
              └─────┴─────────────┴─────────────┘


                         URGENCY / DOMAIN
              ┌─────────────┬─────────────┬─────────────┐
              │   CODING    │   RESEARCH  │  OPERATIONS │
     ┌────────┼─────────────┼─────────────┼─────────────┤
     │CRITICAL│ Audit code  │ Urgent fact │ Immediate   │
     │        │ security    │ lookup      │ rollback    │
     ├────────┼─────────────┼─────────────┼─────────────┤
     │  HIGH  │ Feature     │ Paper deep  │ Deploy with │
     │        │ context     │ dive        │ canary      │
     ├────────┼─────────────┼─────────────┼─────────────┤
     │ NORMAL │ Normal dev  │ General      │ Standard    │
     │        │ docs        │ search      │ ops         │
     ├────────┼─────────────┼─────────────┼─────────────┤
     │   LOW  │ Cleanup,    │ Background  │ Batch jobs, │
     │        │ refactor    │ learning    │ reports     │
     └────────┴─────────────┴─────────────┴─────────────┘
```

### Context Injection Flow

```
    Agent                          AHP Client                      Harness
      │                                │                               │
      │  Model needs context           │                               │
      │  ────────────────────────────▶ │                               │
      │                                │                               │
      │                     ┌──────────┴──────────┐                    │
      │                     │ PreContextPerception │                    │
      │                     │  event created      │                    │
      │                     │  - intent            │                    │
      │                     │  - target            │                    │
      │                     │  - domain            │                    │
      │                     │  - query             │                    │
      │                     │  - constraints       │                    │
      │                     └──────────┬──────────┘                    │
      │                                │                               │
      │                                │  AhpEvent                     │
      │                                │  (blocking)                   │
      │                                │ ─────────────────────────────▶│
      │                                │                               │
      │                                │                 ┌────────────┴────────┐
      │                                │                 │ Policy Evaluation  │
      │                                │                 │ - Check permissions │
      │                                │                 │ - Search knowledge  │
      │                                │                 │ - Retrieve files    │
      │                                │                 │ - Build context      │
      │                                │                 └────────────┬────────┘
      │                                │                               │
      │                                │  Decision {                    │
      │                                │    decision: "allow",          │
      │                                │    injected_context: {         │
      │                                │      facts: [...],             │
      │                                │      file_contents: [...],    │
      │                                │      project_summary: {...}   │
      │                                │    }                          │
      │                                │  }                            │
      │                                │ ◀─────────────────────────────│
      │                                │                               │
      │                     ┌──────────┴──────────┐                     │
      │                     │ PostContextPerception│                     │
      │                     │  - facts_retrieved  │                     │
      │                     │  - files_retrieved  │                     │
      │                     └──────────┬──────────┘                     │
      │                                │                               │
      │  Context injected              │                               │
      │  into model                    │                               │
      │ ◀──────────────────────────────│                               │
      │                                │                               │
```

## Intent Detection (意图检测)

`intent_detection` fires on **every prompt** before `context_perception`. The harness determines the user's intent using LLM classification, keyword matching, or any custom logic. This enables:

- **Multi-language intent recognition** — Harness can use LLM for non-English prompts
- **Centralized intent taxonomy** — Update detection logic without changing agent code
- **Custom detection rules** — Organization-specific intent patterns

### Intent Values

| Intent | Triggered By | Description |
|--------|-------------|-------------|
| `locate` | "where is", "find", "search for" | User wants to find files/functions |
| `understand` | "how does", "explain", "what does" | User wants to understand code |
| `retrieve` | "remember", "earlier", "previous" | User references past context |
| `explore` | "project structure", "what files" | User wants overview |
| `reason` | "why did", "why is", "cause" | User asks why something happened |
| `validate` | "verify", "check if", "debug" | User wants to verify correctness |
| `compare` | "difference between", "compare" | User wants comparison |
| `track` | "status", "progress", "history" | User asks for status |

### IntentDetection Flow

```
    Agent                          AHP Client                      Harness
      │                                │                               │
      │  User prompt                  │                               │
      │  ────────────────────────────▶│                               │
      │                                │                               │
      │                     ┌──────────┴──────────┐                    │
      │                     │ IntentDetection    │                    │
      │                     │  event created    │                    │
      │                     │  - prompt        │                    │
      │                     │  - workspace     │                    │
      │                     │  - language_hint │                    │
      │                     └──────────┬──────────┘                    │
      │                                │                               │
      │                                │  AhpEvent                     │
      │                                │  (blocking)                   │
      │                                │──────────────────────────────▶│
      │                                │                               │
      │                                │                 ┌────────────┴────────┐
      │                                │                 │ LLM classification │
      │                                │                 │ or custom logic   │
      │                                │                 └────────────┬────────┘
      │                                │                               │
      │                                │  Decision {                    │
      │                                │    decision: "allow",         │
      │                                │    detected_intent: "locate",│
      │                                │    confidence: 0.95,         │
      │                                │    target_hints: {           │
      │                                │      target_type: "function",│
      │                                │      target_name: "auth"    │
      │                                │    }                        │
      │                                │  }                          │
      │                                │◀─────────────────────────────│
      │                                │                               │
      │  Intent detected              │                               │
      │  + PreContextPerception       │                               │
      │  follows with full context    │                               │
      │ ◀──────────────────────────────│                              │
      │                                │                               │
```

### IntentDetection Decision Types

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        INTENT DETECTION DECISION                           │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│   ALLOW (intent detected)                                                   │
│   ┌─────────────────────────────────────────────────────────────────────┐  │
│   │  {                                                                  │  │
│   │    "decision": "allow",                                             │  │
│   │    "detected_intent": "locate",                                    │  │
│   │    "confidence": 0.95,                                              │  │
│   │    "target_hints": {                                                │  │
│   │      "target_type": "function",                                     │  │
│   │      "target_name": "authenticate",                                 │  │
│   │      "domain": "coding"                                             │  │
│   │    }                                                                │  │
│   │  }                                                                  │  │
│   └─────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
│   BLOCK (skip context perception)                                            │
│   ┌─────────────────────────────────────────────────────────────────────┐  │
│   │  {                                                                  │  │
│   │    "decision": "block",                                             │  │
│   │    "reason": "intent detection disabled by policy"                 │  │
│   │  }                                                                  │  │
│   └─────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### ContextPerception Decision Types

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        CONTEXT INJECTION DECISION                            │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│   ALLOW (with context)                                                       │
│   ┌─────────────────────────────────────────────────────────────────────┐  │
│   │  {                                                                  │  │
│   │    "decision": "allow",                                             │  │
│   │    "injected_context": {                                            │  │
│   │      "facts": [                                                     │  │
│   │        {"content": "...", "source": "...", "confidence": 0.95}       │  │
│   │      ],                                                              │  │
│   │      "file_contents": [                                             │  │
│   │        {"path": "...", "snippet": "...", "relevance_score": 0.9}   │  │
│   │      ],                                                              │  │
│   │      "project_summary": {                                           │  │
│   │        "project_name": "...", "language": "...", "key_files": [...] │  │
│   │      }                                                              │  │
│   │    }                                                                │  │
│   │  }                                                                  │  │
│   └─────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
│   BLOCK (skip context)                                                       │
│   ┌─────────────────────────────────────────────────────────────────────┐  │
│   │  {                                                                  │  │
│   │    "decision": "block",                                             │  │
│   │    "reason": "context forbidden by policy"                          │  │
│   │  }                                                                  │  │
│   └─────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
│   REFINE (need more info)                                                    │
│   ┌─────────────────────────────────────────────────────────────────────┐  │
│   │  {                                                                  │  │
│   │    "decision": "refine",                                            │  │
│   │    "hints": {                                                       │  │
│   │      "suggested_intent": "understand",                              │  │
│   │      "suggested_domain": "coding",                                  │  │
│   │      "clarifying_question": "What specific aspect of X?"             │  │
│   │    }                                                                │  │
│   │  }                                                                  │  │
│   └─────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Transport Agnostic

AHP works over any transport layer:

- **stdio** — Local subprocess (default, simplest)
- **HTTP** — Remote harness, web deployment
- **WebSocket** — Bidirectional streaming, low latency
- **gRPC** — High-performance RPC (feature placeholder; not implemented yet)
- **Unix Socket** — Local IPC, lower overhead than stdio

The **protocol** (message format) is identical across transports. Choose the transport that fits your deployment.

## Protocol Format

AHP uses **JSON-RPC 2.0**:

```json
// Agent → Harness (request)
{
  "jsonrpc": "2.0",
  "id": "req-123",
  "method": "ahp/event",
  "params": {
    "event_type": "pre_action",
    "session_id": "sess-abc",
    "agent_id": "agent-xyz",
    "timestamp": "2026-04-10T12:00:00Z",
    "depth": 0,
    "payload": { ... },
    "context": { ... }
  }
}

// Harness → Agent (response)
{
  "jsonrpc": "2.0",
  "id": "req-123",
  "result": {
    "decision": "allow",
    "reason": null,
    "modified_payload": null
  }
}
```

## Quick Start

### Rust Client

```rust
use a3s_ahp::{AhpClient, Transport, EventType, Decision};

let client = AhpClient::new(Transport::Stdio {
    program: "python3".into(),
    args: vec!["harness.py".into()],
}).await?;

let decision = client.send_event_decision(
    EventType::PreAction,
    serde_json::json!({
        "action_type": "tool_call",
        "tool_name": "bash",
        "arguments": { "command": "ls -la" }
    })
).await?;

match decision {
    Decision::Allow { .. } => println!("Proceed"),
    Decision::Block { reason, .. } => println!("Blocked: {}", reason),
    _ => {}
}
```

### Python Harness

```python
import json, sys

for line in sys.stdin:
    event = json.loads(line)
    req_id = event.get("id")

    if event["method"] == "ahp/event":
        event_type = event["params"]["event_type"]
        payload = event["params"]["payload"]

        # Apply policy
        if event_type == "pre_action" and is_dangerous(payload):
            result = {"decision": "block", "reason": "Dangerous action"}
        else:
            result = {"decision": "allow"}

        if req_id:  # Request (blocking)
            print(json.dumps({"jsonrpc": "2.0", "id": req_id, "result": result}))
            sys.stdout.flush()
```

## Project Structure

```
ahp/
├── src/
│   ├── lib.rs          # Main library (AhpClient, AhpServer, types)
│   ├── protocol.rs     # Protocol types (EventType, Decision, etc.)
│   ├── client.rs       # Client implementation
│   ├── server.rs       # Server implementation
│   ├── error.rs        # Error types
│   ├── auth.rs         # Authentication
│   └── transport/      # Transport implementations
│       ├── mod.rs
│       ├── stdio.rs
│       ├── http.rs
│       ├── websocket.rs
│       ├── unix_socket.rs
│       └── grpc.rs
├── examples/
│   ├── simple_client.rs
│   ├── simple_server.py
│   ├── http_client.rs
│   ├── http_server.rs
│   └── websocket_*.rs
└── Cargo.toml
```

## Features

- **Framework-agnostic** — Any agent can implement AHP
- **Language-neutral** — Harnesses can be written in any language
- **Transport-flexible** — Works over stdio, HTTP, WebSocket, and Unix sockets; gRPC is reserved as a feature placeholder
- **Bidirectional** — Agents can query harness, not just receive commands
- **Extensible** — New event types via capability negotiation
- **Structured context** — Rich context injection for informed decisions

## Version

- **Protocol:** 2.3
- **This crate:** 2.3.1

## License

MIT
