//! HTTP client example

use a3s_ahp::{AhpClient, Decision, EventType, Transport};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create client with HTTP transport (no auth)
    let client = AhpClient::new(Transport::Http {
        url: "http://localhost:8080/ahp".into(),
        auth: None,
    })
    .await?;

    println!("Connected to HTTP harness server");

    // Perform handshake
    let handshake = client.handshake(Vec::new()).await?;
    println!("✓ Handshake successful");
    println!(
        "  Harness: {} v{}",
        handshake.harness_info.name, handshake.harness_info.version
    );
    println!("  Capabilities: {:?}", handshake.harness_info.capabilities);

    // Send pre-action event
    println!("\nSending pre-action event...");
    let decision = client
        .send_event_decision(
            EventType::PreAction,
            serde_json::json!({
                "action_type": "tool_call",
                "tool_name": "bash",
                "arguments": {
                    "command": "ls -la /tmp"
                }
            }),
        )
        .await?;

    match decision {
        Decision::Allow { .. } => println!("✓ Action allowed"),
        Decision::Block { reason, .. } => println!("✗ Action blocked: {}", reason),
        Decision::Modify {
            modified_payload, ..
        } => {
            println!("⚠ Action modified: {:?}", modified_payload);
        }
        _ => println!("? Other decision"),
    }

    // Send post-action notification
    println!("\nSending post-action notification...");
    client
        .send_event(
            EventType::PostAction,
            serde_json::json!({
                "action_type": "tool_call",
                "tool_name": "bash",
                "result": {
                    "status": "success",
                    "output": "total 42\ndrwxr-xr-x ...",
                    "exit_code": 0,
                    "duration_ms": 45
                }
            }),
        )
        .await?;
    println!("✓ Notification sent");

    // Query the harness
    println!("\nQuerying harness...");
    let query_response = client
        .query(
            "should_i_proceed",
            serde_json::json!({
                "question": "Should I delete this file?",
                "file_path": "/workspace/important.txt"
            }),
        )
        .await?;

    println!("✓ Query response:");
    println!("  Answer: {:?}", query_response.answer);
    if let Some(reason) = query_response.reason {
        println!("  Reason: {}", reason);
    }
    if let Some(alternatives) = query_response.alternatives {
        println!("  Alternatives: {:?}", alternatives);
    }

    client.close().await?;
    println!("\n✓ Connection closed");

    Ok(())
}
