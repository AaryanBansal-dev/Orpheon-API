//! Intent API endpoints.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use orpheon_core::{Budget, Constraint, Intent, IntentBuilder, IntentStatus, Preference, TimeWindow};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::state::AppState;

/// Request to submit a new intent.
#[derive(Debug, Deserialize)]
pub struct SubmitIntentRequest {
    /// The kind of intent.
    pub kind: String,
    
    /// Constraints for the intent.
    #[serde(default)]
    pub constraints: Vec<ConstraintInput>,
    
    /// Preferences for the intent.
    #[serde(default)]
    pub preferences: Vec<PreferenceInput>,
    
    /// Budget configuration.
    pub budget: Option<BudgetInput>,
    
    /// Metadata.
    #[serde(default)]
    pub metadata: serde_json::Value,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ConstraintInput {
    StateMatch { expression: String },
    ResourceLimit { resource: String, limit: f64 },
    Sla { metric: String, threshold: u64, unit: String },
}

#[derive(Debug, Deserialize)]
pub struct PreferenceInput {
    pub objective: String,
    pub direction: String,
    pub weight: f32,
}

#[derive(Debug, Deserialize)]
pub struct BudgetInput {
    pub max_cost: Option<f64>,
    pub currency: Option<String>,
    pub max_duration_ms: Option<u64>,
    pub max_retries: Option<u32>,
}

/// Response after submitting an intent.
#[derive(Debug, Serialize)]
pub struct SubmitIntentResponse {
    pub id: Uuid,
    pub status: String,
    pub message: String,
}

/// Response with intent details.
#[derive(Debug, Serialize)]
pub struct IntentResponse {
    pub id: Uuid,
    pub kind: String,
    pub status: String,
    pub plan_id: Option<Uuid>,
    pub artifact_id: Option<Uuid>,
    pub error: Option<String>,
    pub created_at: String,
}

/// Submit a new intent.
pub async fn submit_intent(
    State(state): State<AppState>,
    Json(req): Json<SubmitIntentRequest>,
) -> Result<(StatusCode, Json<SubmitIntentResponse>), (StatusCode, String)> {
    // Build the intent
    let mut builder = Intent::builder().kind(&req.kind);
    
    // Add constraints
    for c in req.constraints {
        let constraint = match c {
            ConstraintInput::StateMatch { expression } => {
                Constraint::StateMatch { expression }
            }
            ConstraintInput::ResourceLimit { resource, limit } => {
                Constraint::ResourceLimit { resource, limit }
            }
            ConstraintInput::Sla { metric, threshold, unit } => {
                Constraint::Sla { metric, threshold, unit }
            }
        };
        builder = builder.constraint(constraint);
    }
    
    // Add preferences
    for p in req.preferences {
        let direction = if p.direction == "minimize" {
            orpheon_core::intent::OptimizationDirection::Minimize
        } else {
            orpheon_core::intent::OptimizationDirection::Maximize
        };
        
        builder = builder.preference(Preference {
            objective: p.objective,
            direction,
            weight: p.weight,
        });
    }
    
    // Add budget
    if let Some(b) = req.budget {
        let budget = Budget {
            max_cost: b.max_cost,
            currency: b.currency.unwrap_or_else(|| "USD".to_string()),
            max_duration_ms: b.max_duration_ms,
            max_retries: b.max_retries.unwrap_or(3),
        };
        builder = builder.budget(budget);
    }
    
    // Add metadata
    if !req.metadata.is_null() {
        builder = builder.metadata(req.metadata);
    }
    
    // Build the intent
    let intent = builder.build().map_err(|e| {
        (StatusCode::BAD_REQUEST, e.to_string())
    })?;
    
    let intent_id = intent.id;
    
    // Store the intent
    state.store_intent(intent).await;
    
    Ok((
        StatusCode::CREATED,
        Json(SubmitIntentResponse {
            id: intent_id,
            status: "received".to_string(),
            message: "Intent submitted successfully".to_string(),
        }),
    ))
}

/// Get an intent by ID.
pub async fn get_intent(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<IntentResponse>, (StatusCode, String)> {
    let record = state.get_intent(id).await.ok_or_else(|| {
        (StatusCode::NOT_FOUND, format!("Intent {} not found", id))
    })?;
    
    Ok(Json(IntentResponse {
        id: record.intent.id,
        kind: record.intent.kind,
        status: format!("{:?}", record.status).to_lowercase(),
        plan_id: record.plan_id,
        artifact_id: record.artifact_id,
        error: record.error,
        created_at: record.intent.created_at.to_rfc3339(),
    }))
}

/// Cancel an intent.
pub async fn cancel_intent(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    let record = state.get_intent(id).await.ok_or_else(|| {
        (StatusCode::NOT_FOUND, format!("Intent {} not found", id))
    })?;
    
    // Check if cancellable
    if record.status.is_terminal() {
        return Err((
            StatusCode::CONFLICT,
            format!("Intent {} is already in terminal state", id),
        ));
    }
    
    state.update_intent_status(id, IntentStatus::Cancelled).await;
    
    Ok(StatusCode::NO_CONTENT)
}

/// Get the plan for an intent.
pub async fn get_plan(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<orpheon_core::Plan>, (StatusCode, String)> {
    let plan = state.get_plan_for_intent(id).await.ok_or_else(|| {
        (StatusCode::NOT_FOUND, format!("Plan for intent {} not found", id))
    })?;
    
    Ok(Json(plan))
}

/// Get the artifact for an intent.
pub async fn get_artifact(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<orpheon_core::ExecutionArtifact>, (StatusCode, String)> {
    let artifact = state.get_artifact_for_intent(id).await.ok_or_else(|| {
        (StatusCode::NOT_FOUND, format!("Artifact for intent {} not found", id))
    })?;
    
    Ok(Json(artifact))
}

/// List all intents.
pub async fn list_intents(
    State(state): State<AppState>,
) -> Json<Vec<IntentResponse>> {
    let records = state.list_intents().await;
    
    let responses: Vec<IntentResponse> = records
        .into_iter()
        .map(|r| IntentResponse {
            id: r.intent.id,
            kind: r.intent.kind,
            status: format!("{:?}", r.status).to_lowercase(),
            plan_id: r.plan_id,
            artifact_id: r.artifact_id,
            error: r.error,
            created_at: r.intent.created_at.to_rfc3339(),
        })
        .collect();
    
    Json(responses)
}
