use axum::{
    extract::{
        ws::{Message, WebSocket},
        Path, State, WebSocketUpgrade,
    },
    response::Response,
};
use futures::{sink::SinkExt, stream::StreamExt};
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::AppState;

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Path(tenant_id): Path<i64>,
    State(state): State<Arc<AppState>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, tenant_id, state))
}

async fn handle_socket(socket: WebSocket, tenant_id: i64, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();

    // TODO: Extract user_id and token from connection parameters/headers
    let user_id = 1; // Placeholder - should be extracted from JWT token

    // Create a channel for sending t_sys_messages to this connection
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

    // Register the channel with the manager
    {
        let mut manager = state.ws_manager.write().await;
        manager.add_connection(tenant_id, user_id, tx).await;
    }

    // Spawn a task to handle sending t_sys_messages from the channel
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    // Send welcome message
    let welcome = serde_json::json!({
        "type": "connected",
        "data": {
            "tenant_id": tenant_id,
            "user_id": user_id,
            "timestamp": chrono::Utc::now(),
        }
    });

    if let Ok(_msg) = serde_json::to_string(&welcome) {
        let manager = state.ws_manager.read().await;
        let _ = manager.send_to_user(tenant_id, user_id, welcome).await;
    }

    // Handle incoming t_sys_messages
    let manager_for_handler = state.ws_manager.clone();
    let state_for_handler = state.clone();
    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Ok(data) = serde_json::from_str::<serde_json::Value>(&text) {
                    handle_client_message(data, tenant_id, user_id, &state_for_handler).await;
                }
            }
            Ok(Message::Ping(_bytes)) => {
                // We can't use sender here since it was moved into the spawned task
                // Just log the ping for now
                tracing::debug!("Received ping from user {}", user_id);
            }
            Ok(Message::Pong(_)) => {
                // Pong response - keep alive
            }
            Ok(Message::Close(_)) => {
                break;
            }
            Err(e) => {
                tracing::error!("WebSocket error: {:?}", e);
                break;
            }
            _ => {}
        }
    }

    // Remove connection
    {
        let mut manager = manager_for_handler.write().await;
        manager.remove_connection(tenant_id, user_id).await;
    }
}

async fn handle_client_message(
    data: serde_json::Value,
    tenant_id: i64,
    user_id: i64,
    state: &Arc<AppState>,
) {
    let msg_type = data.get("type").and_then(|v| v.as_str());

    match msg_type {
        Some("ping") => {
            let manager = state.ws_manager.read().await;
            let pong = serde_json::json!({
                "type": "pong",
                "data": {
                    "timestamp": chrono::Utc::now(),
                }
            });
            manager.send_to_user(tenant_id, user_id, pong).await;
        }
        Some("mark_read") => {
            if let Some(msg_id) = data.get("message_id").and_then(|v| v.as_i64()) {
                let _ = state
                    .repos
                    .message
                    .mark_as_read(msg_id, user_id)
                    .await;

                // Update unread count in Redis (if available)
                // This would require the Redis connection in state
            }
        }
        Some("typing") => {
            // Handle typing indicator
            // Could broadcast to other users in a conversation
        }
        _ => {
            tracing::debug!("Unknown message type: {:?}", msg_type);
        }
    }
}
