//! WebSocket client example

use a3s_ahp::{AhpClient, Decision, EventType, Transport};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create client with WebSocket transport
    let client = AhpClient::new(Transport::WebSocket {
        url: "ws://localhost:8081/ahp".into(),
        auth: None,
    })
    .await?;

    println!("Connected to WebSocket harness server");

    // Perform handshake
    let handshake = client.handshake(Vec::new()).await?;
    println!("✓ Handshake successful");
    println!(
        "  Harness: {} v{}",
        handshake.harness_info.name, handshake.harness_info.version
    );

    // Send multiple events in sequence
    for i in 1..=5 {
        println!("\n--- Event {} ---", i);

        let decision = client
            .send_event_decision(
                EventType::PreAction,
                serde_json::json!({
                    "action_type": "tool_call",
                    "tool_name": "bash",
                    "arguments": {
                        "command": format!("echo 'Test {}'", i)
                    }
                }),
            )
            .await?;

        match decision {
            Decision::Allow { .. } => println!("✓ Action {} allowed", i),
            Decision::Block { reason, .. } => println!("✗ Action {} blocked: {}", i, reason),
            _ => println!("? Other decision for action {}", i),
        }

        // Small delay between events
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    // Test batch processing
    println!("\n--- Batch Processing ---");
    let events = vec![
        a3s_ahp::AhpEvent {
            event_type: EventType::PreAction,
            session_id: uuid::Uuid::new_v4().to_string(),
            agent_id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            depth: 0,
            payload: serde_json::json!({
                "action_type": "tool_call",
                "tool_name": "bash",
                "arguments": { "command": "ls" }
            }),
            context: None,
            metadata: None,
        },
        a3s_ahp::AhpEvent {
            event_type: EventType::PreAction,
            session_id: uuid::Uuid::new_v4().to_string(),
            agent_id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            depth: 0,
            payload: serde_json::json!({
                "action_type": "tool_call",
                "tool_name": "bash",
                "arguments": { "command": "pwd" }
            }),
            context: None,
            metadata: None,
        },
        a3s_ahp::AhpEvent {
            event_type: EventType::PreAction,
            session_id: uuid::Uuid::new_v4().to_string(),
            agent_id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            depth: 0,
            payload: serde_json::json!({
                "action_type": "tool_call",
                "tool_name": "bash",
                "arguments": { "command": "rm -rf /" }  // This should be blocked
            }),
            context: None,
            metadata: None,
        },
    ];

    let batch_response = client.send_batch(events).await?;
    println!(
        "✓ Batch processed: {} decisions",
        batch_response.decisions.len()
    );
    for (i, decision) in batch_response.decisions.iter().enumerate() {
        match decision {
            Decision::Allow { .. } => println!("  [{}] Allow", i + 1),
            Decision::Block { reason, .. } => println!("  [{}] Block: {}", i + 1, reason),
            _ => println!("  [{}] Other", i + 1),
        }
    }

    client.close().await?;
    println!("\n✓ Connection closed");

    Ok(())
}
