//! Simple AHP client example

use a3s_ahp::{AhpClient, Transport, EventType, Decision};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create client with stdio transport
    let client = AhpClient::new(Transport::Stdio {
        program: "python3".into(),
        args: vec!["examples/simple_server.py".into()],
    }).await?;

    // Perform handshake
    let handshake = client.handshake().await?;
    println!("Connected to: {} v{}", 
        handshake.harness_info.name,
        handshake.harness_info.version
    );

    // Send pre-action event
    let decision = client.send_event(
        EventType::PreAction,
        serde_json::json!({
            "action_type": "tool_call",
            "tool_name": "bash",
            "arguments": {
                "command": "ls -la"
            }
        })
    ).await?;

    match decision {
        Decision::Allow { .. } => println!("✓ Action allowed"),
        Decision::Block { reason, .. } => println!("✗ Action blocked: {}", reason),
        _ => println!("? Other decision"),
    }

    // Send post-action notification
    client.send_event(
        EventType::PostAction,
        serde_json::json!({
            "action_type": "tool_call",
            "tool_name": "bash",
            "result": {
                "status": "success",
                "output": "total 42\ndrwxr-xr-x ...",
                "exit_code": 0
            }
        })
    ).await?;

    println!("✓ Notification sent");

    // Query the harness
    let query_response = client.query(
        "should_i_proceed",
        serde_json::json!({
            "question": "Should I delete this file?",
            "file_path": "/workspace/important.txt"
        })
    ).await?;

    println!("Query answer: {:?}", query_response.answer);

    client.close().await?;
    Ok(())
}
