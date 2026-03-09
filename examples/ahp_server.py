#!/usr/bin/env python3
"""
AHP Server — Python reference implementation of the Agent Harness Protocol.

The Agent Harness Protocol (AHP) is a language-agnostic protocol for
supervising AI agent sessions. This server communicates with A3S Code via
newline-delimited JSON-RPC 2.0 over stdio.

Protocol:
  - Host sends requests (with "id") for blocking events that need a decision.
  - Host sends notifications (no "id") for fire-and-forget events.
  - For requests, this server writes a JSON-RPC response to stdout.
  - All logging goes to stderr so it does not interfere with the protocol.

Usage:
  # Start directly (the host process spawns this automatically)
  python3 ahp_server.py

  # Or run standalone to test:
  echo '{"jsonrpc":"2.0","id":1,"method":"harness/event","params":{"event_type":"pre_tool_use","payload":{"session_id":"s1","tool":"Bash","args":{"command":"rm -rf /"},"working_directory":"/","recent_tools":[]}}}' | python3 ahp_server.py
"""

from __future__ import annotations

import json
import re
import sys
from typing import Any

# ---------------------------------------------------------------------------
# Policy configuration
# ---------------------------------------------------------------------------

# Patterns matched against Bash command strings.
# Any match causes the command to be blocked.
BLOCKED_PATTERNS: list[tuple[str, str]] = [
    (r"rm\s+-[rf]{1,2}\s+/", "recursive delete from root"),
    (r"\bdd\s+if=", "raw disk write"),
    (r"\bmkfs\.", "filesystem format"),
    (r">\s*/dev/sd", "direct disk write"),
    (r"chmod\s+[0-7]*7[0-7]{2}\s+/", "world-writable root path"),
    (r"curl\s+.*\|\s*(ba)?sh", "pipe curl to shell"),
    (r"wget\s+.*\|\s*(ba)?sh", "pipe wget to shell"),
    (r":\(\)\s*\{.*:\|:&\s*\}", "fork bomb"),
]

# Patterns matched against tool output strings.
# Any match emits a warning to stderr (observe-only, no blocking).
SENSITIVE_OUTPUT_PATTERNS: list[str] = [
    r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}",  # e-mail addresses
    r"(?i)(password|secret|token|api[_-]?key)\s*[:=]\s*\S+",
    r"\b(?:\d{4}[\s-]?){3}\d{4}\b",  # credit-card-like numbers
]

# ---------------------------------------------------------------------------
# Event handlers
# ---------------------------------------------------------------------------


def on_pre_tool_use(payload: dict[str, Any], depth: int) -> dict[str, Any]:
    """Inspect a tool call before it executes. Return block or continue."""
    tool: str = payload.get("tool", "")
    args: Any = payload.get("args", {})
    command: str = args.get("command", "") if isinstance(args, dict) else ""
    session_id: str = payload.get("session_id", "?")
    depth_tag = f"d{depth}" if depth > 0 else "root"

    if tool == "Bash" and command:
        for pattern, description in BLOCKED_PATTERNS:
            if re.search(pattern, command, re.IGNORECASE):
                _log(
                    f"BLOCK  [{session_id}][{depth_tag}] "
                    f"Bash command matches '{description}': {command[:120]}"
                )
                return {
                    "action": "block",
                    "reason": f"AHP: command blocked — {description}: {command[:80]}",
                }

    _log(
        f"ALLOW  [{session_id}][{depth_tag}] "
        f"{tool}: {command[:80] if command else '(no command)'}"
    )
    return {"action": "continue"}


def on_pre_prompt(payload: dict[str, Any], depth: int) -> dict[str, Any]:
    """Inspect or modify a prompt before it is sent to the LLM."""
    depth_tag = f"d{depth}" if depth > 0 else "root"
    _log(
        f"PROMPT [{payload.get('session_id', '?')}][{depth_tag}] "
        f"message_count={payload.get('message_count', 0)}"
    )
    return {"action": "continue"}


