#!/usr/bin/env python3
"""
AHP Idle Detection Integration Test

Tests the AHP idle detection feature with real minmax model.
Uses environment variables for configuration from .a3s/config.hcl.

Run:
    export MINIMAX_API_KEY="sk-ZaH1YnkiGmcBt8qxKWfsBV5w9aInp4QuDUeq1HEIOAzEg5cT"
    export MINIMAX_BASE_URL="http://35.220.164.252:3888/v1/"
    export MINIMAX_MODEL="MiniMax-M2.7-highspeed"
    python3 examples/test_ahp_idle_detection.py
"""

import os
import sys
import json
import asyncio
import httpx
from datetime import datetime
from typing import Optional


# =============================================================================
# Configuration from environment
# =============================================================================

MINIMAX_API_KEY = os.environ.get("MINIMAX_API_KEY", "sk-ZaH1YnkiGmcBt8qxKWfsBV5w9aInp4QuDUeq1HEIOAzEg5cT")
MINIMAX_BASE_URL = os.environ.get("MINIMAX_BASE_URL", "http://35.220.164.252:3888/v1/")
MINIMAX_MODEL = os.environ.get("MINIMAX_MODEL", "MiniMax-M2.7-highspeed")
AHP_SERVER_URL = os.environ.get("AHP_SERVER_URL", "http://localhost:8080/ahp")


# =============================================================================
# AHP Protocol Types
# =============================================================================

class AhpEvent:
    """AHP Event structure"""
    def __init__(
        self,
        event_type: str,
        session_id: str,
        agent_id: str,
        timestamp: str,
        depth: int,
        payload: dict,
        context: Optional[dict] = None,
        metadata: Optional[dict] = None
    ):
        self.event_type = event_type
        self.session_id = session_id
        self.agent_id = agent_id
        self.timestamp = timestamp
        self.depth = depth
        self.payload = payload
        self.context = context
        self.metadata = metadata

    @classmethod
    def from_dict(cls, data: dict) -> "AhpEvent":
        return cls(
            event_type=data["event_type"],
            session_id=data["session_id"],
            agent_id=data["agent_id"],
            timestamp=data["timestamp"],
            depth=data["depth"],
            payload=data["payload"],
            context=data.get("context"),
            metadata=data.get("metadata")
        )

    def to_dict(self) -> dict:
        result = {
            "event_type": self.event_type,
            "session_id": self.session_id,
            "agent_id": self.agent_id,
            "timestamp": self.timestamp,
            "depth": self.depth,
            "payload": self.payload,
        }
        if self.context:
            result["context"] = self.context
        if self.metadata:
            result["metadata"] = self.metadata
        return result


class Decision:
    """AHP Decision types"""
    @staticmethod
    def allow(modified_payload=None, metadata=None) -> dict:
        return {
            "decision": "allow",
            "modified_payload": modified_payload,
            "metadata": metadata
        }

    @staticmethod
    def block(reason: str, metadata=None) -> dict:
        return {
            "decision": "block",
            "reason": reason,
            "metadata": metadata
        }

    @staticmethod
    def idle_allow() -> dict:
        return {"decision": "allow"}

    @staticmethod
    def idle_defer(reason: str = None) -> dict:
        return {"decision": "defer", "reason": reason}


# =============================================================================
# Minmax LLM Client
# =============================================================================

class MinmaxClient:
    """Client for Minmax API compatible endpoints"""

    def __init__(self, api_key: str, base_url: str, model: str):
        self.api_key = api_key
        self.base_url = base_url.rstrip("/")
        self.model = model
        self.client = httpx.AsyncClient(timeout=60.0)

    async def complete(self, messages: list, system: str = None) -> str:
        """Send a completion request to the model"""
        headers = {
            "Authorization": f"Bearer {self.api_key}",
            "Content-Type": "application/json"
        }

        payload = {
            "model": self.model,
            "messages": messages,
            "temperature": 0.7,
            "max_tokens": 500
        }

        if system:
            payload["messages"] = [{"role": "system", "content": system}] + messages

        response = await self.client.post(
            f"{self.base_url}/chat/completions",
            headers=headers,
            json=payload
        )

        if response.status_code != 200:
            raise Exception(f"API error: {response.status_code} - {response.text}")

        data = response.json()
        return data["choices"][0]["message"]["content"]

    async def close(self):
        await self.client.aclose()


# =============================================================================
# AHP Event Handler with LLM
# =============================================================================

