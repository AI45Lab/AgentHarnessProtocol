//! WebSocket server example

use a3s_ahp::server::EventHandler;
use a3s_ahp::{AhpEvent, AhpServer, Decision, QueryRequest, QueryResponse, Result};
use async_trait::async_trait;
use std::sync::Arc;

/// Simple event handler that blocks dangerous commands
struct SimpleHandler;

#[async_trait]
impl EventHandler for SimpleHandler {
    async fn handle_event(&self, event: &AhpEvent) -> Result<Decision> {
        match event.event_type {
            a3s_ahp::EventType::PreAction => {
                // Check if command is dangerous
                if let Some(command) = event
                    .payload
                    .get("arguments")
                    .and_then(|args| args.get("command"))
                    .and_then(|cmd| cmd.as_str())
                {
                    tracing::info!("Checking command: {}", command);

                    if command.contains("rm -rf") || command.contains("dd if=") {
                        tracing::warn!("Blocking dangerous command: {}", command);
                        return Ok(Decision::Block {
                            reason: format!("Dangerous command detected: {}", command),
                            metadata: None,
                        });
                    }
                }
                Ok(Decision::Allow {
                    modified_payload: None,
                    metadata: None,
                })
            }
            _ => Ok(Decision::Allow {
                modified_payload: None,
                metadata: None,
            }),
        }
    }

    async fn handle_notification(&self, event: &AhpEvent) -> Result<()> {
        tracing::info!("Received notification: {:?}", event.event_type);
        Ok(())
    }

    async fn handle_query(&self, query: &QueryRequest) -> Result<QueryResponse> {
        let question = query
            .payload
            .get("question")
            .and_then(|q| q.as_str())
            .unwrap_or("");

        tracing::info!("Received query: {}", question);

        if question.to_lowercase().contains("delete") {
            Ok(QueryResponse {
                answer: serde_json::json!("no"),
                reason: Some("Deletion requires explicit confirmation".to_string()),
                alternatives: Some(vec![
                    "Move to trash".to_string(),
                    "Create backup first".to_string(),
                ]),
            })
        } else {
            Ok(QueryResponse {
                answer: serde_json::json!("yes"),
                reason: Some("No concerns detected".to_string()),
                alternatives: None,
            })
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Create server with simple handler
    let handler = Arc::new(SimpleHandler);
    let server = Arc::new(AhpServer::new(handler));

    // Create WebSocket server
    let ws_server = a3s_ahp::transport::websocket::WebSocketServer::new(server);

    println!("Starting WebSocket server on ws://0.0.0.0:8081/ahp");
    println!("Press Ctrl+C to stop");

    // Run server
    ws_server.run(([0, 0, 0, 0], 8081)).await?;

    Ok(())
}
