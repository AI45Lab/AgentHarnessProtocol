# a3s-ahp — Agent Harness Protocol

Language-agnostic JSON-RPC 2.0 supervision layer for [A3S Code](https://github.com/A3S-Lab/Code) agent sessions.

## Overview

AHP lets you supervise, inspect, and control A3S Code agent sessions from **any external process** — Python, Go, Java, a shell script, or anything else that can read/write newline-delimited JSON. It is modeled on the same stdio transport principle as MCP.

The host (A3S Code) spawns your harness server as a child process and streams lifecycle events over stdin. The server returns decisions over stdout.

```
A3S Code host
  └── AhpHookExecutor  (implements HookExecutor)
        ├── pre_tool_use / pre_prompt  →  JSON-RPC request  →  await response
        └── all other events           →  JSON-RPC notification  →  no wait

Your harness server (any language)
  ├── reads newline-delimited JSON from stdin
  └── writes JSON-RPC responses to stdout
```

## Quick start (Rust)

```rust
use a3s_ahp::AhpHookExecutor;
use a3s_code_core::{Agent, SessionOptions};

let harness = AhpHookExecutor::spawn("python3", &["harness.py"]).await?;
let session = Agent::new("agent.hcl").await?.session("/workspace", Some(
    SessionOptions::new().with_hook_executor(harness),
))?;
```

## Wire format

### Request (blocking event — host → server)

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "harness/event",
  "params": {
    "event_type": "pre_tool_use",
    "payload": { "session_id": "s1", "tool": "Bash", "args": { "command": "ls" }, ... },
    "meta": { "depth": 0 }
  }
}
```

### Response (server → host)

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": { "action": "continue" }
}
```

Valid actions: `continue`, `block`, `skip`, `retry`.

### Notification (fire-and-forget event — no response expected)

```json
{
  "jsonrpc": "2.0",
  "method": "harness/event",
  "params": { "event_type": "post_tool_use", "payload": { ... }, "meta": { "depth": 0 } }
}
```

## Blocking vs. fire-and-forget events

| Event | Type |
|-------|------|
| `pre_tool_use` | Blocking — host waits for decision |
| `pre_prompt` | Blocking — host waits for decision |
| all others | Fire-and-forget — notification only |

## `meta.depth`

Every message includes `meta.depth` — the sub-agent nesting level (`0` = top-level session). Use it to apply stricter policies to deeply nested agents.

## Reference server

`examples/ahp_server.py` is a fully functional Python reference implementation that blocks dangerous Bash commands and detects sensitive output patterns.

```bash
# Test standalone
echo '{"jsonrpc":"2.0","id":1,"method":"harness/event","params":{"event_type":"pre_tool_use","payload":{"session_id":"t","tool":"Bash","args":{"command":"rm -rf /"},"working_directory":"/","recent_tools":[]},"meta":{"depth":0}}}' \
  | python3 examples/ahp_server.py
```

## License

MIT