class LlmEventHandler:
    """
    AHP Event Handler that uses LLM for decision making.
    Demonstrates context-aware control with idle detection and capabilities.
    """

    def __init__(self, llm_client: MinmaxClient):
        self.llm_client = llm_client
        self.session_contexts: dict = {}  # session_id -> context
        self.idle_events: list = []  # track idle events for testing

    async def handle_event(self, event: AhpEvent) -> dict:
        """Handle a blocking event with LLM-powered decision making"""
        print(f"\n[handle_event] Received: {event.event_type}")

        # Build context for decision
        context = self._build_context(event)

        # If capabilities are exposed, demonstrate how to use them
        if event.context and event.context.get("capabilities"):
            await self._handle_capabilities(event, event.context["capabilities"])

        # Use LLM to make context-aware decision
        if event.event_type == "pre_action":
            return await self._handle_pre_action(event, context)
        elif event.event_type == "pre_prompt":
            return await self._handle_pre_prompt(event, context)
        elif event.event_type == "idle":
            return await self._handle_idle(event, context)
        else:
            return Decision.allow()

    async def _handle_capabilities(self, event: AhpEvent, capabilities: dict):
        """
        Demonstrate how the server can use client exposes capabilities.

        Capabilities allow the client to expose:
        - HTTP endpoints for the server to call
        - Query handlers
        - Memory search endpoints
        - Cross-session query endpoints
        - Any custom functionality
        """
        print(f"  [capabilities] Client exposed {len(capabilities)} capabilities:")

        for name, cap in capabilities.items():
            cap_type = cap.get("type") if isinstance(cap, dict) else "unknown"
            print(f"    - {name}: {cap_type}")

            # Example: If client exposes a memory_search capability, use it
            if name == "memory_search" and isinstance(cap, dict):
                url = cap.get("url")
                if url:
                    print(f"      -> Would call: {url}")

            # Example: If client exposes a session_info capability
            if name == "session_info" and isinstance(cap, dict):
                handler = cap.get("handler")
                if handler:
                    print(f"      -> Would invoke handler: {handler}")

    async def _handle_pre_action(self, event: AhpEvent, context: dict) -> dict:
        """Handle pre-action event - decide whether to allow the tool call"""
        tool_name = event.payload.get("tool", "unknown")
        arguments = event.payload.get("arguments", {})

        system_prompt = """You are a security-focused AI assistant evaluating tool calls.
Analyze the tool name and arguments. If the tool appears dangerous (e.g., rm -rf, format disk, etc.),
respond with "BLOCK" followed by a reason. Otherwise respond "ALLOW"."""

        user_message = f"Tool: {tool_name}\nArguments: {json.dumps(arguments, indent=2)}\n\nDecision:"

        response = await self.llm_client.complete(
            [{"role": "user", "content": user_message}],
            system=system_prompt
        )

        print(f"  [pre_action] LLM response: {response[:100]}...")

        if response.strip().upper().startswith("BLOCK"):
            reason = response.strip()[5:].strip() if len(response) > 5 else "LLM blocked"
            return Decision.block(reason)
        return Decision.allow()

    async def _handle_pre_prompt(self, event: AhpEvent, context: dict) -> dict:
        """Handle pre-prompt event - analyze the prompt before execution"""
        prompt = event.payload.get("prompt", "")[:200]

        system_prompt = """You are an AI assistant analyzing user prompts for safety.
If the prompt appears malicious or dangerous, respond "BLOCK" followed by a reason.
Otherwise respond "ALLOW"."""

        response = await self.llm_client.complete(
            [{"role": "user", "content": f"Prompt: {prompt}\n\nDecision:"}],
            system=system_prompt
        )

        print(f"  [pre_prompt] LLM response: {response[:100]}...")

        if response.strip().upper().startswith("BLOCK"):
            reason = response.strip()[5:].strip() if len(response) > 5 else "LLM blocked"
            return Decision.block(reason)
        return Decision.allow()

    async def _handle_idle(self, event: AhpEvent, context: dict) -> dict:
        """
        Handle idle event - this is the key feature for dream system.
        When the agent is idle, we can trigger background consolidation.
        """
        idle_duration = event.payload.get("idle_duration_ms", 0)
        idle_reason = event.payload.get("idle_reason", "unknown")
        suggested_action = event.payload.get("suggested_action", "dream")

        print(f"  [idle] Duration: {idle_duration}ms, Reason: {idle_reason}")
        print(f"  [idle] Suggested action: {suggested_action}")

        # Track for test verification
        self.idle_events.append({
            "timestamp": event.timestamp,
            "duration_ms": idle_duration,
            "reason": idle_reason,
            "suggested_action": suggested_action
        })

        # Use LLM to decide whether to allow background consolidation
        memory_summary = context.get("memory_summary", {})
        recent_topics = memory_summary.get("recent_topics", [])

        system_prompt = """You are a background task coordinator.
When an AI agent becomes idle, you can decide to run background tasks like:
- Memory consolidation (整理记忆)
- Fact extraction (提取事实)
- Knowledge graph updates

If the agent has been active recently, consider allowing consolidation.
If it just started or was very briefly idle, consider deferring."""

        topics_str = ", ".join(recent_topics[:5]) if recent_topics else "none"
        user_message = f"""Agent Idle Event:
- Idle duration: {idle_duration}ms ({idle_duration/1000:.1f} seconds)
- Reason: {idle_reason}
- Suggested action: {suggested_action}
- Recent topics: {topics_str}

Should we ALLOW background consolidation or DEFER? Consider:
- Long idle (>30s) = allow consolidation
- Brief idle (<10s) = defer"""

        response = await self.llm_client.complete(
            [{"role": "user", "content": user_message}],
            system=system_prompt
        )

        print(f"  [idle] LLM decision: {response[:100]}...")

        if response.strip().upper().startswith("DEFER"):
            reason = response.strip()[5:].strip() if len(response) > 5 else "LLM deferred"
            return Decision.idle_defer(reason)

        return Decision.idle_allow()

    def _build_context(self, event: AhpEvent) -> dict:
        """Build context for decision making"""
        session_id = event.session_id

        if session_id not in self.session_contexts:
            self.session_contexts[session_id] = {
                "event_count": 0,
                "memory_summary": {"recent_topics": []},
                "session_stats": {"total_actions": 0}
            }

        ctx = self.session_contexts[session_id]
        ctx["event_count"] += 1

        if event.event_type == "post_action":
            ctx["session_stats"]["total_actions"] += 1

        # Build EventContext structure (for AHP protocol)
        return {
            "recent_facts": None,
            "memory_summary": ctx["memory_summary"],
            "session_stats": ctx["session_stats"],
            "current_task": None
        }


