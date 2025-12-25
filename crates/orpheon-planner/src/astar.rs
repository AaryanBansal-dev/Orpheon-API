//! A* search-based planner implementation.

use std::collections::{BinaryHeap, HashMap, HashSet};
use std::cmp::Ordering;
use std::time::Instant;

use async_trait::async_trait;
use orpheon_core::{Intent, OrpheonError, Plan, PlanningStrategy, Result, Step};
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::planner::{Planner, PlannerConfig, PlanningAction, PlanningState};

/// A* search-based planner.
pub struct AStarPlanner {
    config: PlannerConfig,
    /// Available actions the planner can use.
    actions: Vec<PlanningAction>,
}

/// Node in the A* search tree.
#[derive(Clone)]
struct SearchNode {
    /// Current state.
    state: PlanningState,
    /// Steps taken to reach this state.
    steps: Vec<Step>,
    /// g(n): Actual cost from start.
    g_cost: f64,
    /// h(n): Heuristic estimate to goal.
    h_cost: f64,
    /// f(n) = g(n) + h(n).
    f_cost: f64,
    /// Unique identifier for this node.
    id: Uuid,
}

impl PartialEq for SearchNode {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for SearchNode {}

impl Ord for SearchNode {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering for min-heap (lower f_cost = higher priority)
        other.f_cost.partial_cmp(&self.f_cost).unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for SearchNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl AStarPlanner {
    /// Create a new A* planner with default configuration.
    pub fn new() -> Self {
        Self {
            config: PlannerConfig::default(),
            actions: Self::default_actions(),
        }
    }

    /// Create a new A* planner with custom configuration.
    pub fn with_config(config: PlannerConfig) -> Self {
        Self {
            config,
            actions: Self::default_actions(),
        }
    }

    /// Register an action that the planner can use.
    pub fn register_action(&mut self, action: PlanningAction) {
        self.actions.push(action);
    }

    /// Get default actions for common operations.
    fn default_actions() -> Vec<PlanningAction> {
        vec![
            PlanningAction {
                name: "allocate_resource".to_string(),
                preconditions: vec![],
                effects: vec!["resource_allocated".to_string()],
                cost: 1.0,
                duration_ms: 100,
            },
            PlanningAction {
                name: "provision_compute".to_string(),
                preconditions: vec!["resource_allocated".to_string()],
                effects: vec!["compute_ready".to_string()],
                cost: 5.0,
                duration_ms: 500,
            },
            PlanningAction {
                name: "configure_network".to_string(),
                preconditions: vec!["compute_ready".to_string()],
                effects: vec!["network_configured".to_string()],
                cost: 2.0,
                duration_ms: 200,
            },
            PlanningAction {
                name: "deploy_workload".to_string(),
                preconditions: vec!["compute_ready".to_string(), "network_configured".to_string()],
                effects: vec!["workload_deployed".to_string()],
                cost: 3.0,
                duration_ms: 1000,
            },
            PlanningAction {
                name: "verify_health".to_string(),
                preconditions: vec!["workload_deployed".to_string()],
                effects: vec!["health_verified".to_string()],
                cost: 0.5,
                duration_ms: 100,
            },
            PlanningAction {
                name: "finalize".to_string(),
                preconditions: vec!["health_verified".to_string()],
                effects: vec!["complete".to_string()],
                cost: 0.1,
                duration_ms: 50,
            },
        ]
    }

    /// Heuristic function: estimate cost to reach goal.
    fn heuristic(&self, state: &PlanningState, intent: &Intent) -> f64 {
        // Simple heuristic: count missing goal conditions
        // In a real implementation, this would be more sophisticated
        let mut missing = 0.0;
        
        // Check if we have the "complete" state
        if !state.variables.contains_key("complete") {
            missing += 5.0;
        }
        
        // Add penalty for budget proximity
        if let Some(max_cost) = intent.budget.max_cost {
            let remaining = max_cost - state.accumulated_cost;
            if remaining < 0.0 {
                missing += 1000.0; // Heavy penalty for exceeding budget
            }
        }
        
        missing
    }

    /// Check if an action's preconditions are satisfied.
    fn preconditions_met(&self, action: &PlanningAction, state: &PlanningState) -> bool {
        action.preconditions.iter().all(|pre| {
            state.variables.contains_key(pre)
        })
    }

    /// Apply an action to a state, returning the new state.
    fn apply_action(&self, action: &PlanningAction, state: &PlanningState) -> PlanningState {
        let mut new_state = state.clone();
        
        for effect in &action.effects {
            new_state.variables.insert(effect.clone(), serde_json::Value::Bool(true));
        }
        
        new_state.accumulated_cost += action.cost;
        new_state.accumulated_time_ms += action.duration_ms;
        
        new_state
    }

    /// Check if the goal is satisfied.
    fn is_goal_reached(&self, state: &PlanningState, _intent: &Intent) -> bool {
        state.variables.contains_key("complete")
    }

    /// Check if constraints are violated.
    fn constraints_violated(&self, state: &PlanningState, intent: &Intent) -> bool {
        // Check budget constraint
        if let Some(max_cost) = intent.budget.max_cost {
            if state.accumulated_cost > max_cost {
                return true;
            }
        }

        // Check time constraint
        if let Some(max_time) = intent.budget.max_duration_ms {
            if state.accumulated_time_ms > max_time {
                return true;
            }
        }

        false
    }

    /// Convert search steps to plan steps.
    fn steps_to_plan(&self, steps: Vec<Step>, intent: &Intent) -> Plan {
        let mut plan = Plan::new(intent.id, PlanningStrategy::Heuristic);
        
        let total_cost: f64 = steps.iter().map(|s| s.estimated_cost).sum();
        let total_time: u64 = steps.iter().map(|s| s.estimated_duration_ms).sum();
        
        plan.estimated_cost = total_cost;
        plan.estimated_latency_ms = total_time;
        plan.confidence_score = 0.85; // A* typically produces high-confidence plans
        
        for step in steps {
            plan.steps.push(step);
        }
        
        plan
    }
}

impl Default for AStarPlanner {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Planner for AStarPlanner {
    async fn plan(&self, intent: &Intent, initial_state: &PlanningState) -> Result<Plan> {
        let start_time = Instant::now();
        
        info!("Starting A* planning for intent {}", intent.id);
        
        // Initialize open and closed sets
        let mut open_set: BinaryHeap<SearchNode> = BinaryHeap::new();
        let mut closed_set: HashSet<Uuid> = HashSet::new();
        let mut states_explored = 0;
        
        // Create initial node
        let h_cost = self.heuristic(initial_state, intent);
        let initial_node = SearchNode {
            state: initial_state.clone(),
            steps: Vec::new(),
            g_cost: 0.0,
            h_cost,
            f_cost: h_cost,
            id: Uuid::new_v4(),
        };
        
        open_set.push(initial_node);
        
        while let Some(current) = open_set.pop() {
            states_explored += 1;
            
            // Check resource limits
            if states_explored > self.config.max_states_explored {
                warn!("A* exceeded max states explored limit");
                return Err(OrpheonError::PlanningFailed {
                    intent_id: intent.id,
                    message: format!("Exceeded maximum states explored: {}", self.config.max_states_explored),
                });
            }
            
            let elapsed_ms = start_time.elapsed().as_millis() as u64;
            if elapsed_ms > self.config.max_planning_time_ms {
                warn!("A* exceeded max planning time");
                return Err(OrpheonError::PlanningFailed {
                    intent_id: intent.id,
                    message: format!("Exceeded maximum planning time: {}ms", self.config.max_planning_time_ms),
                });
            }
            
            // Check if goal reached
            if self.is_goal_reached(&current.state, intent) {
                info!(
                    "A* found plan with {} steps, explored {} states in {}ms",
                    current.steps.len(),
                    states_explored,
                    elapsed_ms
                );
                return Ok(self.steps_to_plan(current.steps, intent));
            }
            
            // Skip if already visited
            if closed_set.contains(&current.id) {
                continue;
            }
            closed_set.insert(current.id);
            
            // Expand neighbors (try each applicable action)
            for action in &self.actions {
                if !self.preconditions_met(action, &current.state) {
                    continue;
                }
                
                let new_state = self.apply_action(action, &current.state);
                
                // Skip if constraints violated
                if self.constraints_violated(&new_state, intent) {
                    debug!("Skipping action {} due to constraint violation", action.name);
                    continue;
                }
                
                // Create new step
                let mut new_steps = current.steps.clone();
                let step = Step::new(&action.name, &action.name)
                    .with_cost(action.cost)
                    .with_duration(action.duration_ms);
                
                // Add dependencies to previous step if any
                let step = if let Some(last) = new_steps.last() {
                    step.depends_on(last.id)
                } else {
                    step
                };
                
                new_steps.push(step);
                
                // Calculate costs
                let g_cost = current.g_cost + action.cost;
                let h_cost = self.heuristic(&new_state, intent);
                let f_cost = g_cost + h_cost;
                
                let new_node = SearchNode {
                    state: new_state,
                    steps: new_steps,
                    g_cost,
                    h_cost,
                    f_cost,
                    id: Uuid::new_v4(),
                };
                
                open_set.push(new_node);
            }
        }
        
        // No plan found
        Err(OrpheonError::PlanningFailed {
            intent_id: intent.id,
            message: "No valid plan found after exhaustive search".to_string(),
        })
    }

