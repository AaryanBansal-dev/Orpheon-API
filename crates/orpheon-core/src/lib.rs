//! # Orpheon Core
//!
//! Core primitives and types for the Orpheon Protocol.
//!
//! This crate provides the fundamental building blocks:
//! - [`Intent`] - Declaration of desired future state
//! - [`Plan`] - DAG of execution steps
//! - [`ExecutionArtifact`] - Proof of outcome
//! - [`OrpheonError`] - Protocol error types

pub mod artifact;
pub mod error;
pub mod intent;
pub mod plan;
pub mod types;

// Re-exports for convenience
pub use artifact::{ExecutionArtifact, ExecutionEvent, Outcome};
pub use error::{OrpheonError, Result};
pub use intent::{Budget, Constraint, Intent, IntentBuilder, Preference, Signature, TimeWindow};
pub use plan::{Plan, PlanningStrategy, Step};
pub use types::*;

/// Prelude module for common imports
pub mod prelude {
    pub use crate::artifact::{ExecutionArtifact, ExecutionEvent, Outcome};
    pub use crate::error::{OrpheonError, Result};
    pub use crate::intent::{
        Budget, Constraint, Intent, IntentBuilder, Preference, Signature, TimeWindow,
    };
    pub use crate::plan::{Plan, PlanningStrategy, Step};
}