# =============================================================================
# Simple AHP HTTP Server (for testing)
# =============================================================================

async def run_http_server(handler: LlmEventHandler, port: int = 8080):
    """Run a simple HTTP server that handles AHP JSON-RPC requests"""
    from aiohttp import web

    async def handle_ahp(request):
        """Handle AHP JSON-RPC requests"""
        body = await request.json()
        print(f"\n[HTTP Server] Received: {json.dumps(body, indent=2)[:300]}...")

        jsonrpc_id = body.get("id", "unknown")
        method = body.get("method", "")
        params = body.get("params", {})

        if method == "ahp/handshake":
            response = {
                "jsonrpc": "2.0",
                "id": jsonrpc_id,
                "result": {
                    "protocol_version": "2.0",
                    "harness_info": {
                        "name": "llm-test-harness",
                        "version": "1.0.0",
                        "capabilities": ["pre_action", "post_action", "pre_prompt", "idle"]
                    }
                }
            }
        elif method == "ahp/event":
            # Parse event
            event = AhpEvent.from_dict(params)
            decision = await handler.handle_event(event)
            response = {
                "jsonrpc": "2.0",
                "id": jsonrpc_id,
                "result": decision
            }
        elif method == "ahp/query":
            response = {
                "jsonrpc": "2.0",
                "id": jsonrpc_id,
                "result": {"answer": "query response"}
            }
        else:
            response = {
                "jsonrpc": "2.0",
                "id": jsonrpc_id,
                "error": {"code": -32601, "message": f"Method not found: {method}"}
            }

        print(f"[HTTP Server] Response: {json.dumps(response, indent=2)[:300]}...")
        return web.json_response(response)

    app = web.Application()
    app.router.add_post("/ahp", handle_ahp)

    runner = web.AppRunner(app)
    await runner.setup()
    site = web.TCPSite(runner, "localhost", port)
    await site.start()
    print(f"\n[AHP Server] Listening on http://localhost:{port}/ahp")
    return runner


