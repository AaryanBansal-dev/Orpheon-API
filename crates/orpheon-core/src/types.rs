//! Common types used across the Orpheon Protocol.

use serde::{Deserialize, Serialize};

/// Status of an Intent in the system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntentStatus {
    /// Intent has been received and is being validated.
    Received,
    /// Intent is valid and planning has started.
    Planning,
    /// A plan has been generated and is being negotiated.
    Negotiating,
    /// Plan has been accepted and is executing.
    Executing,
    /// Execution is being compensated due to failure.
    Compensating,
    /// Intent has been successfully fulfilled.
    Complete,
    /// Intent has failed and cannot be recovered.
    Failed,
    /// Intent was cancelled by the client.
    Cancelled,
}

impl IntentStatus {
    /// Returns true if this is a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            IntentStatus::Complete | IntentStatus::Failed | IntentStatus::Cancelled
        )
    }

    /// Returns true if the intent is currently being processed.
    pub fn is_active(&self) -> bool {
        matches!(
            self,
            IntentStatus::Received
                | IntentStatus::Planning
                | IntentStatus::Negotiating
                | IntentStatus::Executing
                | IntentStatus::Compensating
        )
    }
}

/// Priority level for an intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    /// Lowest priority, processed when resources are available.
    Low,
    /// Normal priority (default).
    #[default]
    Normal,
    /// Higher priority, processed before normal.
    High,
    /// Highest priority, processed immediately.
    Critical,
}

/// Resource type for budget and constraint tracking.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceType {
    /// Monetary cost in a specific currency.
    Money { currency: String },
    /// Compute resources (CPU cores).
    Compute,
    /// Memory in bytes.
    Memory,
    /// Storage in bytes.
    Storage,
    /// Network bandwidth in bytes/sec.
    Bandwidth,
    /// Time in milliseconds.
    Time,
    /// Custom resource type.
    Custom(String),
}

/// Event types that can occur during intent processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventType {
    /// Plan is being negotiated.
    Negotiating {
        proposal_id: uuid::Uuid,
        estimated_cost: f64,
        estimated_latency_ms: u64,
    },
    /// A step is being executed.
    Executing {
        step_id: uuid::Uuid,
        step_name: String,
        progress: f32,
    },
    /// Step completed successfully.
    StepComplete {
        step_id: uuid::Uuid,
        duration_ms: u64,
    },
    /// Execution completed successfully.
    Complete { artifact_id: uuid::Uuid },
    /// An error occurred.
    Error { message: String, recoverable: bool },
    /// Intent was cancelled.
    Cancelled { reason: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intent_status_terminal() {
        assert!(IntentStatus::Complete.is_terminal());
        assert!(IntentStatus::Failed.is_terminal());
        assert!(IntentStatus::Cancelled.is_terminal());
        assert!(!IntentStatus::Executing.is_terminal());
    }

    #[test]
    fn test_intent_status_active() {
        assert!(IntentStatus::Executing.is_active());
        assert!(IntentStatus::Planning.is_active());
        assert!(!IntentStatus::Complete.is_active());
    }
}
