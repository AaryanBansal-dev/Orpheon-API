//! WebSocket endpoints.

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    response::Response,
};
use orpheon_state::StateStore;
use serde::{Deserialize, Serialize};
use tokio::time::{interval, Duration};
use uuid::Uuid;

use crate::state::AppState;

/// WebSocket message for intent updates.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum IntentStreamMessage {
    /// Status update.
    StatusUpdate {
        intent_id: Uuid,
        status: String,
        plan_id: Option<Uuid>,
        artifact_id: Option<Uuid>,
    },
    /// Error message.
    Error { message: String },
    /// Ping for keepalive.
    Ping,
}

/// Intent status stream.
pub async fn intent_stream(
    ws: WebSocketUpgrade,
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> Response {
    ws.on_upgrade(move |socket| handle_intent_stream(socket, id, state))
}

async fn handle_intent_stream(mut socket: WebSocket, intent_id: Uuid, state: AppState) {
    let mut poll_interval = interval(Duration::from_millis(500));
    let mut last_status = String::new();

    loop {
        tokio::select! {
            _ = poll_interval.tick() => {
                // Check intent status
                if let Some(record) = state.get_intent(intent_id).await {
                    let status = format!("{:?}", record.status).to_lowercase();
                    
                    // Only send if status changed
                    if status != last_status {
                        last_status = status.clone();
                        
                        let msg = IntentStreamMessage::StatusUpdate {
                            intent_id,
                            status,
                            plan_id: record.plan_id,
                            artifact_id: record.artifact_id,
                        };
                        
                        let json = serde_json::to_string(&msg).unwrap();
                        if socket.send(Message::Text(json.into())).await.is_err() {
                            break;
                        }
                        
                        // Close if terminal
                        if record.status.is_terminal() {
                            break;
                        }
                    }
                } else {
                    let msg = IntentStreamMessage::Error {
                        message: format!("Intent {} not found", intent_id),
                    };
                    let json = serde_json::to_string(&msg).unwrap();
                    let _ = socket.send(Message::Text(json.into())).await;
                    break;
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Ping(data))) => {
                        let _ = socket.send(Message::Pong(data)).await;
                    }
                    _ => {}
                }
            }
        }
    }
}

/// Negotiation stream.
pub async fn negotiate_stream(
    ws: WebSocketUpgrade,
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> Response {
    ws.on_upgrade(move |socket| handle_negotiate_stream(socket, id, state))
}

async fn handle_negotiate_stream(mut socket: WebSocket, intent_id: Uuid, _state: AppState) {
    // Send initial message
    let msg = serde_json::json!({
        "type": "connected",
        "intent_id": intent_id,
        "message": "Connected to negotiation stream"
    });
    
    if socket.send(Message::Text(msg.to_string().into())).await.is_err() {
        return;
    }

    // Handle incoming messages
    while let Some(msg) = socket.recv().await {
        match msg {
            Ok(Message::Text(text)) => {
                // Echo for now (real implementation would handle negotiation protocol)
                let response = serde_json::json!({
                    "type": "ack",
                    "received": text.to_string()
                });
                if socket.send(Message::Text(response.to_string().into())).await.is_err() {
                    break;
                }
            }
            Ok(Message::Close(_)) | Err(_) => break,
            Ok(Message::Ping(data)) => {
                let _ = socket.send(Message::Pong(data)).await;
            }
            _ => {}
        }
    }
}

/// State subscription stream.
pub async fn state_stream(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> Response {
    ws.on_upgrade(move |socket| handle_state_stream(socket, state))
}

async fn handle_state_stream(mut socket: WebSocket, state: AppState) {
    // Send initial message
    let version = state.state_store.version().await;
    let msg = serde_json::json!({
        "type": "connected",
        "version": version,
        "message": "Connected to state stream"
    });
    
    if socket.send(Message::Text(msg.to_string().into())).await.is_err() {
        return;
    }

    let mut poll_interval = interval(Duration::from_secs(1));
    let mut last_version = version;

    loop {
        tokio::select! {
            _ = poll_interval.tick() => {
                let current_version = state.state_store.version().await;
                if current_version != last_version {
                    last_version = current_version;
                    
                    let msg = serde_json::json!({
                        "type": "version_update",
                        "version": current_version
                    });
                    
                    if socket.send(Message::Text(msg.to_string().into())).await.is_err() {
                        break;
                    }
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Ping(data))) => {
                        let _ = socket.send(Message::Pong(data)).await;
                    }
                    _ => {}
                }
            }
        }
    }
}
