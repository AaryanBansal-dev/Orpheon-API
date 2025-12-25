//! Error types for the Orpheon Protocol.

use thiserror::Error;
use uuid::Uuid;

/// Main error type for Orpheon operations.
#[derive(Error, Debug, Clone)]
pub enum OrpheonError {
    /// Intent validation failed.
    #[error("Intent validation failed: {message}")]
    IntentInvalid { intent_id: Option<Uuid>, message: String },

    /// Planning failed to find a viable path.
    #[error("Planning failed for intent {intent_id}: {message}")]
    PlanningFailed { intent_id: Uuid, message: String },

    /// Execution of a step failed.
    #[error("Execution failed at step {step_id}: {message}")]
    ExecutionFailed {
        intent_id: Uuid,
        step_id: Uuid,
        message: String,
        recoverable: bool,
    },

    /// Negotiation was rejected by the counterparty.
    #[error("Negotiation rejected: {reason}")]
    NegotiationRejected { intent_id: Uuid, reason: String },

    /// Operation timed out.
    #[error("Operation timed out after {duration_ms}ms: {message}")]
    Timeout { duration_ms: u64, message: String },

    /// Constraint violation detected.
    #[error("Constraint violated: {constraint}")]
    ConstraintViolation { intent_id: Uuid, constraint: String },

    /// Budget exceeded.
    #[error("Budget exceeded: spent {spent}, limit {limit}")]
    BudgetExceeded {
        intent_id: Uuid,
        spent: f64,
        limit: f64,
    },

    /// State store error.
    #[error("State store error: {message}")]
    StateError { message: String },

    /// Serialization/deserialization error.
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Cryptographic operation failed.
    #[error("Cryptographic error: {0}")]
    CryptoError(String),

    /// Resource not found.
    #[error("Resource not found: {resource_type} with id {id}")]
    NotFound { resource_type: String, id: String },

    /// Internal error (should not happen).
    #[error("Internal error: {0}")]
    Internal(String),

    /// Connection error.
    #[error("Connection error: {0}")]
    ConnectionError(String),
}

impl OrpheonError {
    /// Returns true if this error is recoverable.
    pub fn is_recoverable(&self) -> bool {
        match self {
            OrpheonError::ExecutionFailed { recoverable, .. } => *recoverable,
            OrpheonError::Timeout { .. } => true,
            OrpheonError::ConnectionError(_) => true,
            _ => false,
        }
    }

    /// Returns the intent ID if available.
    pub fn intent_id(&self) -> Option<Uuid> {
        match self {
            OrpheonError::IntentInvalid { intent_id, .. } => *intent_id,
            OrpheonError::PlanningFailed { intent_id, .. } => Some(*intent_id),
            OrpheonError::ExecutionFailed { intent_id, .. } => Some(*intent_id),
            OrpheonError::NegotiationRejected { intent_id, .. } => Some(*intent_id),
            OrpheonError::ConstraintViolation { intent_id, .. } => Some(*intent_id),
            OrpheonError::BudgetExceeded { intent_id, .. } => Some(*intent_id),
            _ => None,
        }
    }
}

/// Convenience Result type for Orpheon operations.
pub type Result<T> = std::result::Result<T, OrpheonError>;

impl From<serde_json::Error> for OrpheonError {
    fn from(err: serde_json::Error) -> Self {
        OrpheonError::SerializationError(err.to_string())
    }
}
