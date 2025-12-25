//! Simulation endpoint.

use axum::{extract::State, http::StatusCode, Json};
use orpheon_core::{Budget, Intent, Plan};
use orpheon_planner::planner::PlanningState;
use orpheon_planner::Planner;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::state::AppState;

/// Request for simulation.
#[derive(Debug, Deserialize)]
pub struct SimulateRequest {
    /// The kind of intent to simulate.
    pub kind: String,
    
    /// Constraints for the simulation.
    #[serde(default)]
    pub constraints: Vec<serde_json::Value>,
    
    /// Preferences for the simulation.
    #[serde(default)]
    pub preferences: Vec<serde_json::Value>,
    
    /// Budget configuration.
    pub budget: Option<BudgetInput>,
    
    /// Target simulation time (ISO 8601).
    pub simulate_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct BudgetInput {
    pub max_cost: Option<f64>,
    pub max_duration_ms: Option<u64>,
}

/// Response from simulation.
#[derive(Debug, Serialize)]
pub struct SimulateResponse {
    pub simulation_id: Uuid,
    pub success: bool,
    pub plan: Option<PlanSummary>,
    pub estimated_cost: f64,
    pub estimated_duration_ms: u64,
    pub confidence_score: f32,
    pub warnings: Vec<String>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PlanSummary {
    pub id: Uuid,
    pub steps: usize,
    pub strategy: String,
}

/// Simulate an intent without executing.
pub async fn simulate_intent(
    State(state): State<AppState>,
    Json(req): Json<SimulateRequest>,
) -> Result<Json<SimulateResponse>, (StatusCode, String)> {
    // Build a temporary intent for simulation
    let intent = Intent::builder()
        .kind(&req.kind)
        .budget(Budget {
            max_cost: req.budget.as_ref().and_then(|b| b.max_cost),
            currency: "USD".to_string(),
            max_duration_ms: req.budget.as_ref().and_then(|b| b.max_duration_ms),
            max_retries: 3,
        })
        .build()
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    // Run the planner
    let initial_state = PlanningState::default();
    let plan_result = state.planner.plan(&intent, &initial_state).await;

    match plan_result {
        Ok(plan) => {
            let mut warnings = Vec::new();
            
            // Check budget
            if let Some(ref budget) = req.budget {
                if let Some(max) = budget.max_cost {
                    if plan.estimated_cost > max {
                        warnings.push(format!(
                            "Estimated cost ${:.2} exceeds budget ${:.2}",
                            plan.estimated_cost, max
                        ));
                    }
                }
                if let Some(max) = budget.max_duration_ms {
                    if plan.estimated_latency_ms > max {
                        warnings.push(format!(
                            "Estimated duration {}ms exceeds limit {}ms",
                            plan.estimated_latency_ms, max
                        ));
                    }
                }
            }

            Ok(Json(SimulateResponse {
                simulation_id: Uuid::new_v4(),
                success: true,
                plan: Some(PlanSummary {
                    id: plan.id,
                    steps: plan.steps.len(),
                    strategy: format!("{:?}", plan.strategy).to_lowercase(),
                }),
                estimated_cost: plan.estimated_cost,
                estimated_duration_ms: plan.estimated_latency_ms,
                confidence_score: plan.confidence_score,
                warnings,
                error: None,
            }))
        }
        Err(e) => {
            Ok(Json(SimulateResponse {
                simulation_id: Uuid::new_v4(),
                success: false,
                plan: None,
                estimated_cost: 0.0,
                estimated_duration_ms: 0,
                confidence_score: 0.0,
                warnings: Vec::new(),
                error: Some(e.to_string()),
            }))
        }
    }
}