# =============================================================================
# Mock Agent (simulates A3S Code sending events)
# =============================================================================

class MockA3SAgent:
    """
    Mock A3S Code agent that sends events to the AHP server.
    Simulates idle detection by tracking last activity time.
    """

    def __init__(self, server_url: str, session_id: str, agent_id: str):
        self.server_url = server_url.rstrip("/")
        self.session_id = session_id
        self.agent_id = agent_id
        self.last_activity = datetime.now()
        self.idle_threshold_ms = 5000  # 5 seconds for testing
        self.client = httpx.AsyncClient(timeout=30.0)

        # Client自主 exposes its capabilities for the server to use
        # These capabilities can be arbitrary - the server decides what to use
        self.capabilities = {
            "memory_search": {
                "type": "http",
                "url": "http://localhost:8080/memory/search",
                "methods": ["GET", "POST"]
            },
            "session_info": {
                "type": "query",
                "handler": "get_session_info",
                "description": "Get current session information"
            },
            "cross_session": {
                "type": "http",
                "url": "http://localhost:8080/sessions/query",
                "description": "Query across sessions"
            },
            "custom_ai_tool": {
                "type": "ai",
                "model": "minimax",
                "endpoint": "http://localhost:8080/ai/execute",
                "description": "Delegate to AI for complex reasoning"
            }
        }

    async def send_event(self, event_type: str, payload: dict, context: dict = None) -> dict:
        """Send an event to the AHP server"""
        event = {
            "event_type": event_type,
            "session_id": self.session_id,
            "agent_id": self.agent_id,
            "timestamp": datetime.now().isoformat(),
            "depth": 0,
            "payload": payload,
        }

        # Include context with capabilities
        if context or self.capabilities:
            event["context"] = context or {}
            if self.capabilities:
                event["context"]["capabilities"] = self.capabilities

        request = {
            "jsonrpc": "2.0",
            "id": f"req-{event_type}",
            "method": "ahp/event",
            "params": event
        }

        response = await self.client.post(
            f"{self.server_url}/ahp",
            json=request
        )

        self.last_activity = datetime.now()  # Update activity on any event
        return response.json().get("result", {})

    async def check_and_send_idle(self) -> bool:
        """Check if idle threshold exceeded and send idle event if so"""
        elapsed = (datetime.now() - self.last_activity).total_seconds() * 1000
        if elapsed >= self.idle_threshold_ms:
            payload = {
                "idle_duration_ms": int(elapsed),
                "idle_reason": "no_activity",
                "last_event_type": "post_action",
                "suggested_action": "dream"
            }
            result = await self.send_event("idle", payload)
            print(f"[Agent] Sent idle event, decision: {result.get('decision', 'unknown')}")
            return True
        return False

    async def close(self):
        await self.client.aclose()


# =============================================================================
# Integration Test
# =============================================================================

