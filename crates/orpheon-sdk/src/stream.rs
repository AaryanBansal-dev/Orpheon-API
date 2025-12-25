//! Event stream for real-time updates.

use futures::StreamExt;
use orpheon_core::{ExecutionArtifact, OrpheonError, Result};
use serde::Deserialize;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use uuid::Uuid;

/// Event types from the stream.
#[derive(Debug, Clone)]
pub enum Event {
    /// Plan is being negotiated.
    Negotiating {
        proposal_id: Uuid,
        estimated_cost: f64,
        estimated_latency_ms: u64,
    },
    /// A step is being executed.
    Executing {
        step_id: Uuid,
        step_name: String,
        progress: f32,
    },
    /// Execution completed.
    Complete {
        artifact_id: Uuid,
    },
    /// Status update.
    StatusUpdate {
        status: String,
        plan_id: Option<Uuid>,
        artifact_id: Option<Uuid>,
    },
    /// An error occurred.
    Error {
        message: String,
    },
}

/// WebSocket message from server.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WsMessage {
    StatusUpdate {
        intent_id: Uuid,
        status: String,
        plan_id: Option<Uuid>,
        artifact_id: Option<Uuid>,
    },
    Error {
        message: String,
    },
    Ping,
}

/// Stream of events for an intent.
pub struct EventStream {
    intent_id: Uuid,
    receiver: tokio::sync::mpsc::Receiver<Event>,
    _handle: tokio::task::JoinHandle<()>,
}

impl EventStream {
    /// Connect to the event stream for an intent.
    pub async fn connect(ws_url: &str, intent_id: Uuid) -> Result<Self> {
        let (ws_stream, _) = connect_async(ws_url)
            .await
            .map_err(|e| OrpheonError::ConnectionError(e.to_string()))?;
        
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        
        let handle = tokio::spawn(async move {
            let (_, mut read) = ws_stream.split();
            
            while let Some(msg) = read.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        if let Ok(ws_msg) = serde_json::from_str::<WsMessage>(&text) {
                            let event = match ws_msg {
                                WsMessage::StatusUpdate { status, plan_id, artifact_id, .. } => {
                                    if status == "complete" {
                                        if let Some(aid) = artifact_id {
                                            Event::Complete { artifact_id: aid }
                                        } else {
                                            Event::StatusUpdate { status, plan_id, artifact_id }
                                        }
                                    } else {
                                        Event::StatusUpdate { status, plan_id, artifact_id }
                                    }
                                }
                                WsMessage::Error { message } => Event::Error { message },
                                WsMessage::Ping => continue,
                            };
                            
                            if tx.send(event).await.is_err() {
                                break;
                            }
                        }
                    }
                    Ok(Message::Close(_)) | Err(_) => break,
                    _ => {}
                }
            }
        });
        
        Ok(Self {
            intent_id,
            receiver: rx,
            _handle: handle,
        })
    }
    
    /// Get the intent ID this stream is for.
    pub fn intent_id(&self) -> Uuid {
        self.intent_id
    }
    
    /// Get the next event.
    pub async fn next(&mut self) -> Option<Event> {
        self.receiver.recv().await
    }
}
