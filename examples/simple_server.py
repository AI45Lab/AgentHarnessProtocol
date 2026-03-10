#!/usr/bin/env python3
"""Simple AHP server example"""

import json
import sys

def handle_handshake(params):
    return {
        "protocol_version": "2.0",
        "harness_info": {
            "name": "simple-harness",
            "version": "1.0.0",
            "capabilities": ["pre_action", "post_action", "query"]
        },
        "config": {
            "timeout_ms": 10000,
            "batch_size": 100
        }
    }

def handle_event(event):
    event_type = event.get("event_type")
    payload = event.get("payload", {})
    
    if event_type == "pre_action":
        # Check if command is dangerous
        command = payload.get("arguments", {}).get("command", "")
        if "rm -rf" in command or "dd if=" in command:
            return {
                "decision": "block",
                "reason": "Dangerous command detected"
            }
        return {
            "decision": "allow"
        }
    
    return {"decision": "allow"}

def handle_query(query):
    question = query.get("payload", {}).get("question", "")
    
    if "delete" in question.lower():
        return {
            "answer": "no",
            "reason": "Deletion requires explicit confirmation",
            "alternatives": ["Move to trash", "Create backup first"]
        }
    
    return {
        "answer": "yes",
        "reason": "No concerns detected"
    }

def main():
    for line in sys.stdin:
        try:
            msg = json.loads(line.strip())
            req_id = msg.get("id")
            method = msg.get("method", "")
            params = msg.get("params", {})
            
            # Handle request (blocking)
            if req_id:
                if method == "ahp/handshake":
                    result = handle_handshake(params)
                elif method == "ahp/event":
                    result = handle_event(params)
                elif method == "ahp/query":
                    result = handle_query(params)
                else:
                    result = {"error": "Unknown method"}
                
                response = {
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": result
                }
                print(json.dumps(response), flush=True)
            
            # Handle notification (fire-and-forget)
            else:
                if method == "ahp/event":
                    # Just log it
                    event_type = params.get("event_type")
                    sys.stderr.write(f"[INFO] Received notification: {event_type}\n")
                    sys.stderr.flush()
        
        except Exception as e:
            sys.stderr.write(f"[ERROR] {e}\n")
            sys.stderr.flush()

if __name__ == "__main__":
    main()
