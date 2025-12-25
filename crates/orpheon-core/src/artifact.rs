//! Execution artifact types for the Orpheon Protocol.
//!
//! An ExecutionArtifact is the "Proof of Outcome" generated when an intent is finalized.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::intent::Intent;
use crate::plan::Plan;

/// The execution artifact provides proof of outcome.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionArtifact {
    /// Unique identifier for this artifact.
    pub id: Uuid,

    /// The original intent that was fulfilled.
    pub intent: Intent,

    /// The final plan that was executed.
    pub final_plan: Plan,

    /// Trace of all execution events.
    pub trace: Vec<ExecutionEvent>,

    /// The outcome of the execution.
    pub outcome: Outcome,

    /// Timestamp when execution completed.
    pub timestamp: DateTime<Utc>,

    /// Merkle root of the execution trace for verifiable logging.
    pub merkle_root: String,

    /// Total actual cost incurred.
    pub actual_cost: f64,

    /// Total actual duration in milliseconds.
    pub actual_duration_ms: u64,

    /// Metadata about the execution environment.
    pub execution_metadata: ExecutionMetadata,
}

/// An event that occurred during execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionEvent {
    /// Unique identifier for this event.
    pub id: Uuid,

    /// The step this event is associated with.
    pub step_id: Uuid,

    /// Type of event.
    pub event_type: ExecutionEventType,

    /// Timestamp when the event occurred.
    pub timestamp: DateTime<Utc>,

    /// Duration of the operation in milliseconds (if applicable).
    pub duration_ms: Option<u64>,

    /// Additional data about the event.
    pub data: serde_json::Value,
}

/// Types of execution events.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionEventType {
    /// Step execution started.
    StepStarted,
    /// Step execution completed successfully.
    StepCompleted,
    /// Step execution failed.
    StepFailed,
    /// Step is being retried.
    StepRetrying,
    /// Compensation action started.
    CompensationStarted,
    /// Compensation action completed.
    CompensationCompleted,
    /// State was updated.
    StateUpdated,
    /// Resource was allocated.
    ResourceAllocated,
    /// Resource was released.
    ResourceReleased,
    /// External API was called.
    ExternalCall,
    /// Timeout occurred.
    Timeout,
    /// Custom event type.
    Custom(String),
}

/// Outcome of an intent execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    /// Execution completed successfully.
    Success,
    /// Execution failed.
    Failure {
        /// The reason for failure.
        reason: String,
        /// Whether compensation was attempted.
        compensated: bool,
    },
    /// Execution partially succeeded.
    PartialSuccess {
        /// Percentage of steps that succeeded.
        success_rate: u8,
        /// Description of what succeeded/failed.
        details: String,
    },
    /// Execution was cancelled.
    Cancelled {
        /// Who cancelled (client, system, timeout).
        by: String,
        /// Reason for cancellation.
        reason: String,
    },
}

impl Outcome {
    /// Check if the outcome is successful.
    pub fn is_success(&self) -> bool {
        matches!(self, Outcome::Success)
    }

    /// Check if the outcome is a failure.
    pub fn is_failure(&self) -> bool {
        matches!(self, Outcome::Failure { .. })
    }
}

/// Metadata about the execution environment.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExecutionMetadata {
    /// Node ID that executed the intent.
    pub node_id: String,

    /// Version of the Orpheon node.
    pub node_version: String,

    /// Region where execution occurred.
    pub region: Option<String>,

    /// Additional metadata.
    #[serde(flatten)]
    pub extra: serde_json::Value,
}

impl ExecutionArtifact {
    /// Create a new execution artifact.
    pub fn new(intent: Intent, plan: Plan, outcome: Outcome) -> Self {
        let now = Utc::now();
        let mut artifact = Self {
            id: Uuid::new_v4(),
            intent,
            final_plan: plan,
            trace: Vec::new(),
            outcome,
            timestamp: now,
            merkle_root: String::new(),
            actual_cost: 0.0,
            actual_duration_ms: 0,
            execution_metadata: ExecutionMetadata::default(),
        };
        artifact.merkle_root = artifact.compute_merkle_root();
        artifact
    }

    /// Add an execution event to the trace.
    pub fn add_event(&mut self, event: ExecutionEvent) {
        if let Some(duration) = event.duration_ms {
            self.actual_duration_ms += duration;
        }
        self.trace.push(event);
        // Recompute merkle root
        self.merkle_root = self.compute_merkle_root();
    }