    async fn validate_plan(&self, plan: &Plan, current_state: &PlanningState) -> Result<bool> {
        // Simulate execution of the plan
        let mut state = current_state.clone();
        
        for step in &plan.steps {
            // Find the action for this step
            let action = self.actions.iter().find(|a| a.name == step.action);
            
            match action {
                Some(action) => {
                    if !self.preconditions_met(action, &state) {
                        return Ok(false);
                    }
                    state = self.apply_action(action, &state);
                }
                None => {
                    // Unknown action, assume it's valid
                    debug!("Unknown action {} in plan validation", step.action);
                }
            }
        }
        
        Ok(true)
    }

    fn config(&self) -> &PlannerConfig {
        &self.config
    }

    fn set_config(&mut self, config: PlannerConfig) {
        self.config = config;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use orpheon_core::Intent;

    #[tokio::test]
    async fn test_astar_planning() {
        let planner = AStarPlanner::new();
        let intent = Intent::builder()
            .kind("provision_compute")
            .build()
            .unwrap();
        
        let initial_state = PlanningState::default();
        let result = planner.plan(&intent, &initial_state).await;
        
        assert!(result.is_ok());
        let plan = result.unwrap();
        assert!(!plan.steps.is_empty());
    }

    #[tokio::test]
    async fn test_plan_validation() {
        let planner = AStarPlanner::new();
        let intent = Intent::builder()
            .kind("test")
            .build()
            .unwrap();
        
        let initial_state = PlanningState::default();
        let plan = planner.plan(&intent, &initial_state).await.unwrap();
        
        let valid = planner.validate_plan(&plan, &initial_state).await.unwrap();
        assert!(valid);
    }
}