async def test_idle_detection():
    """Integration test for AHP idle detection with minmax model"""

    print("=" * 70)
    print("AHP Idle Detection Integration Test")
    print("=" * 70)
    print(f"\nConfiguration:")
    print(f"  Model: {MINIMAX_MODEL}")
    print(f"  Base URL: {MINIMAX_BASE_URL}")
    print(f"  Server URL: {AHP_SERVER_URL}")

    # Initialize LLM client
    print("\n[1] Initializing Minmax client...")
    llm_client = MinmaxClient(MINIMAX_API_KEY, MINIMAX_BASE_URL, MINIMAX_MODEL)

    # Test LLM connectivity
    print("[2] Testing LLM connectivity...")
    try:
        response = await llm_client.complete(
            [{"role": "user", "content": "Reply with just the word 'OK'"}]
        )
        print(f"    LLM response: {response.strip()}")
        if response.strip().upper() != "OK":
            print("    WARNING: Unexpected response")
    except Exception as e:
        print(f"    ERROR: {e}")
        return False

    # Initialize handler
    print("[3] Initializing LLM event handler...")
    handler = LlmEventHandler(llm_client)

    # Start HTTP server in background
    print("[4] Starting AHP HTTP server...")
    runner = await run_http_server(handler, 8080)

    # Give server time to start
    await asyncio.sleep(1)

    # Create mock agent
    print("[5] Creating mock A3S agent...")
    agent = MockA3SAgent("http://localhost:8080", "test-session-123", "test-agent-001")

    # Send some events to establish context
    print("\n[6] Sending initial events...")

    print("  - Sending session_start...")
    await agent.send_event("session_start", {
        "session_id": "test-session-123",
        "system_prompt": "You are a helpful assistant",
        "model_provider": "openai",
        "model_name": MINIMAX_MODEL
    })

    print("  - Sending pre_action (read file)...")
    result = await agent.send_event("pre_action", {
        "tool": "read",
        "arguments": {"file_path": "/tmp/test.txt"}
    })
    print(f"    Decision: {result.get('decision')}")

    print("  - Sending post_action...")
    await agent.send_event("post_action", {
        "tool": "read",
        "arguments": {"file_path": "/tmp/test.txt"},
        "result": {"success": True, "output": "file contents"}
    })

    print("  - Sending pre_prompt...")
    result = await agent.send_event("pre_prompt", {
        "prompt": "What files are in the current directory?",
        "system_prompt": "You are a helpful assistant",
        "message_count": 2
    })
    print(f"    Decision: {result.get('decision')}")

    # Now wait for idle threshold to be exceeded
    print(f"\n[7] Waiting for idle threshold ({agent.idle_threshold_ms}ms)...")
    print("    (Simulating idle time...)")

    # Simulate idle by waiting
    await asyncio.sleep(6)

    # Check and send idle event
    print("\n[8] Checking for idle condition...")
    is_idle = await agent.check_and_send_idle()

    if is_idle:
        print(f"\n[9] Idle event was sent! Total idle events received: {len(handler.idle_events)}")
        if handler.idle_events:
            last_idle = handler.idle_events[-1]
            print(f"    Last idle event:")
            print(f"      - Duration: {last_idle['duration_ms']}ms")
            print(f"      - Reason: {last_idle['reason']}")
            print(f"      - Suggested action: {last_idle['suggested_action']}")

    # Cleanup
    print("\n[10] Cleaning up...")
    await agent.close()
    await runner.cleanup()
    await llm_client.close()

    print("\n" + "=" * 70)
    print("Test Complete!")
    print("=" * 70)

    # Verify results
    if len(handler.idle_events) > 0:
        print("\n✅ SUCCESS: Idle detection working!")
        return True
    else:
        print("\n❌ FAILED: No idle events received")
        return False


async def test_llm_decision_making():
    """Test LLM-powered decision making for pre_action"""

    print("\n" + "=" * 70)
    print("AHP LLM Decision Making Test")
    print("=" * 70)

    llm_client = MinmaxClient(MINIMAX_API_KEY, MINIMAX_BASE_URL, MINIMAX_MODEL)
    handler = LlmEventHandler(llm_client)

    # Test safe action
    print("\n[Test 1] Safe action (read file)...")
    safe_event = AhpEvent(
        event_type="pre_action",
        session_id="test-session",
        agent_id="test-agent",
        timestamp=datetime.now().isoformat(),
        depth=0,
        payload={"tool": "read", "arguments": {"file_path": "/tmp/test.txt"}}
    )
    decision = await handler.handle_event(safe_event)
    print(f"  Decision: {decision.get('decision')}")
    assert decision.get('decision') == 'allow', f"Expected allow, got {decision}"

    # Test potentially dangerous action
    print("\n[Test 2] Potentially dangerous action (rm)...")
    dangerous_event = AhpEvent(
        event_type="pre_action",
        session_id="test-session",
        agent_id="test-agent",
        timestamp=datetime.now().isoformat(),
        depth=0,
        payload={"tool": "bash", "arguments": {"command": "rm -rf /tmp/*"}}
    )
    decision = await handler.handle_event(dangerous_event)
    print(f"  Decision: {decision.get('decision')}")
    # LLM should block this

    await llm_client.close()
    print("\n✅ LLM decision making test complete!")
    return True


# =============================================================================
# Main
# =============================================================================

if __name__ == "__main__":
    import sys

    # Check for required dependencies
    try:
        import httpx
        import aiohttp
    except ImportError:
        print("Installing required dependencies...")
        import subprocess
        subprocess.check_call([sys.executable, "-m", "pip", "install", "httpx", "aiohttp"])
        print("Dependencies installed. Run again.")

    async def main():
        # Run tests
        success1 = await test_llm_decision_making()
        success2 = await test_idle_detection()

        if success1 and success2:
            print("\n\n🎉 All tests passed!")
            sys.exit(0)
        else:
            print("\n\n❌ Some tests failed")
            sys.exit(1)

    asyncio.run(main())