    /// Compute the Merkle root of the execution trace.
    pub fn compute_merkle_root(&self) -> String {
        if self.trace.is_empty() {
            return "0".repeat(64);
        }

        // Hash each event
        let mut hashes: Vec<Vec<u8>> = self
            .trace
            .iter()
            .map(|event| {
                let json = serde_json::to_string(event).unwrap_or_default();
                let mut hasher = Sha256::new();
                hasher.update(json.as_bytes());
                hasher.finalize().to_vec()
            })
            .collect();

        // Build Merkle tree
        while hashes.len() > 1 {
            let mut next_level = Vec::new();

            for chunk in hashes.chunks(2) {
                let mut hasher = Sha256::new();
                hasher.update(&chunk[0]);
                if chunk.len() > 1 {
                    hasher.update(&chunk[1]);
                } else {
                    // Duplicate last hash if odd number
                    hasher.update(&chunk[0]);
                }
                next_level.push(hasher.finalize().to_vec());
            }

            hashes = next_level;
        }

        hashes
            .first()
            .map(|h| h.iter().map(|b| format!("{:02x}", b)).collect())
            .unwrap_or_else(|| "0".repeat(64))
    }

    /// Verify the Merkle root matches the trace.
    pub fn verify_merkle_root(&self) -> bool {
        self.merkle_root == self.compute_merkle_root()
    }

    /// Get all failed steps from the trace.
    pub fn failed_steps(&self) -> Vec<&ExecutionEvent> {
        self.trace
            .iter()
            .filter(|e| e.event_type == ExecutionEventType::StepFailed)
            .collect()
    }

    /// Get all successful steps from the trace.
    pub fn successful_steps(&self) -> Vec<&ExecutionEvent> {
        self.trace
            .iter()
            .filter(|e| e.event_type == ExecutionEventType::StepCompleted)
            .collect()
    }

    /// Calculate the success rate (0.0 to 1.0).
    pub fn success_rate(&self) -> f32 {
        let completed = self
            .trace
            .iter()
            .filter(|e| e.event_type == ExecutionEventType::StepCompleted)
            .count();
        let failed = self
            .trace
            .iter()
            .filter(|e| e.event_type == ExecutionEventType::StepFailed)
            .count();

        let total = completed + failed;
        if total == 0 {
            return 0.0;
        }

        completed as f32 / total as f32
    }
}

impl ExecutionEvent {
    /// Create a new step started event.
    pub fn step_started(step_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            step_id,
            event_type: ExecutionEventType::StepStarted,
            timestamp: Utc::now(),
            duration_ms: None,
            data: serde_json::Value::Null,
        }
    }

    /// Create a new step completed event.
    pub fn step_completed(step_id: Uuid, duration_ms: u64) -> Self {
        Self {
            id: Uuid::new_v4(),
            step_id,
            event_type: ExecutionEventType::StepCompleted,
            timestamp: Utc::now(),
            duration_ms: Some(duration_ms),
            data: serde_json::Value::Null,
        }
    }

    /// Create a new step failed event.
    pub fn step_failed(step_id: Uuid, error: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            step_id,
            event_type: ExecutionEventType::StepFailed,
            timestamp: Utc::now(),
            duration_ms: None,
            data: serde_json::json!({ "error": error.into() }),
        }
    }

    /// Add data to the event.
    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = data;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intent::Intent;
    use crate::plan::{Plan, PlanningStrategy};

    fn create_test_intent() -> Intent {
        Intent::builder()
            .kind("test_intent")
            .build()
            .unwrap()
    }

    #[test]
    fn test_artifact_creation() {
        let intent = create_test_intent();
        let plan = Plan::new(intent.id, PlanningStrategy::Deterministic);
        let artifact = ExecutionArtifact::new(intent, plan, Outcome::Success);

        assert!(artifact.outcome.is_success());
        assert!(!artifact.merkle_root.is_empty());
    }

    #[test]
    fn test_merkle_root_computation() {
        let intent = create_test_intent();
        let plan = Plan::new(intent.id, PlanningStrategy::Deterministic);
        let mut artifact = ExecutionArtifact::new(intent, plan, Outcome::Success);

        let step_id = Uuid::new_v4();
        artifact.add_event(ExecutionEvent::step_started(step_id));
        artifact.add_event(ExecutionEvent::step_completed(step_id, 100));

        assert!(artifact.verify_merkle_root());
    }

    #[test]
    fn test_success_rate() {
        let intent = create_test_intent();
        let plan = Plan::new(intent.id, PlanningStrategy::Deterministic);
        let mut artifact = ExecutionArtifact::new(intent, plan, Outcome::Success);

        let step1 = Uuid::new_v4();
        let step2 = Uuid::new_v4();
        let step3 = Uuid::new_v4();

        artifact.add_event(ExecutionEvent::step_completed(step1, 100));
        artifact.add_event(ExecutionEvent::step_completed(step2, 100));
        artifact.add_event(ExecutionEvent::step_failed(step3, "error"));

        let rate = artifact.success_rate();
        assert!((rate - 0.666).abs() < 0.01);
    }

    #[test]
    fn test_outcome_checks() {
        assert!(Outcome::Success.is_success());
        assert!(!Outcome::Success.is_failure());

        let failure = Outcome::Failure {
            reason: "test".to_string(),
            compensated: false,
        };
        assert!(failure.is_failure());
        assert!(!failure.is_success());
    }
}
