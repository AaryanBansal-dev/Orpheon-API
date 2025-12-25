//! Planner trait and configuration.

use async_trait::async_trait;
use orpheon_core::{Intent, OrpheonError, Plan, Result};
use serde::{Deserialize, Serialize};

/// Configuration for the planner.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerConfig {
    /// Maximum number of steps allowed in a plan.
    pub max_steps: usize,

    /// Maximum planning time in milliseconds.
    pub max_planning_time_ms: u64,

    /// Maximum number of states to explore.
    pub max_states_explored: usize,

    /// Enable plan caching/memoization.
    pub enable_memoization: bool,

    /// Confidence threshold (0.0 to 1.0) below which plans are rejected.
    pub min_confidence: f32,
}

impl Default for PlannerConfig {
    fn default() -> Self {
        Self {
            max_steps: 100,
            max_planning_time_ms: 30_000,
            max_states_explored: 10_000,
            enable_memoization: true,
            min_confidence: 0.5,
        }
    }
}

/// State representation for planning.
#[derive(Debug, Clone)]
pub struct PlanningState {
    /// Current state variables.
    pub variables: std::collections::HashMap<String, serde_json::Value>,
    
    /// Cost accumulated so far.
    pub accumulated_cost: f64,
    
    /// Time accumulated so far.
    pub accumulated_time_ms: u64,
}

impl Default for PlanningState {
    fn default() -> Self {
        Self {
            variables: std::collections::HashMap::new(),
            accumulated_cost: 0.0,
            accumulated_time_ms: 0,
        }
    }
}

/// Action that can be taken during planning.
#[derive(Debug, Clone)]
pub struct PlanningAction {
    /// Name of the action.
    pub name: String,
    
    /// Preconditions that must be true.
    pub preconditions: Vec<String>,
    
    /// Effects on state variables.
    pub effects: Vec<String>,
    
    /// Estimated cost.
    pub cost: f64,
    
    /// Estimated duration in milliseconds.
    pub duration_ms: u64,
}

/// Trait for planning engines.
#[async_trait]
pub trait Planner: Send + Sync {
    /// Generate a plan for the given intent.
    async fn plan(&self, intent: &Intent, initial_state: &PlanningState) -> Result<Plan>;

    /// Check if a plan is still valid.
    async fn validate_plan(&self, plan: &Plan, current_state: &PlanningState) -> Result<bool>;

    /// Get the planner configuration.
    fn config(&self) -> &PlannerConfig;

    /// Update the planner configuration.
    fn set_config(&mut self, config: PlannerConfig);
}

/// Result of a planning operation with additional metadata.
#[derive(Debug)]
pub struct PlanningResult {
    /// The generated plan (if successful).
    pub plan: Option<Plan>,

    /// Number of states explored during planning.
    pub states_explored: usize,

    /// Time spent planning in milliseconds.
    pub planning_time_ms: u64,

    /// Error if planning failed.
    pub error: Option<OrpheonError>,
}