def on_post_tool_use(payload: dict[str, Any], depth: int) -> None:
    """Observe tool output for sensitive data (fire-and-forget)."""
    result = payload.get("result", {})
    output: str = result.get("output", "") if isinstance(result, dict) else ""
    if output:
        for pattern in SENSITIVE_OUTPUT_PATTERNS:
            if re.search(pattern, output):
                depth_tag = f"d{depth}" if depth > 0 else "root"
                _log(
                    f"ALERT  [{payload.get('session_id', '?')}][{depth_tag}] "
                    f"Sensitive pattern detected in {payload.get('tool', '?')} output"
                )
                break


def on_session_start(payload: dict[str, Any], depth: int) -> None:
    depth_tag = f"d{depth}" if depth > 0 else "root"
    _log(
        f"START  [{payload.get('session_id', '?')}][{depth_tag}] "
        f"model={payload.get('model_provider', '?')}/{payload.get('model_name', '?')}"
    )


def on_session_end(payload: dict[str, Any], depth: int) -> None:
    depth_tag = f"d{depth}" if depth > 0 else "root"
    _log(
        f"END    [{payload.get('session_id', '?')}][{depth_tag}] "
        f"tokens={payload.get('total_tokens', 0)} "
        f"tool_calls={payload.get('total_tool_calls', 0)}"
    )


# ---------------------------------------------------------------------------
# Dispatcher
# ---------------------------------------------------------------------------

# Blocking events: the host waits for a response.
BLOCKING_HANDLERS: dict[str, Any] = {
    "pre_tool_use": on_pre_tool_use,
    "pre_prompt": on_pre_prompt,
}

# Fire-and-forget events: no response is required.
NOTIFICATION_HANDLERS: dict[str, Any] = {
    "post_tool_use": on_post_tool_use,
    "session_start": on_session_start,
    "session_end": on_session_end,
}


def dispatch(event_type: str, payload: dict[str, Any], depth: int) -> dict[str, Any] | None:
    """
    Dispatch an event to the appropriate handler.
    Returns a decision dict for blocking events, None for notifications.
    """
    if event_type in BLOCKING_HANDLERS:
        return BLOCKING_HANDLERS[event_type](payload, depth)
    if event_type in NOTIFICATION_HANDLERS:
        NOTIFICATION_HANDLERS[event_type](payload, depth)
        return None
    # Unknown event — default to continue (forward-compatible).
    return {"action": "continue"}


# ---------------------------------------------------------------------------
# Main loop
# ---------------------------------------------------------------------------


def _log(message: str) -> None:
    """Write a log line to stderr (never pollutes the JSON-RPC stdout stream)."""
    print(f"[AHP] {message}", file=sys.stderr, flush=True)


def main() -> None:
    _log("AHP server started — reading events from stdin")

    for raw_line in sys.stdin:
        line = raw_line.strip()
        if not line:
            continue

        try:
            msg: dict[str, Any] = json.loads(line)
        except json.JSONDecodeError as exc:
            _log(f"ERROR  Could not parse JSON: {exc} — line: {line[:200]}")
            continue

        params: dict[str, Any] = msg.get("params", {})
        event_type: str = params.get("event_type", "")
        payload: dict[str, Any] = params.get("payload", {})
        depth: int = params.get("meta", {}).get("depth", 0)
        req_id = msg.get("id")

        if req_id is None:
            # Notification — dispatch and move on.
            dispatch(event_type, payload, depth)
            continue

        # Request — must respond.
        try:
            result = dispatch(event_type, payload, depth)
        except Exception as exc:
            _log(f"ERROR  Handler raised: {exc}")
            result = None

        if result is None:
            result = {"action": "continue"}

        response = {
            "jsonrpc": "2.0",
            "id": req_id,
            "result": result,
        }
        print(json.dumps(response), flush=True)

    _log("AHP server exiting — stdin closed")


if __name__ == "__main__":
    main()
